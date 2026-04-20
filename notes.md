# Yonro's Text Editor - Implementation Notes & Learning Journey

Building a terminal-based text editor in Rust following and extending the [Hecto tutorial](https://www.flenker.blog/hecto/).

## Table of Contents

1. [Phase I: Raw I/O Mode & Keypressing](#phase-i--raw-io-mode--keypressing)
2. [Phase II: Text Viewing & Caret Movement](#phase-ii--text-viewing--caret-movement)
3. [Phase III: Text Editing](#phase-iii--text-editing)
4. [Phase IV: Search & Fuzzy Matching](#phase-iv--search--fuzzy-matching)
5. [Phase V: Syntax Highlighting](#phase-v--syntax-highlighting)
6. [Phase VI: Plugin System](#phase-vi--plugin-system)
7. [Phase VII: Writer-Specific Features](#phase-vii--writer-specific-features)

---

## Phase I: Raw I/O Mode & Keypressing

### Challenge: Terminal Mode Abstraction

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
pub struct Document {
    lines: Vec<Line>,
    filename: Option<String>,
}

pub struct Line {
    text: String,  // UTF-8 string (grapheme-aware)
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
```

#### Implementation in Editor

```rust
pub struct Line {
    text: String,
    graphemes: Vec<usize>,  // Byte indices of each grapheme
}

impl Line {
    pub fn new(text: String) -> Self {
        let graphemes = UnicodeSegmentation::graphemes(text.as_str(), true)
            .map(|g| g.len())  // Byte length
            .scan(0, |acc, len| {
                let result = *acc;
                *acc += len;
                Some(result)
            })
            .collect();

        Line { text, graphemes }
    }

    pub fn get_grapheme(&self, index: usize) -> Option<&str> {
        if index >= self.graphemes.len() {
            return None;
        }
        let start = self.graphemes[index];
        let end = if index + 1 < self.graphemes.len() {
            self.graphemes[index + 1]
        } else {
            self.text.len()
        };
        Some(&self.text[start..end])
    }
}
```

#### Width Calculation

Graphemes have varying display widths:

```rust
use unicode_width::UnicodeWidth;

"a".width()           // 1 (normal ASCII)
"é".width()           // 1 (combining character)
"👨".width()          // 2 (emoji - double width)
"➜".width()           // 2 (symbol - double width)
"\t".width()          // 8 (tab - variable)
```

This is critical for:

- Cursor positioning
- Line wrapping
- Text alignment

**Key Learning:** Never count characters when positioning; always use `UnicodeWidth`.

---

## Phase III: Text Editing

---

## Phase IV: Search & Fuzzy Matching

---

## Phase VI: Plugin System

---

## Key Learnings & Takeaways

---
