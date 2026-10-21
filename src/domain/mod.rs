//! Pure functions over plain data. No IO, no clock, no mutation: `rules/` enforces it.

mod command;
mod greeting;

pub use command::{Command, parse_command};
pub use greeting::{GreetError, greeting};
