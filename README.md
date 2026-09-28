kk
==

[![Crates.io](https://img.shields.io/crates/v/kk.svg)](https://crates.io/crates/kk)
[![Actions Status](https://github.com/sile/kk/workflows/CI/badge.svg)](https://github.com/sile/kk/actions)
![License](https://img.shields.io/crates/l/kk)

A TUI text editor.

kk is an abbreviation for "KaKu (書く)" (meaning "to write" in Japanese).

Features
--------

- E-ink display friendly with black and white interface
- Mouse support (click, scroll up / down)
- No implicit behaviors
  - No background tasks running
- Not an environment, just a writing tool
  - I use kk in combination with the following tools:
    - tmux (for multi-window management)
    - mamediff (for git diff management)
    - mamegrep (for multi-file search)
    - attini (for LLM coding agent)

Intentionally Unsupported Features
---------------------------------

- Color
- Syntax Highlight
- Plugin / Extension system
- Configuration file
- Multi (split) windows
- LSP

Installation
------------

```console
$ cargo install kk

$ kk -h
A TUI text editor

Usage: kk [OPTIONS] FILE

Example:
  $ kk /path/to/file

Arguments:
  FILE A file, optionally followed by :LINE to start at, or :LINE:COLUMN

Options:
      --version    Print version
  -h, --help       Print help ('--help' for full help, '-h' for summary)
  -c, --create-new Create the file, which must not already exist
```
