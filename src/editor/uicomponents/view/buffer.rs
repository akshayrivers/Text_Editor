use super::super::super::AnnotatedString;
use super::FileInfo;
use super::Highlighter;
use super::Line;
use crate::prelude::*;
use std::io::Write;
use std::ops::Range;
use std::{fs::read_to_string, fs::File, io::Error};
#[derive(Default)]
pub struct Buffer {
    lines: Vec<Line>,
    file_info: FileInfo,
    dirty: bool,
}

impl Buffer {
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub const fn get_file_info(&self) -> &FileInfo {
        &self.file_info
    }
    pub fn grapheme_count(&self, idx: LineIdx) -> GraphemeIdx {
        self.lines.get(idx).map_or(0, Line::grapheme_count)
    }
    pub fn width_until(&self, idx: LineIdx, until: GraphemeIdx) -> GraphemeIdx {
        self.lines
            .get(idx)
            .map_or(0, |line| line.width_until(until))
    }

    pub fn get_highlighted_substring(
        &self,
        line_idx: LineIdx,
        range: Range<GraphemeIdx>,
        highlighter: &Highlighter,
    ) -> Option<AnnotatedString> {
        self.lines.get(line_idx).map(|line| {
            line.get_annotated_visible_substr(range, Some(&highlighter.get_annotations(line_idx)))
        })
    }
    pub fn highlight(&self, idx: LineIdx, highlighter: &mut Highlighter) {
        if let Some(line) = self.lines.get(idx) {
            highlighter.highlight(idx, line);
        }
    }
    pub fn load(file_name: &str) -> Result<Self, Error> {
        let contents = read_to_string(file_name)?;
        let mut lines = Vec::new();
        for value in contents.lines() {
            lines.push(Line::from(value))
        }
        Ok(Self {
            lines,
            file_info: FileInfo::from(file_name),
            dirty: false,
        })
    }
    pub fn search_forward(&self, query: &str, from: Location) -> Option<Location> {
        //finally iterator magic
        if query.is_empty() {
            return None;
        }
        let mut is_first = true;
        for (line_idx, line) in self
            .lines
            .iter()
            .enumerate()
            .cycle()
            .skip(from.line_idx)
            .take(self.lines.len().saturating_add(1))
        {
            let from_grapheme_idx = if is_first {
                is_first = false;
                from.grapheme_idx
            } else {
                0
            };
            if let Some(grapheme_idx) = line.search_forward(query, from_grapheme_idx) {
                return Some(Location {
                    grapheme_idx,
                    line_idx,
                });
            }
        }
        None
    }
    pub fn get_char_at(&self, at: Location) -> Option<char> {
        Some(' ')
    }
    pub fn search_backward(&self, query: &str, from: Location) -> Option<Location> {
        if query.is_empty() {
            return None;
        }
        let mut is_first = true;
        for (line_idx, line) in self
            .lines
            .iter()
            .enumerate()
            .rev()
            .cycle()
            .skip(
                self.lines
                    .len()
                    .saturating_sub(from.line_idx)
                    .saturating_sub(1),
            )
            .take(self.lines.len().saturating_add(1))
        {
            let from_grapheme_idx = if is_first {
                is_first = false;
                from.grapheme_idx
            } else {
                line.grapheme_count()
            };
            if let Some(grapheme_idx) = line.search_backward(query, from_grapheme_idx) {
                return Some(Location {
                    grapheme_idx,
                    line_idx,
                });
            }
        }
        None
    }
    fn save_to_file(&self, file_info: &FileInfo) -> Result<(), Error> {
        if let Some(file_path) = &file_info.get_path() {
            let mut file = File::create(file_path)?;
            for line in &self.lines {
                writeln!(file, "{line}")?;
            }
        } else {
            #[cfg(debug_assertions)]
            {
                panic!("Attempting to save with no file path present");
            }
        }
        Ok(())
    }
    pub fn save_as(&mut self, file_name: &str) -> Result<(), Error> {
        let file_info = FileInfo::from(file_name);
        self.save_to_file(&file_info)?;
        self.file_info = file_info;
        self.dirty = false;
        Ok(())
    }
    pub fn save(&mut self) -> Result<(), Error> {
        self.save_to_file(&self.file_info)?;
        self.dirty = false;
        Ok(())
    }
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
    pub const fn is_file_loaded(&self) -> bool {
        self.file_info.has_path()
    }
    pub fn height(&self) -> LineIdx {
        self.lines.len()
    }
    pub fn insert_char(&mut self, character: char, at: Location) {
        debug_assert!(at.line_idx <= self.height());
        if at.line_idx == self.height() {
            self.lines.push(Line::from(&character.to_string()));
            self.dirty = true;
        } else if let Some(line) = self.lines.get_mut(at.line_idx) {
            line.insert_char(character, at.grapheme_idx);
            self.dirty = true;
        }
    }
    pub fn delete(&mut self, at: Location) {
        if let Some(line) = self.lines.get(at.line_idx) {
            if at.grapheme_idx >= line.grapheme_count()
                && self.height() > at.line_idx.saturating_add(1)
            {
                let next_line = self.lines.remove(at.line_idx.saturating_add(1));
                // clippy::indexing_slicing: We checked for existence of this line in the surrounding if statment
                #[allow(clippy::indexing_slicing)]
                self.lines[at.line_idx].append(&next_line);
                self.dirty = true;
            } else if at.grapheme_idx < line.grapheme_count() {
                #[allow(clippy::indexing_slicing)]
                self.lines[at.line_idx].delete(at.grapheme_idx);
                self.dirty = true;
            }
        }
    }
    pub fn insert_newline(&mut self, at: Location) {
        if at.line_idx == self.height() {
            self.lines.push(Line::default());
            self.dirty = true;
        } else if let Some(line) = self.lines.get_mut(at.line_idx) {
            let new = line.split(at.grapheme_idx);
            self.lines.insert(at.line_idx.saturating_add(1), new);
            self.dirty = true;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::Location;
    use std::fs;
    use std::path::Path;

    fn loc(line: usize, col: usize) -> Location {
        Location {
            line_idx: line,
            grapheme_idx: col,
        }
    }

    #[test]
    fn buffer_insert_char_basic() {
        let mut buffer = Buffer::default();

        buffer.insert_char('h', loc(0, 0));
        buffer.insert_char('i', loc(0, 1));

        assert_eq!(buffer.height(), 1);
        assert_eq!(buffer.grapheme_count(0), 2);
        assert!(buffer.is_dirty());
    }

    #[test]
    fn buffer_insert_newline() {
        let mut buffer = Buffer::default();

        buffer.insert_char('h', loc(0, 0));
        buffer.insert_char('i', loc(0, 1));

        buffer.insert_newline(loc(0, 2));

        assert_eq!(buffer.height(), 2);
        assert!(buffer.is_dirty());
    }

    #[test]
    fn buffer_insert_newline_middle() {
        let mut buffer = Buffer::default();

        buffer.insert_char('h', loc(0, 0));
        buffer.insert_char('e', loc(0, 1));
        buffer.insert_char('l', loc(0, 2));
        buffer.insert_char('l', loc(0, 3));
        buffer.insert_char('o', loc(0, 4));

        buffer.insert_newline(loc(0, 2));

        assert_eq!(buffer.height(), 2);
    }

    #[test]
    fn buffer_delete_char() {
        let mut buffer = Buffer::default();

        buffer.insert_char('h', loc(0, 0));
        buffer.insert_char('i', loc(0, 1));

        buffer.delete(loc(0, 1));

        assert_eq!(buffer.grapheme_count(0), 1);
    }

    #[test]
    fn buffer_delete_merge_lines() {
        let mut buffer = Buffer::default();

        buffer.insert_char('h', loc(0, 0));
        buffer.insert_newline(loc(0, 1));
        buffer.insert_char('i', loc(1, 0));

        buffer.delete(loc(0, 1));

        assert_eq!(buffer.height(), 1);
        assert_eq!(buffer.grapheme_count(0), 2);
    }

    #[test]
    fn buffer_search_forward_same_line() {
        let mut buffer = Buffer::default();

        for (i, c) in "hello world".chars().enumerate() {
            buffer.insert_char(c, loc(0, i));
        }

        let result = buffer.search_forward("world", loc(0, 0));

        assert!(result.is_some());
        let result = result.unwrap();

        assert_eq!(result.line_idx, 0);
        assert_eq!(result.grapheme_idx, 6);
    }

    #[test]
    fn buffer_search_forward_multiline() {
        let mut buffer = Buffer::default();

        for (i, c) in "hello".chars().enumerate() {
            buffer.insert_char(c, loc(0, i));
        }

        buffer.insert_newline(loc(0, 5));

        for (i, c) in "world".chars().enumerate() {
            buffer.insert_char(c, loc(1, i));
        }

        let result = buffer.search_forward("world", loc(0, 0));

        assert!(result.is_some());
        let result = result.unwrap();

        assert_eq!(result.line_idx, 1);
    }

    #[test]
    fn buffer_search_backward() {
        let mut buffer = Buffer::default();

        for (i, c) in "hello".chars().enumerate() {
            buffer.insert_char(c, loc(0, i));
        }

        buffer.insert_newline(loc(0, 5));

        for (i, c) in "hello".chars().enumerate() {
            buffer.insert_char(c, loc(1, i));
        }

        let result = buffer.search_backward("hello", loc(1, 5));

        assert!(result.is_some());
        let result = result.unwrap();

        assert_eq!(result.line_idx, 1);
    }

    #[test]
    fn buffer_height() {
        let mut buffer = Buffer::default();

        buffer.insert_char('a', loc(0, 0));
        buffer.insert_newline(loc(0, 1));
        buffer.insert_char('b', loc(1, 0));

        assert_eq!(buffer.height(), 2);
    }

    #[test]
    fn buffer_is_empty() {
        let buffer = Buffer::default();

        assert!(buffer.is_empty());
    }

    #[test]
    fn buffer_dirty_flag() {
        let mut buffer = Buffer::default();

        assert!(!buffer.is_dirty());

        buffer.insert_char('a', loc(0, 0));

        assert!(buffer.is_dirty());
    }

    #[test]
    fn buffer_save_and_load() {
        let file = "test_buffer_save.txt";

        let mut buffer = Buffer::default();

        buffer.insert_char('h', loc(0, 0));
        buffer.insert_char('i', loc(0, 1));

        buffer.save_as(file).unwrap();

        let loaded = Buffer::load(file).unwrap();

        assert_eq!(loaded.height(), 1);
        assert_eq!(loaded.grapheme_count(0), 2);

        fs::remove_file(file).unwrap();
    }

    #[test]
    fn buffer_is_file_loaded() {
        let mut buffer = Buffer::default();

        assert!(!buffer.is_file_loaded());

        buffer.save_as("test_file.txt").unwrap();

        assert!(buffer.is_file_loaded());

        fs::remove_file("test_file.txt").unwrap();
    }

    #[test]
    fn buffer_width_until() {
        let mut buffer = Buffer::default();

        buffer.insert_char('a', loc(0, 0));
        buffer.insert_char('b', loc(0, 1));
        buffer.insert_char('c', loc(0, 2));

        let width = buffer.width_until(0, 2);

        assert_eq!(width, 2);
    }

    #[test]
    fn buffer_multiple_newlines() {
        let mut buffer = Buffer::default();

        buffer.insert_char('a', loc(0, 0));
        buffer.insert_newline(loc(0, 1));
        buffer.insert_newline(loc(1, 0));
        buffer.insert_char('b', loc(2, 0));

        assert_eq!(buffer.height(), 3);
    }
}
