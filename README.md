kk
--

[![kk](https://img.shields.io/crates/v/kk.svg)](https://crates.io/crates/kk)
![License](https://img.shields.io/crates/l/kk)

A TUI text editor.

**WIP** 

Features
--------

- Black and white e-ink display friendly
- No implicit behaviors
  - Almost every actions are explicitly defined by the config file and triggered by user's key input
  - No background tasks
- Not environment, just a writing tool
- File oriented (e.g, clipboard, cursor position, ...)
  - Easily integrated with other command-line tools

Intentionally Unsupported Features
---------------------------------

- Color
- Syntax Highlight
- Plugin / Extension System
- Multi (split) windows
- LSP (TBD)
  - go-to-definition and completion would be necessary
