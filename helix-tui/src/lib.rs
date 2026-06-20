#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
pub mod backend;
pub mod buffer;
pub mod layout;
pub mod symbols;
pub mod terminal;
pub mod text;
pub mod widgets;

pub use self::terminal::{Terminal, TerminalOptions, Viewport};
