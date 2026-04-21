> This document tracks architectural decisions and implementation details while
> building a terminal text editor from scratch in Rust specifically for my personal use as a writer and a programmer.

# Yonro's Text Editor - Implementation Notes & Learning Journey

Initially it had been a faithful implementation of the text editor built in the [Hecto tutorial](https://www.flenker.blog/hecto/). But I have enhanced and changed it into something which I can use as a writer and also it is also to challenge myself as a programmer.

## Table of Contents

### Core Development Phases

1. [Phase I: Raw I/O Mode & Keypressing](#phase-i-raw-io-mode--keypressing)
2. [Phase II: Text Viewing & Caret Movement](#phase-ii-text-viewing--caret-movement)
3. [Phase III: Text Editing](#phase-iii-text-editing)
4. [Phase IV: Search & Matching](#phase-iv-search--matching)
5. [Phase V: Syntax Highlighting](#phase-v-syntax-highlighting)

### Editor Features & Extensions

6. [Phase VI: Miscellaneous Features](#phase-vi-miscellaneous-features)
7. [Phase VII: Plugin System](#phase-vii-plugin-system)

### Future Work

8. [Future Plans](#future-plans)
9. [Bugs/Backlog](#bugs--backlog)

---

## Phase I: Raw I/O Mode & Keypressing

### Terminal Mode Abstraction

Different operating systems handle terminal I/O differently:

- **Unix/Linux**: POSIX termios API for raw mode
- **Windows**: Console API with different flags and modes
- **macOS**: Inherited from Unix but with specific quirks

**Solution**: Use the `crossterm` crate for cross-platform abstraction.

### What is Raw Mode?

Raw mode disables input/output processing, allowing real-time keystroke detection without waiting for Enter:

```
Normal Mode:        Raw Mode:
User types: a       Immediate event: KeyPress('a')
User types: b       Immediate event: KeyPress('b')
User types: ↵       Immediate event: KeyPress('Enter')
Process 'ab↵'       Each keystroke processed individually
```

### Crossterm: Cross-Platform Terminal Manipulation

**Why Crossterm?**

- Pure Rust library (no C dependencies)
- Supports UNIX and Windows down to Windows 7
- Better performance than alternatives
- Comprehensive API with command system

#### Key Modules

| Module      | Purpose                                    |
| ----------- | ------------------------------------------ |
| `terminal`  | Screen size, raw mode, alternate screen    |
| `cursor`    | Position, show/hide, shape                 |
| `event`     | Keyboard/mouse input                       |
| `style`     | Colors, attributes (bold, underline, etc.) |
| `clipboard` | System clipboard access                    |

### Command Execution Patterns

Crossterm offers two execution strategies:

#### 1. **Lazy Execution (Queuing)**

```rust
use crossterm::{queue, ExecutableCommand};
use std::io::{stdout, Write};

let mut stdout = stdout();
queue!(stdout, MoveTo(0, 0), Print("Hello"))?;
stdout.flush()?;  // Actually execute
```

**Advantages:**

- Better performance (fewer system calls)
- Batch operations
- Full control over flush timing

**Use case:** Real-time TUI rendering (text editors, dashboards)

#### 2. **Direct Execution**

```rust
use crossterm::execute;

execute!(stdout(), MoveTo(0, 0), Print("Hello"))?;
```

**Advantages:**

- Simpler API
- Immediate execution

**Use case:** One-off operations, scripts

**In our code:** We extensively use lazy execution with `queue!` because text editors need optimal performance.

### Implementation: Raw Mode Setup

```rust
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use std::io::stdout;

pub fn enable_editor_mode() -> Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    Ok(())
}

pub fn disable_editor_mode() -> Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
```

**What happens:**

1. `enable_raw_mode()` - Terminal stops processing, sends keystrokes directly
2. `EnterAlternateScreen` - Switches to alternate buffer (preserves terminal state)
3. On exit: Reverse operations restore terminal

### Handling Input Events

```rust
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

match event::read()? {
    Event::Key(key_event) => {
        match key_event.code {
            KeyCode::Char('q') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+Q pressed
            }
            KeyCode::Char(c) => {
                // Regular character
            }
            KeyCode::Enter => {
                // Newline
            }
            _ => {}
        }
    }
    Event::Resize(width, height) => {
        // Terminal resized
    }
    _ => {}
}
```

**Key Learning:** Non-blocking input polling is crucial for responsive editors.

---

## Phase II: Text Viewing & Caret Movement

### Data Structure: Line-based Storage

```rust
pub struct Buffer {
    lines: Vec<Line>,
    file_info: FileInfo,
    dirty: bool,
}

pub struct Line {
    fragments: Vec<TextFragment>,
    string: String,
}
pub struct TextFragment {
    pub grapheme: String,
    pub rendered_width: GraphemeWidth,
    pub replacement: Option<char>,
    pub start_byte_idx: ByteIdx,
}
```

**Why Vec of Lines?**

- Easy horizontal navigation within lines
- Fast vertical navigation (random access)
- Natural file structure representation
- Efficient rendering (render only visible lines)

### Scrolling & Viewport Management

The key challenge: **Map cursor position to screen position**

```
File content:         Screen (24 lines):
Line 1    ┐           Line 50 (visible as line 1)
Line 2    │ offset    Line 51 (visible as line 2)
Line 3    │ = 49      Line 52 (visible as line 3)
...       │           ...
Line 50   ├─ snap     Line 73 (visible as line 24)
Line 51 * │           * = cursor position
Line 52   │
Line 53   │
...       ┘
```

so we went with visualizing a canvas of infinite rows and columns from which we cna choose the size of the terminal to show them correctly
Ultimately we have used various snapping and centering functions to keep the content correct.

#### Viewport Snapping

Three snapping strategies:

1. **Top Snap:** Keep cursor at top of screen

```rust
if cursor_y < offset_y {
    offset_y = cursor_y;  // Snap to top
}
```

2. **Bottom Snap:** Keep cursor at bottom of screen

```rust
if cursor_y >= offset_y + screen_height {
    offset_y = cursor_y - screen_height + 1;
}
```

3. **Center Snap:** Keep cursor centered (better UX)

```rust
let desired_offset = cursor_y.saturating_sub(screen_height / 2);
offset_y = desired_offset;
```

### Grapheme Support: Unicode Complexity

#### The Problem

```rust
// These look like 3 characters but aren't:
"e" + "◌́" = "é"        // e + combining acute
"👨" + "👩" + "👧" = "👨‍👩‍👧"  // Family emoji (ZWJ sequence)

// String indexing is dangerous:
"hello".chars().nth(2)        // Safe: 'l'
"café".chars().nth(3)         // Unsafe: accented é
"👨‍👩‍👧".chars().nth(1)  // Wrong: gets middle component
```

#### Solution: Grapheme Segmentation

```rust
use unicode_segmentation::UnicodeSegmentation;

let text = "café";
let graphemes: Vec<&str> = text.graphemes(true).collect();
// graphemes = ["c", "a", "f", "é"]  ✓ Correct!

let text = "👨‍👩‍👧";
let graphemes: Vec<&str> = text.graphemes(true).collect();
// graphemes = ["👨‍👩‍👧"]  ✓ Entire emoji family as one unit!


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
```

#### Implementation in Editor

```rust
pub struct Line {
    fragments: Vec<TextFragment>,
    string: String,
}

impl Line {
...
}
impl Display for Line {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.string)
    }
}

impl Deref for Line {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.string
    }
}
pub struct TextFragment {
    pub grapheme: String,
    pub rendered_width: GraphemeWidth,
    pub replacement: Option<char>,
    pub start_byte_idx: ByteIdx,
}

```

#### Width Calculation

Graphemes have varying display widths:

```rust
use unicode_width::UnicodeWidth;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

"a".width()           // 1 (normal ASCII)
"é".width()           // 1 (combining character)
"👨".width()          // 2 (emoji - double width)
"➜".width()           // 2 (symbol - double width)
"\t".width()          // 8 (tab - variable)


#[derive(Copy, Clone, Debug)]
pub enum GraphemeWidth {
    Half,
    Full,
}
impl From<GraphemeWidth> for usize {
    fn from(val: GraphemeWidth) -> Self {
        match val {
            GraphemeWidth::Half => 1,
            GraphemeWidth::Full => 2,
        }
    }
}

```

This is critical for:

- Cursor positioning
- Line wrapping
- Text alignment

**Key Learning:** Never count characters when positioning; always use `UnicodeWidth`.

---

## Phase III: Text Editing

Under the hood we have primarily used the crossterms events to execute the commands which we are differentiating on our own custom logic

```rust
pub enum Command {
    Move(Move),
    Edit(Edit),
    System(System),
}
pub enum Move {
    PageUp,
    PageDown,
    StartOfLine,
    EndOfLine,
    Up,
    Left,
    Right,
    Down,
}
pub enum Edit {
    Insert(char),
    InsertNewLine,
    Delete,
    DeleteBackward,
}
pub enum System {
    Save,
    Resize(Size),
    Quit,
    Dismiss,
    Search,
}

```

Where we first correctly track which command is issued and which underlying function needs to performed
and the commands are then dispatched though process_command:

```rust
    fn process_command(&mut self, command: Command) {
        if let System(Resize(size)) = command {
            self.handle_resize_command(size);
            return;
        }
        match self.prompt_type {
            PromptType::Search => self.process_command_during_search(command),
            PromptType::Save => self.process_command_during_save(command),
            PromptType::None => self.process_command_no_prompt(command),
        }
    }
```

we have tried to divulge work further into more functions to keep the flow clean and modular

```rust
 ...
    fn insert_char(&mut self, character: char) {
        let old_len = self.buffer.grapheme_count(self.text_location.line_idx);
        self.buffer.insert_char(character, self.text_location);
        let new_len = self.buffer.grapheme_count(self.text_location.line_idx);
        let grapheme_delta = new_len.saturating_sub(old_len);
        if grapheme_delta > 0 {
            // we move right with scroll handling
            self.handle_move_command(Move::Right);
        }
        self.mark_redraw(true);
    }
    ...
```

## Phase IV: Search & Fuzzy Matching

### Search Architecture

Since our editor already maintains a **line-based buffer with grapheme-aware indexing**, implementing search becomes a matter of:

1. Tracking search query
2. Traversing buffer line-by-line
3. Returning match locations
4. Highlighting matches during rendering

The search system is built around three core components:

- **Editor Search State**
- **Buffer Search Traversal**
- **Line-Level Matching**

---

### Search State Management

Search state is stored using a `SearchInfo` structure:

- Current query
- Selected match
- Navigation direction

Search begins from the current cursor location:

```rust
pub fn search(&mut self, query: &str) {
    if let Some(search_info) = &mut self.search_info {
        search_info.query = Some(Line::from(query));
    }
    self.search_in_direction(self.text_location, SearchDirection::default());
}
```

#### Key Design Decision

Search is **cursor-relative**, meaning:

- Search starts from cursor
- Navigation continues from last match
- Matches wrap around buffer

This mimics behavior of editors like:

- Vim
- VSCode
- Sublime

---

### Directional Search

Search supports:

- Forward search
- Backward search
- Next match
- Previous match

Core search dispatcher:

```rust
fn search_in_direction(&mut self, from: Location, direction: SearchDirection)
```

Behavior:

- Extract query
- Choose search direction
- Search buffer
- Move cursor to result
- Center viewport

This keeps search responsive and user-friendly.

---

### Next / Previous Match Navigation

Search navigation is implemented using:

```rust
pub fn search_next(&mut self)
pub fn search_prev(&mut self)
```

#### Forward Search

Forward search advances cursor:

```rust
let step_right = min(query.grapheme_count(), 1);
```

This ensures:

- Avoids infinite loops
- Moves at least one grapheme forward

---

### Buffer Search Implementation

The buffer performs **line-by-line traversal**:

```rust
pub fn search_forward(&self, query: &str, from: Location)
```

Key idea:

- Iterate lines
- Search within line
- Return first match

Implementation uses iterator chaining:

```rust
.lines
.iter()
.enumerate()
.cycle()
.skip(from.line_idx)
.take(self.lines.len() + 1)
```

### Why `.cycle()` ?

This enables **wrap-around search**:

```
File:

Line 1
Line 2
Line 3
Line 4

Cursor at Line 3

Search Forward:

Line 3 → Line 4 → Line 1 → Line 2
```

This matches modern editor behavior.

---

### Line-Level Search

Each `Line` handles substring matching:

```rust
pub fn search_forward(
    &self,
    query: &str,
    from_grapheme_idx: GraphemeIdx,
)
```

Steps:

1. Convert grapheme index → byte index
2. Extract substring
3. Run `match_indices()`
4. Convert byte index → grapheme index

This ensures:

- Unicode safety
- Grapheme correctness
- Accurate cursor placement

---

### Finding All Matches

Search uses:

```rust
pub fn find_all(&self, query: &str, range: Range<ByteIdx>)
```

This returns:

```rust
Vec<(ByteIdx, GraphemeIdx)>
```

Why both?

| Value       | Purpose            |
| ----------- | ------------------ |
| ByteIdx     | Rendering          |
| GraphemeIdx | Cursor positioning |

This separation is crucial because:

- Rendering uses byte offsets
- Cursor uses grapheme positions

---

##Search Highlighting

Once matches are found, we highlight them during rendering.

This is implemented using:

```rust
SearchResultHighlighter
```

This component:

- Finds all matches in line
- Marks selected match
- Returns annotations

---

### Highlighting Architecture

Rendering pipeline:

```
Line
 ↓
Syntax Highlighter
 ↓
Search Highlighter
 ↓
Annotations
 ↓
Renderer
```

Search highlighting integrates cleanly with syntax highlighting.

---

### Highlight All Matches

```rust
fn highlight_matched_words(&self, line: &Line, result: &mut Vec<Annotation>)
```

This:

- Finds all matches
- Creates annotations
- Marks match ranges

```rust
Annotation {
    annotation_type: AnnotationType::Match,
    start_byte_idx,
    end_byte_idx
}
```

This keeps rendering stateless and modular.

---

### Highlight Selected Match

The currently selected match is highlighted differently:

```rust
AnnotationType::SelectedMatch
```

This enables:

- Bright highlight for current match
- Dim highlight for other matches

Example:

```
hello world hello

      ^ selected
```

---

### Annotation System

Annotations are used for:

- Syntax highlighting
- Search highlighting
- Future features

Example:

```rust
pub struct Annotation {
    annotation_type: AnnotationType,
    start_byte_idx: ByteIdx,
    end_byte_idx: ByteIdx,
}
```

This design allows:

- Multiple overlapping highlights
- Layered rendering
- Plugin extensibility

---

### Why Highlight During Rendering?

Instead of modifying buffer:

We:

- Keep buffer immutable
- Apply highlights dynamically

Advantages:

- No mutation overhead
- Cleaner architecture
- Plugin-friendly

---

## Current Search Features

Currently implemented:

- Forward search
- Backward search
- Wrap-around search
- Highlight all matches
- Highlight selected match
- Grapheme-safe matching

---

## Future Improvements

## Case-Insensitive Search

Example:

```
Hello
hello
HELLO
```

---

## Regex Search

Possible integration:

- `regex`
- `fancy-regex`

---

## Fuzzy Search

Instead of exact match:

```
query: edt

matches:

editor
edited
editing
```

Possible libraries:

- fuzzy-matcher
- skim

---

## Search Panel

Future UI:

```
Search: hello
Matches: 12
```

---

## Incremental Search

Search updates as user types:

```
h → matches
he → refined matches
hel → final matches
```

---

### Key Learning

Search in text editors is deceptively complex:

- Unicode safe indexing
- Efficient traversal
- Rendering highlights
- Navigation state

Even "simple search" requires:

- Buffer traversal
- Unicode correctness
- Rendering integration

This implementation now provides a **solid foundation** for:

- Fuzzy search
- Regex search
- LSP references
- Symbol search

---

## Phase V: Syntax Highlighting

For building the highlighter, we implemented a trait-based architecture that allows different syntax highlighters to be plugged in depending on the file type. Currently, three file types are supported:

- Rust
- Markdown
- Plain Text

Each syntax highlighter implements the `SyntaxHighlighter` trait and is dynamically dispatched through the main `Highlighter` struct. This design keeps the highlighting logic modular and extensible, making it straightforward to add support for additional languages in the future.

The `Highlighter` combines syntax highlighting and search result highlighting. Both highlighters operate independently and return annotations which are merged during rendering.

Unlike the earlier implementation, highlights are now computed **on demand** rather than being cached. This removes synchronization issues, reduces memory usage, and simplifies invalidation logic.

```rust
#[derive(Default)]
pub struct Highlighter<'a> {
    syntax_highlighter: Option<Box<dyn SyntaxHighlighter>>,
    search_result_highlighter: Option<SearchResultHighlighter<'a>>,
}
```

The `SyntaxHighlighter` trait defines a single method that computes annotations for a given line:

```rust
pub trait SyntaxHighlighter {
    fn highlight(&mut self, idx: LineIdx, line: &Line) -> Vec<Annotation>;
}
```

Depending on the file type, an appropriate syntax highlighter is created:

```rust
fn create_syntax_highlighter(file_type: FileType) -> Option<Box<dyn SyntaxHighlighter>> {
    match file_type {
        FileType::Rust => Some(Box::<RustSyntaxHighlighter>::default()),
        FileType::Text => None,
        FileType::MarkDown => None,
    }
}
```

Search highlighting is implemented using a dedicated highlighter that operates alongside syntax highlighting:

```rust
#[derive(Default)]
pub struct SearchResultHighlighter<'a> {
    matched_word: &'a str,
    selected_match: Option<Location>,
}
```

Both syntax highlighting and search highlighting generate `Annotation` values, which are merged by the main `Highlighter` before rendering.

This architecture provides:

- Modular syntax highlighting
- Dynamic dispatch for extensibility
- Stateless, on-demand highlighting
- Clean separation of concerns
- Easy addition of new languages

---

## Phase VI: Miscellaneous Features

### Unit Tests

### Undo / Redo

### Copy / Paste

### Logging

---

## Phase VII: Plugin System

---

## Future Plans

---

## Bugs / Backlog
