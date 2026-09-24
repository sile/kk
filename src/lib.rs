//! `kk`'s Sans I/O core.
//!
//! This crate holds everything that does not touch a file descriptor, the file
//! system, or a clock: the buffer model ([`TextBuffer`]), the editor state
//! ([`State`]) with its pure handlers, the key bindings, and the renderers that
//! paint a [`Frame`](tuinix::Frame) from that state.
//!
//! The one thing the core decides but does not perform is I/O: loading a file,
//! saving a buffer, and reading and writing the terminal all happen at the edge
//! in the binary's `main` function and its `app` module. The core hands back
//! plain values -- the text to write, the frame to draw -- and is driven
//! entirely by calls from the edge. Nothing here calls back out.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod action;
mod binding;
mod buffer;
mod clipboard;
mod fmt;
mod grep_mode;
mod message_line;
mod state;
mod status_line;
mod terminal;
mod text_area;

pub use action::{Action, GrepAction};
pub use binding::{Context, Resolved, resolve};
pub use buffer::{TextBuffer, TextLine, TextPosition};
pub use clipboard::Clipboard;
pub use fmt::input;
pub use grep_mode::{GrepMode, GrepQueryRenderer, Highlight};
pub use message_line::MessageLineRenderer;
pub use state::State;
pub use status_line::StatusLineRenderer;
pub use terminal::{char_cols, put_str, str_cols};
pub use text_area::TextAreaRenderer;
