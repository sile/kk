kk
--

[![kk](https://img.shields.io/crates/v/kk.svg)](https://crates.io/crates/kk)
![License](https://img.shields.io/crates/l/kk)

**WIP** 

A TUI text editor.

kk is an abbreviation for "KaKu (書く)" (meaning "to write" in Japanese).

Features
--------

- E-ink display friendly with black and white interface
- No implicit behaviors
  - Nearly all actions are explicitly defined in the configuration file and triggered by user key input
  - No background tasks running
- Not an environment, just a writing tool
  - I use kk in combination with the following tools:
    - tmux (for multi-window management)
    - mamediff (for git diff management)
    - mamegrep (for multi-file search)
    - daberu (for LLM)
    - niho (for Japanese text input)
- File-oriented design (e.g., clipboard, cursor position, etc.)
  - Easy integration with other command-line tools

Intentionally Unsupported Features
---------------------------------

- Color
- Syntax Highlight
- Plugin / Extension System
- Multi (split) windows
- LSP

