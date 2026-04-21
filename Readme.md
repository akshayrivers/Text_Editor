[![Build](https://github.com/akshayrivers/Text_Editorr/actions/workflows/rust.yml/badge.svg)](https://github.com/akshayrivers/Text_Editorr/actions/workflows/rust.yml)

# Yonro's Terminal based text editor

Initially it had been a faithful implementation of the text editor built in the [Hecto tutorial](https://www.flenker.blog/hecto/). But I have enhanced and changed it into something which I can use as a writer and also it is also to challenge myself as a programmer.
For detailed notes please refer to /notes.md

## Overview & Design Philosophy

Instead of jumping directly into features, the editor was built in layers:

- Terminal control
- Rendering
- Editing
- Search
- Highlighting
- Extensibility

Each phase builds on the previous one, gradually evolving the editor from a
simple terminal viewer into a fully functional text editor.

Some guiding principles during development:

- Keep core editor synchronous and stable
- Prefer simple data structures first
- Unicode correctness over premature optimization
- Stateless rendering where possible
- Design for extensibility (plugins later)

This allowed the editor to evolve organically while keeping complexity manageable.

### Core Components

| Component   | Responsibility                     |
| ----------- | ---------------------------------- |
| Editor      | Command dispatch and state         |
| Buffer      | Text storage and manipulation      |
| Line        | Grapheme-aware text representation |
| Highlighter | Syntax + search highlighting       |
| Renderer    | Terminal drawing                   |
| Terminal    | Crossterm abstraction              |

This separation keeps the core editor logic clean and extensible.

## How text editors work:

| Feature             | GNU Nano           | Vim                          | Visual Studio Code  | Yonro's Editor                      |
| ------------------- | ------------------ | ---------------------------- | ------------------- | ----------------------------------- |
| Type                | Terminal           | Terminal                     | GUI                 | Terminal                            |
| Architecture        | Simple             | Modal + Complex              | Plugin-driven       | Layered + Extensible                |
| Learning Curve      | Very Easy          | Hard                         | Easy                | Moderate                            |
| Editing Model       | Direct editing     | Modal editing                | Direct editing      | Direct editing                      |
| Buffer Structure    | Line-based         | Advanced internal structures | Rope / Piece Table  | Line-based (future rope planned)    |
| Rendering           | Full screen redraw | Optimized redraw             | GPU / UI framework  | Incremental terminal rendering      |
| Unicode Support     | Limited            | Good                         | Excellent           | Unicode + Grapheme aware            |
| Syntax Highlighting | Basic              | Advanced                     | Very advanced       | Basic → Improving                   |
| Plugin Support      | Very limited       | Extensive                    | Extremely extensive | Planned architecture                |
| LSP Support         | No                 | Yes (plugins)                | Built-in            | Planned                             |
| Multi-pane UI       | No                 | Yes                          | Yes                 | Planned                             |
| File Explorer       | No                 | Plugin                       | Built-in            | Planned                             |
| Performance         | Fast               | Very fast                    | Heavy but optimized | Fast                                |
| Memory Usage        | Very low           | Low                          | High                | Low                                 |
| Configuration       | Minimal            | Very powerful                | GUI + config        | Planned                             |
| Philosophy          | Simple editor      | Power user editor            | IDE-like editor     | Learning + architecture exploration |

## License

This project started as an implementation inspired by the hecto tutorial:
https://www.flenker.blog/hecto/

However, the current implementation has diverged significantly and includes
additional features, architectural changes, and enhancements.

This project is licensed under the MIT License.
