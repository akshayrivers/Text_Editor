[![Build](https://github.com/akshayrivers/Text_Editorr/actions/workflows/rust.yml/badge.svg)](https://github.com/akshayrivers/Text_Editorr/actions/workflows/rust.yml)
### Following the https://www.flenker.blog/hecto/

We are trying to build a text editor in RUST

# Phase I : RAW I/O mode and basic keypressing

Entering raw I/O mode, meaning we do not process any I/O commands.
Raw mode in text editors is a terminal configuration that disables input/output processing, allowing the application to directly read each keystroke in real-time without waiting for a newline (Enter) or echoing characters. It overrides the default "cooked" (canonical) mode to allow for full control over screen rendering, such as highlighting and cursor movement, commonly used in terminal-based editors like Vim or nano[We are aslo building something like them]

- We have a challenge of having different abstractions for unix and windows which we will need to handle seperately for entering and disabling
  the raw mode along with other specific feautures. So it was deicided that we are going to use Crossterm crate.

## Crossterm:

Cross-platform Terminal Manipulation Library
Crossterm is a pure-rust, terminal manipulation library that makes it possible to write cross-platform text-based interfaces.
This crate supports all UNIX and Windows terminals down to Windows 7.
Command API
The command API makes the use of crossterm much easier and offers more control over when and how a command is executed. A command is just an action you can perform on the terminal e.g. cursor movement.

The command API offers:

- Better Performance.
- Complete control over when to flush.
- Complete control over where the ANSI escape commands are executed to.
- Way easier and nicer API.

There are two ways to use the API command:

1.  Functions can execute commands on types that implement Write. Functions are easier to use and debug. There is a disadvantage, and that is that there is a boilerplate code involved.
2.  Macros are generally seen as more difficult and aren’t always well supported by editors but offer an API with less boilerplate code. If you are not afraid of macros, this is a recommendation.

Linux and Windows 10 systems support ANSI escape codes. Those ANSI escape codes are strings or rather a byte sequence. When we write and flush those to the terminal we can perform some action. For older windows systems a WinAPI call is made.

### Modules:

clipboard: A module for clipboard interaction
cursor: A module to work with the terminal cursor
event: A module to read events.
style: A module to apply attributes and colors on your text.
terminal: A module to work with the terminal.
tty: A module to query if the current instance is a tty. Making it a little more convenient and safe to query whether something is a terminal teletype or not. This module defines the IsTty trait and the is_tty method to return true if the item represents a terminal.

### Macros:

execute: Executes one or more command(s).
queue: Queues one or more command(s) for further execution.

### Traits:

Command: An interface for a command that performs an action on the terminal.
ExecutableCommand: An interface for types that can directly execute commands.
QueueableCommand: An interface for types that can queue commands for further execution.
SynchronizedUpdate: An interface for types that support synchronized updates.

### Command Execution

There are two different ways to execute commands:

#### Lazy Execution [flush]

Flushing bytes to the terminal buffer is a heavy system call. If we perform a lot of actions with the terminal, we want to do this periodically - like with a TUI editor - so that we can flush more data to the terminal buffer at the same time.

Crossterm offers the possibility to do this with queue. With queue you can queue commands, and when you call Write::flush these commands will be executed.

#### Direct Execution [execute]

For many applications it is not at all important to be efficient with ‘flush’ operations. For this use case there is the execute operation. This operation executes the command immediately, and calls the flush under water.

You can pass a custom buffer implementing std::io::Write to this execute operation. The commands will be executed on that buffer. The most common buffer is std::io::stdout however, std::io::stderr is used sometimes as well.

IN OUR CODE, WE HAVE USED THE LAZY EXECUTIONS USING THE flush! macro EXTENSIVELY LIKE IN THIS FUNCTION:

```
    use crossterm::{queue, Command};
    use std::io::{stdout, Error, Write};
    fn queue_command<T: Command>(command: T) -> Result<(), Error> {
        queue!(stdout(), command)?;
        Ok(())
    }
```

ALONG WITH THIS WE HAVE ALSO EXPLICITLY CALLED flush from our std::io which jsut empties out the std buffer.

```
    use std::io::{stdout, Error, Write};
    pub fn execute() -> Result<(), Error> {
        stdout().flush()?;
        Ok(())
    }
```

- We have also used the enable and disable raw mode functions directly from the crossterm modules.
- We also have added the fucntioning of entering a spearate alternate screen so that we could have different buffers from the main terminal and our text editor which helps us in cleaning and logging any info if required. This was also implememted through the crossterms defined fucntions

```
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    };
```

# Phase II :Text viewer + caret movement

## Buffer + Rendering + Reading from a file

## Scrolling and Snapping of Caret

## Support for non ASCII characters [Graphemes]

# Phase III :Text editing

# Phase IV : Search [Fuzzy support]

# Phase V : Syntax Highlighting

# Phase VI: Plugin Support

# Phase VII: Word analytics Plugin {writer specific plugins}
