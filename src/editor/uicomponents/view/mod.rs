use super::super::{
    command::{Edit, Move},
    DocumentStatus, Line, Terminal,
};
use super::UIComponent;
use crate::editor::RowIdx;
use crate::prelude::*;
use std::time::Instant;
use std::{cmp::min, io::Error};
mod buffer;
use buffer::Buffer;
mod searchdirection;
use searchdirection::SearchDirection;

mod highlighter;
use highlighter::Highlighter;
mod fileinfo;
use fileinfo::FileInfo;
mod searchinfo;
use searchinfo::SearchInfo;

enum EditOperation {
    // NOTE: text is char (single Unicode scalar value) not a grapheme cluster.
    // This is safe for keyboard input since the OS composes grapheme clusters
    // before delivery. If paste support is added, this must change to String.
    InsertChar {
        at: Location,
        text: char,
    },
    DeleteChar {
        at: Location,
        text: char,
    },
    InsertNewLine {
        at: Location,
        grapheme_count_at_split: usize,
    },
    DeleteNewLine {
        line_idx: usize,
        split_at_grapheme: usize,
    },
    InsertGroup {
        start: Location,
        chars: Vec<char>,
    },
}
#[derive(Default)]
pub struct View {
    buffer: Buffer,
    needs_redraw: bool,
    // always starting at (0,0)and the size will dietermine the visible area
    size: Size,
    text_location: Location,
    scroll_offset: Position,
    search_info: Option<SearchInfo>,
    undo_stack: Vec<EditOperation>,
    redo_stack: Vec<EditOperation>,
    // timestamp of the last insert (helps us in grouping the steps)
    last_insert_time: Option<Instant>,
    // tracks where the cursor was after the last insert
    // used alongside last_insert_time to detect non-contiguous typing
    last_insert_location: Option<Location>,
}
/// How long a gap between keystrokes before we start a new undo group.
const GROUP_TIMEOUT_MS: u128 = 800;
impl View {
    pub fn get_status(&self) -> DocumentStatus {
        let file_info = self.buffer.get_file_info();
        DocumentStatus {
            file_name: format!("{file_info}"),
            total_lines: self.buffer.height(),
            current_line_idx: self.text_location.line_idx,
            is_modified: self.buffer.is_dirty(),
            file_type: file_info.get_file_type(),
        }
    }
    pub const fn is_file_loaded(&self) -> bool {
        self.buffer.is_file_loaded()
    }
    pub fn handle_edit_command(&mut self, command: Edit) {
        match command {
            Edit::Insert(character) => self.insert_char(character),
            Edit::Delete => self.delete(),
            Edit::DeleteBackward => self.delete_backward(),
            Edit::InsertNewLine => self.insert_newline(),
        }
    }
    pub fn handle_move_command(&mut self, command: Move) {
        // This match moves the positon, but does not check for all boundaries.
        // The final boundarline checking happens after the match statement.
        let Size { height, .. } = self.size;
        match command {
            Move::Up => self.move_up(1),
            Move::Down => self.move_down(1),
            Move::Left => self.move_left(),
            Move::Right => self.move_right(),
            Move::PageUp => self.move_up(height.saturating_sub(1)),
            Move::PageDown => self.move_down(height.saturating_sub(1)),
            Move::StartOfLine => self.move_to_start_of_line(),
            Move::EndOfLine => self.move_to_end_of_line(),
        }
        self.scroll_text_location_into_view();
    }
    pub fn load(&mut self, file_name: &str) -> Result<(), Error> {
        let buffer = Buffer::load(file_name)?;
        self.buffer = buffer;
        self.mark_redraw(true);
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), Error> {
        self.buffer.save()?;
        self.mark_redraw(true);
        Ok(())
    }
    pub fn save_as(&mut self, file_name: &str) -> Result<(), Error> {
        self.buffer.save_as(file_name)?;
        self.mark_redraw(true);
        Ok(())
    }
    fn insert_char(&mut self, character: char) {
        let old_len = self.buffer.grapheme_count(self.text_location.line_idx);
        self.buffer.insert_char(character, self.text_location);
        self.redo_stack.clear();

        let now = Instant::now();
        let should_group = self
            .last_insert_time
            .map(|t| now.duration_since(t).as_millis() < GROUP_TIMEOUT_MS)
            .unwrap_or(false)
            && self
                .last_insert_location
                .map(|loc| {
                    loc.line_idx == self.text_location.line_idx
                        && loc.grapheme_idx == self.text_location.grapheme_idx
                })
                .unwrap_or(false);

        if should_group {
            // Try to merge into the last op on the undo stack
            if let Some(EditOperation::InsertGroup { chars, .. }) = self.undo_stack.last_mut() {
                chars.push(character);
                self.last_insert_time = Some(now);
                // Still need to move cursor
                let new_len = self.buffer.grapheme_count(self.text_location.line_idx);
                if new_len.saturating_sub(old_len) > 0 {
                    self.handle_move_command(Move::Right);
                }
                self.mark_redraw(true);
                return;
            }
            // Last op was InsertChar (single) — upgrade it to a group
            if let Some(EditOperation::InsertChar { at, text }) = self.undo_stack.pop() {
                self.undo_stack.push(EditOperation::InsertGroup {
                    start: at,
                    chars: vec![text, character],
                });
                self.last_insert_time = Some(now);
                let new_len = self.buffer.grapheme_count(self.text_location.line_idx);
                if new_len.saturating_sub(old_len) > 0 {
                    self.handle_move_command(Move::Right);
                }
                self.mark_redraw(true);
                return;
            }
        }

        // No grouping: push a fresh InsertChar
        self.undo_stack.push(EditOperation::InsertChar {
            at: self.text_location,
            text: character,
        });
        self.last_insert_time = Some(now);

        let new_len = self.buffer.grapheme_count(self.text_location.line_idx);
        if new_len.saturating_sub(old_len) > 0 {
            self.handle_move_command(Move::Right);
        }
        self.mark_redraw(true);
    }
    fn insert_newline(&mut self) {
        self.last_insert_time = None;
        self.last_insert_location = None;
        let grapheme_count_at_split = self.buffer.grapheme_count(self.text_location.line_idx);
        self.undo_stack.push(EditOperation::InsertNewLine {
            at: self.text_location,
            grapheme_count_at_split,
        });
        self.redo_stack.clear();
        self.buffer.insert_newline(self.text_location);
        self.handle_move_command(Move::Right);
        self.mark_redraw(true);
    }
    fn delete_backward(&mut self) {
        self.last_insert_time = None;
        self.last_insert_location = None;
        // Recording before moving, so we know the true deletion location
        if self.text_location.line_idx == 0 && self.text_location.grapheme_idx == 0 {
            return; // Nothing to do at start of document
        }

        if self.text_location.grapheme_idx == 0 {
            // Backspace at line start = merge this line onto previous
            // The "newline" being deleted is at the end of line_idx - 1
            let prev_line_idx = self.text_location.line_idx.saturating_sub(1);
            let split_at = self.buffer.grapheme_count(prev_line_idx);
            self.undo_stack.push(EditOperation::DeleteNewLine {
                line_idx: prev_line_idx,
                split_at_grapheme: split_at,
            });
            self.redo_stack.clear();
            // Move left (to end of previous line), then delete the newline via buffer.delete
            self.handle_move_command(Move::Left);
            self.buffer.delete(self.text_location); // deletes end-of-line = merges lines
        } else {
            self.handle_move_command(Move::Left);
            // Now at is the grapheme we want to delete
            if let Some(ch) = self.buffer.get_char_at(self.text_location) {
                self.undo_stack.push(EditOperation::DeleteChar {
                    at: self.text_location,
                    text: ch,
                });
                self.redo_stack.clear();
                self.buffer.delete(self.text_location);
            }
        }
        self.mark_redraw(true);
    }
    fn delete(&mut self) {
        self.last_insert_time = None;
        self.last_insert_location = None;
        let at = self.text_location;
        let grapheme_count = self.buffer.grapheme_count(at.line_idx);

        if at.grapheme_idx >= grapheme_count {
            // Delete at end-of-line = merge next line onto this one
            if at.line_idx.saturating_add(1) < self.buffer.height() {
                self.undo_stack.push(EditOperation::DeleteNewLine {
                    line_idx: at.line_idx,
                    split_at_grapheme: grapheme_count,
                });
                self.redo_stack.clear();
                self.buffer.delete(at);
            }
            // else: at end of last line, nothing to do
        } else if let Some(ch) = self.buffer.get_char_at(at) {
            self.undo_stack
                .push(EditOperation::DeleteChar { at, text: ch });
            self.redo_stack.clear();
            self.buffer.delete(at);
        }
        self.mark_redraw(true);
    }

    fn render_line(at: RowIdx, line_text: &str) -> Result<(), Error> {
        Terminal::print_row(at, line_text)
    }
    fn build_welcome_message(width: usize) -> String {
        if width == 0 {
            return String::new();
        }
        let welcome_message = format!("{NAME} editor -- version {VERSION}");
        let len = welcome_message.len();
        let remaining_width = width.saturating_sub(1);
        if remaining_width < len {
            return "~".to_string();
        }
        format!("{:<1}{:^remaining_width$}", "~", welcome_message)
    }
    // hmm not so very simple undo redo anymore ig
    pub fn undo(&mut self) {
        if let Some(op) = self.undo_stack.pop() {
            self.apply_undo(&op);
            self.redo_stack.push(op);
            self.scroll_text_location_into_view();
            self.mark_redraw(true);
        }
    }

    pub fn redo(&mut self) {
        if let Some(op) = self.redo_stack.pop() {
            self.apply_redo(&op);
            self.undo_stack.push(op);
            self.scroll_text_location_into_view();
            self.mark_redraw(true);
        }
    }
    fn apply_undo(&mut self, op: &EditOperation) {
        match op {
            EditOperation::InsertChar { at, .. } => {
                // Undo an insert = delete the character that was inserted
                self.buffer.delete(*at);
                self.text_location = *at;
            }
            EditOperation::DeleteChar { at, text } => {
                // Undo a delete = re-insert the character
                self.buffer.insert_char(*text, *at);
                self.text_location = *at;
            }
            EditOperation::InsertNewLine {
                at,
                grapheme_count_at_split,
            } => {
                // Undo an Enter = merge the line that was split back together.
                // After insert_newline at `at`, the cursor moved to {line_idx+1, grapheme_idx:0}.
                // The split point is at `at.grapheme_idx` on line `at.line_idx`.
                // To undo: delete the newline = buffer.delete at end of at.line_idx
                self.buffer.delete(Location {
                    line_idx: at.line_idx,
                    grapheme_idx: *grapheme_count_at_split,
                });
                self.text_location = *at;
            }
            EditOperation::DeleteNewLine {
                line_idx,
                split_at_grapheme,
            } => {
                // Undo a newline deletion = re-split the merged line
                self.buffer.insert_newline(Location {
                    line_idx: *line_idx,
                    grapheme_idx: *split_at_grapheme,
                });
                self.text_location = Location {
                    line_idx: line_idx.saturating_add(1),
                    grapheme_idx: 0,
                };
            }
            EditOperation::InsertGroup { start, chars } => {
                // Delete all chars in the group, working backwards from the last
                // inserted position so byte indices stay valid.
                // The chars were inserted left-to-right starting at `start`.
                // After inserting N chars, the last char is at start.grapheme_idx + N - 1.
                let end_grapheme = start.grapheme_idx.saturating_add(chars.len());
                for grapheme_idx in (start.grapheme_idx..end_grapheme).rev() {
                    self.buffer.delete(Location {
                        line_idx: start.line_idx,
                        grapheme_idx,
                    });
                }
                self.text_location = *start;
            }
        }
    }
    fn apply_redo(&mut self, op: &EditOperation) {
        match op {
            EditOperation::InsertChar { at, text } => {
                self.buffer.insert_char(*text, *at);
                self.text_location = Location {
                    line_idx: at.line_idx,
                    grapheme_idx: at.grapheme_idx.saturating_add(1),
                };
            }
            EditOperation::DeleteChar { at, .. } => {
                self.buffer.delete(*at);
                self.text_location = *at;
            }
            EditOperation::InsertNewLine { at, .. } => {
                self.buffer.insert_newline(*at);
                self.text_location = Location {
                    line_idx: at.line_idx.saturating_add(1),
                    grapheme_idx: 0,
                };
            }
            EditOperation::DeleteNewLine {
                line_idx,
                split_at_grapheme,
            } => {
                // Redo the merge: delete the newline at end of line_idx
                self.buffer.delete(Location {
                    line_idx: *line_idx,
                    grapheme_idx: *split_at_grapheme,
                });
                self.text_location = Location {
                    line_idx: *line_idx,
                    grapheme_idx: *split_at_grapheme,
                };
            }
            EditOperation::InsertGroup { start, chars } => {
                // Re-insert chars left-to-right
                let mut current = *start;
                for &ch in chars {
                    self.buffer.insert_char(ch, current);
                    current.grapheme_idx = current.grapheme_idx.saturating_add(1);
                }
                self.text_location = current;
            }
        }
    }
    // SCROLLING
    fn scroll_vertically(&mut self, to: RowIdx) {
        let Size { height, .. } = self.size;
        let offset_changed = if to < self.scroll_offset.row {
            self.scroll_offset.row = to;
            true
        } else if to >= self.scroll_offset.row.saturating_add(height) {
            self.scroll_offset.row = to.saturating_sub(height).saturating_add(1);
            true
        } else {
            false
        };
        if offset_changed {
            self.mark_redraw(true);
        }
    }
    fn scroll_horizontally(&mut self, to: ColIdx) {
        let Size { width, .. } = self.size;
        let offset_changed = if to < self.scroll_offset.col {
            self.scroll_offset.col = to;
            true
        } else if to >= self.scroll_offset.col.saturating_add(width) {
            self.scroll_offset.col = to.saturating_sub(width).saturating_add(1);
            true
        } else {
            false
        };
        if offset_changed {
            self.mark_redraw(true);
        }
    }
    fn center_text_location(&mut self) {
        let Size { height, width } = self.size;
        let Position { row, col } = self.text_location_to_position();
        let vertical_mid = height.div_ceil(2);
        let horizontal_mid = width.div_ceil(2);
        self.scroll_offset.row = row.saturating_sub(vertical_mid);
        self.scroll_offset.col = col.saturating_sub(horizontal_mid);
        self.mark_redraw(true);
    }
    fn scroll_text_location_into_view(&mut self) {
        let Position { row, col } = self.text_location_to_position();
        self.scroll_vertically(row);
        self.scroll_horizontally(col);
    }
    pub fn caret_position(&self) -> Position {
        self.text_location_to_position()
            .saturating_sub(self.scroll_offset)
    }
    fn text_location_to_position(&self) -> Position {
        let row = self.text_location.line_idx;
        debug_assert!(row.saturating_sub(1) <= self.buffer.height());
        let col = self
            .buffer
            .width_until(row, self.text_location.grapheme_idx);
        Position { col, row }
    }

    fn move_up(&mut self, step: usize) {
        self.text_location.line_idx = self.text_location.line_idx.saturating_sub(step);
        self.snap_to_valid_grapheme();
    }
    fn move_down(&mut self, step: usize) {
        self.text_location.line_idx = self.text_location.line_idx.saturating_add(step);
        self.snap_to_valid_grapheme();
        self.snap_to_valid_line();
    }
    // clippy::arithmetic_side_effects: This function performs arithmetic calculations
    // after explicitly checking that the target value will be within bounds.
    #[allow(clippy::arithmetic_side_effects)]
    fn move_right(&mut self) {
        let grapheme_count = self.buffer.grapheme_count(self.text_location.line_idx);
        if self.text_location.grapheme_idx < grapheme_count {
            self.text_location.grapheme_idx += 1;
        } else {
            self.move_to_start_of_line();
            self.move_down(1);
        }
    }
    #[allow(clippy::arithmetic_side_effects)]
    fn move_left(&mut self) {
        if self.text_location.grapheme_idx > 0 {
            self.text_location.grapheme_idx -= 1;
        } else if self.text_location.line_idx > 0 {
            self.move_up(1);
            self.move_to_end_of_line();
        }
    }

    fn move_to_start_of_line(&mut self) {
        self.text_location.grapheme_idx = 0;
    }
    fn move_to_end_of_line(&mut self) {
        self.text_location.grapheme_idx = self.buffer.grapheme_count(self.text_location.line_idx);
    }

    // Ensures self.location.grapheme_idx points to a valid grapheme index by snapping it to the left most grapheme if appropriate.
    // Doesn't trigger scrolling.
    fn snap_to_valid_grapheme(&mut self) {
        self.text_location.grapheme_idx = min(
            self.text_location.grapheme_idx,
            self.buffer.grapheme_count(self.text_location.line_idx),
        )
    }
    // Ensures self.location.line_idx points to a valid line index by snapping it to the bottom most line if appropriate.
    // Doesn't trigger scrolling.
    fn snap_to_valid_line(&mut self) {
        self.text_location.line_idx = min(self.text_location.line_idx, self.buffer.height());
    }
    // region : Search
    pub fn enter_search(&mut self) {
        self.search_info = Some(SearchInfo {
            prev_location: self.text_location,
            prev_scroll_offset: self.scroll_offset,
            query: None,
        });
    }
    pub fn exit_search(&mut self) {
        self.search_info = None;
        self.mark_redraw(true);
    }
    pub fn dismiss_search(&mut self) {
        if let Some(search_info) = &self.search_info {
            self.text_location = search_info.prev_location;
            self.scroll_offset = search_info.prev_scroll_offset;
            self.scroll_text_location_into_view();
        }
        self.exit_search();
    }
    pub fn search(&mut self, query: &str) {
        if let Some(search_info) = &mut self.search_info {
            search_info.query = Some(Line::from(query));
        }
        self.search_in_direction(self.text_location, SearchDirection::default());
    }
    fn get_search_query(&self) -> Option<&Line> {
        let query = self
            .search_info
            .as_ref()
            .and_then(|search_info| search_info.query.as_ref());

        debug_assert!(
            query.is_some(),
            "Attempting to search with malformed searchinfo present"
        );
        query
    }
    fn search_in_direction(&mut self, from: Location, direction: SearchDirection) {
        if let Some(location) = self.get_search_query().and_then(|query| {
            if query.is_empty() {
                None
            } else if direction == SearchDirection::Forward {
                self.buffer.search_forward(query, from)
            } else {
                self.buffer.search_backward(query, from)
            }
        }) {
            self.text_location = location;
            self.center_text_location();
        };
        self.mark_redraw(true);
    }
    pub fn search_next(&mut self) {
        let step_right = self
            .get_search_query()
            .map_or(1, |query| min(query.grapheme_count(), 1));
        let location = Location {
            line_idx: self.text_location.line_idx,
            grapheme_idx: self.text_location.grapheme_idx.saturating_add(step_right),
        };
        self.search_in_direction(location, SearchDirection::Forward);
    }
    pub fn search_prev(&mut self) {
        self.search_in_direction(self.text_location, SearchDirection::Backward);
    }
    // endregion
}

impl UIComponent for View {
    fn mark_redraw(&mut self, value: bool) {
        self.needs_redraw = value;
    }
    fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }
    fn set_size(&mut self, size: Size) {
        self.size = size;
        self.scroll_text_location_into_view();
    }
    fn draw(&mut self, origin_row: RowIdx) -> Result<(), Error> {
        let Size { height, width } = self.size;
        let end_y = origin_row.saturating_add(height);

        let top_third = height.div_ceil(3);
        let scroll_top = self.scroll_offset.row;
        let query = self
            .search_info
            .as_ref()
            .and_then(|search_info| search_info.query.as_deref());
        let selected_match = query.is_some().then_some(self.text_location);
        let mut highlighter = Highlighter::new(
            query,
            selected_match,
            self.buffer.get_file_info().get_file_type(),
        );
        for current_row in 0..end_y.saturating_add(scroll_top) {
            self.buffer.highlight(current_row, &mut highlighter); // the full document is highlighted
        }
        for current_row in origin_row..end_y {
            // to get the correct line index, we have to take current_row (the absolute row on screen),
            // subtract origin_row to get the current row relative to the view (ranging from 0 to self.size.height)
            // and add the scroll offset.
            let line_idx = current_row
                .saturating_sub(origin_row)
                .saturating_add(scroll_top);
            let left = self.scroll_offset.col;
            let right = self.scroll_offset.col.saturating_add(width);
            if let Some(annotated_string) =
                self.buffer
                    .get_highlighted_substring(line_idx, left..right, &highlighter)
            {
                Terminal::print_annotated_row(current_row, &annotated_string)?;
            } else if current_row == top_third && self.buffer.is_empty() {
                Self::render_line(current_row, &Self::build_welcome_message(width))?;
            } else {
                Self::render_line(current_row, "~")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::command::Edit;

    // Helper: build a View with some text already loaded
    fn view_with_text(text: &str) -> View {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });
        for ch in text.chars() {
            if ch == '\n' {
                view.handle_edit_command(Edit::InsertNewLine);
            } else {
                view.handle_edit_command(Edit::Insert(ch));
            }
        }
        // Clear undo/redo stacks so tests start clean
        // (typing during setup shouldn't count as undoable history)
        view.undo_stack.clear();
        view.redo_stack.clear();
        // Reset cursor to top-left
        view.text_location = Location {
            line_idx: 0,
            grapheme_idx: 0,
        };
        view
    }

    // ── Basic insert/undo ────────────────────────────────────────────────────

    #[test]
    fn undo_single_insert() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        view.handle_edit_command(Edit::Insert('a'));
        assert_eq!(view.buffer.grapheme_count(0), 1);

        view.undo();
        assert_eq!(view.buffer.grapheme_count(0), 0);
    }

    #[test]
    fn redo_single_insert() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        view.handle_edit_command(Edit::Insert('a'));
        view.undo();
        assert_eq!(view.buffer.grapheme_count(0), 0);

        view.redo();
        assert_eq!(view.buffer.grapheme_count(0), 1);
    }

    #[test]
    fn undo_multiple_inserts() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        for ch in "hello".chars() {
            view.handle_edit_command(Edit::Insert(ch));
        }
        assert_eq!(view.buffer.grapheme_count(0), 5);

        for _ in 0..5 {
            view.undo();
        }
        assert_eq!(view.buffer.grapheme_count(0), 0);
    }

    #[test]
    fn undo_then_redo_restores_text() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        for ch in "hi".chars() {
            view.handle_edit_command(Edit::Insert(ch));
        }
        view.undo();
        view.undo();
        view.redo();
        view.redo();

        // Should be back to "hi"
        assert_eq!(view.buffer.grapheme_count(0), 2);
    }

    // ── Newline undo/redo ────────────────────────────────────────────────────

    #[test]
    fn undo_newline_merges_lines() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        for ch in "hello".chars() {
            view.handle_edit_command(Edit::Insert(ch));
        }
        view.handle_edit_command(Edit::InsertNewLine);
        assert_eq!(view.buffer.height(), 2);

        view.undo();
        assert_eq!(view.buffer.height(), 1);
        assert_eq!(view.buffer.grapheme_count(0), 5);
    }

    #[test]
    fn redo_newline_splits_lines() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        for ch in "hello".chars() {
            view.handle_edit_command(Edit::Insert(ch));
        }
        view.handle_edit_command(Edit::InsertNewLine);
        view.undo();
        assert_eq!(view.buffer.height(), 1);

        view.redo();
        assert_eq!(view.buffer.height(), 2);
    }

    #[test]
    fn newline_in_middle_of_line_undo() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        for ch in "hello".chars() {
            view.handle_edit_command(Edit::Insert(ch));
        }
        // Move cursor to middle
        view.text_location = Location {
            line_idx: 0,
            grapheme_idx: 2,
        };
        view.handle_edit_command(Edit::InsertNewLine);

        // Should be "he" on line 0, "llo" on line 1
        assert_eq!(view.buffer.height(), 2);
        assert_eq!(view.buffer.grapheme_count(0), 2);
        assert_eq!(view.buffer.grapheme_count(1), 3);

        view.undo();

        assert_eq!(view.buffer.height(), 1);
        assert_eq!(view.buffer.grapheme_count(0), 5);
    }

    // ── Delete undo/redo ─────────────────────────────────────────────────────

    #[test]
    fn undo_delete_restores_char() {
        let mut view = view_with_text("hi");
        view.handle_edit_command(Edit::Delete);
        assert_eq!(view.buffer.grapheme_count(0), 1);

        view.undo();
        assert_eq!(view.buffer.grapheme_count(0), 2);
    }

    #[test]
    fn undo_backspace_restores_char() {
        let mut view = view_with_text("hi");
        // Move cursor to end of line
        view.text_location = Location {
            line_idx: 0,
            grapheme_idx: 2,
        };
        view.handle_edit_command(Edit::DeleteBackward);
        assert_eq!(view.buffer.grapheme_count(0), 1);

        view.undo();
        assert_eq!(view.buffer.grapheme_count(0), 2);
    }

    #[test]
    fn undo_backspace_at_line_start_restores_newline() {
        let mut view = view_with_text("hello\nworld");
        // Position at start of line 1
        view.text_location = Location {
            line_idx: 1,
            grapheme_idx: 0,
        };
        assert_eq!(view.buffer.height(), 2);

        view.handle_edit_command(Edit::DeleteBackward);
        assert_eq!(view.buffer.height(), 1);

        view.undo();
        assert_eq!(view.buffer.height(), 2);
        assert_eq!(view.buffer.grapheme_count(0), 5);
        assert_eq!(view.buffer.grapheme_count(1), 5);
    }

    #[test]
    fn undo_delete_at_line_end_restores_newline() {
        let mut view = view_with_text("hello\nworld");
        // Position at end of line 0
        view.text_location = Location {
            line_idx: 0,
            grapheme_idx: 5,
        };
        assert_eq!(view.buffer.height(), 2);

        view.handle_edit_command(Edit::Delete);
        assert_eq!(view.buffer.height(), 1);

        view.undo();
        assert_eq!(view.buffer.height(), 2);
    }

    // ── Cursor position after undo/redo ──────────────────────────────────────

    #[test]
    fn cursor_position_restored_after_undo_insert() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        view.handle_edit_command(Edit::Insert('a'));
        // Cursor should be at grapheme 1 after insert
        assert_eq!(view.text_location.grapheme_idx, 1);

        view.undo();
        // Cursor should go back to where the insert happened
        assert_eq!(view.text_location.grapheme_idx, 0);
    }

    #[test]
    fn cursor_position_restored_after_redo_insert() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        view.handle_edit_command(Edit::Insert('a'));
        view.undo();
        view.redo();

        assert_eq!(view.text_location.grapheme_idx, 1);
    }

    #[test]
    fn cursor_on_correct_line_after_undo_newline() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        for ch in "hello".chars() {
            view.handle_edit_command(Edit::Insert(ch));
        }
        view.handle_edit_command(Edit::InsertNewLine);
        assert_eq!(view.text_location.line_idx, 1);

        view.undo();
        assert_eq!(view.text_location.line_idx, 0);
    }

    // ── Redo stack cleared on new edit ───────────────────────────────────────

    #[test]
    fn new_edit_clears_redo_stack() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        view.handle_edit_command(Edit::Insert('a'));
        view.undo();
        assert!(!view.redo_stack.is_empty());

        // New edit should wipe redo
        view.handle_edit_command(Edit::Insert('b'));
        assert!(view.redo_stack.is_empty());
    }

    // ── Edge cases ───────────────────────────────────────────────────────────

    #[test]
    fn undo_on_empty_stack_does_nothing() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        // Should not panic
        view.undo();
        assert_eq!(view.buffer.height(), 0);
    }

    #[test]
    fn redo_on_empty_stack_does_nothing() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        view.redo();
        assert_eq!(view.buffer.height(), 0);
    }

    #[test]
    fn backspace_at_document_start_does_nothing() {
        let mut view = view_with_text("hi");
        view.text_location = Location {
            line_idx: 0,
            grapheme_idx: 0,
        };

        view.handle_edit_command(Edit::DeleteBackward);

        // Nothing changed, nothing on undo stack
        assert_eq!(view.buffer.grapheme_count(0), 2);
        assert!(view.undo_stack.is_empty());
    }

    #[test]
    fn delete_at_document_end_does_nothing() {
        let mut view = view_with_text("hi");
        let last_line = view.buffer.height().saturating_sub(1);
        let end_grapheme = view.buffer.grapheme_count(last_line);
        view.text_location = Location {
            line_idx: last_line,
            grapheme_idx: end_grapheme,
        };

        view.handle_edit_command(Edit::Delete);

        assert_eq!(view.buffer.grapheme_count(0), 2);
        assert!(view.undo_stack.is_empty());
    }

    // ── Unicode / grapheme correctness ───────────────────────────────────────

    #[test]
    fn undo_insert_unicode_char() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        view.handle_edit_command(Edit::Insert('ü'));
        assert_eq!(view.buffer.grapheme_count(0), 1);

        view.undo();
        assert_eq!(view.buffer.grapheme_count(0), 0);
    }

    // ── Word-granularity grouping ─────────────────────────────────────────────

    #[test]
    fn rapid_inserts_form_a_group() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        // Simulate typing "hello" with no delay — all within GROUP_TIMEOUT_MS
        for ch in "hello".chars() {
            view.handle_edit_command(Edit::Insert(ch));
            // Force the timestamp to be recent so grouping triggers
            view.last_insert_time = Some(std::time::Instant::now());
            view.last_insert_location = Some(view.text_location);
        }

        // Should be ONE op on the undo stack (the group), not five
        assert_eq!(view.undo_stack.len(), 1);
        assert!(matches!(
            view.undo_stack.last(),
            Some(EditOperation::InsertGroup { .. })
        ));
    }

    #[test]
    fn undo_group_removes_all_chars_at_once() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        for ch in "hello".chars() {
            view.handle_edit_command(Edit::Insert(ch));
            view.last_insert_time = Some(std::time::Instant::now());
            view.last_insert_location = Some(view.text_location);
        }
        assert_eq!(view.buffer.grapheme_count(0), 5);

        view.undo();

        // All 5 chars gone in one undo
        assert_eq!(view.buffer.grapheme_count(0), 0);
        assert!(view.undo_stack.is_empty());
    }

    #[test]
    fn redo_group_reinserts_all_chars() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        for ch in "hello".chars() {
            view.handle_edit_command(Edit::Insert(ch));
            view.last_insert_time = Some(std::time::Instant::now());
            view.last_insert_location = Some(view.text_location);
        }
        view.undo();
        assert_eq!(view.buffer.grapheme_count(0), 0);

        view.redo();
        assert_eq!(view.buffer.grapheme_count(0), 5);
    }

    #[test]
    fn multiple_undos_and_redos_stay_consistent() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        // Type "abc" with forced grouping — they merge into ONE InsertGroup
        for ch in "abc".chars() {
            view.handle_edit_command(Edit::Insert(ch));
            view.last_insert_time = Some(std::time::Instant::now());
            view.last_insert_location = Some(view.text_location);
        }
        // One undo removes the whole group ("abc")
        assert_eq!(view.buffer.grapheme_count(0), 3);
        view.undo();
        assert_eq!(view.buffer.grapheme_count(0), 0);

        // One redo restores the whole group
        view.redo();
        assert_eq!(view.buffer.grapheme_count(0), 3);
    }

    #[test]
    fn delete_between_inserts_breaks_group() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        // Type "hel" — forms InsertGroup("hel"), cursor at grapheme 3
        for ch in "hel".chars() {
            view.handle_edit_command(Edit::Insert(ch));
            view.last_insert_time = Some(std::time::Instant::now());
            view.last_insert_location = Some(view.text_location);
        }

        // Backspace (DeleteBackward) at position 3 deletes 'l', breaks the group
        // We use DeleteBackward here because Delete at end-of-line-with-no-next-line is a no-op
        view.handle_edit_command(Edit::DeleteBackward);
        // "he" remains, stack: [InsertGroup("hel"), DeleteChar('l')]

        // Type "lo" — forms a new group, cursor advances
        for ch in "lo".chars() {
            view.handle_edit_command(Edit::Insert(ch));
            view.last_insert_time = Some(std::time::Instant::now());
            view.last_insert_location = Some(view.text_location);
        }
        // "helo" in buffer, stack: [InsertGroup("hel"), DeleteChar('l'), InsertGroup("lo")]

        assert_eq!(view.undo_stack.len(), 3);
    }

    #[test]
    fn cursor_at_group_start_after_undo() {
        let mut view = View::default();
        view.set_size(Size {
            height: 24,
            width: 80,
        });

        // Start cursor at grapheme 3 (simulate typing mid-document)
        view.text_location = Location {
            line_idx: 0,
            grapheme_idx: 0,
        };
        for ch in "abc".chars() {
            view.handle_edit_command(Edit::Insert(ch));
            view.last_insert_time = Some(std::time::Instant::now());
            view.last_insert_location = Some(view.text_location);
        }
        view.undo();

        assert_eq!(view.text_location.grapheme_idx, 0);
    }
}
