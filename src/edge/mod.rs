//! The only place with side effects: process arguments, stdout, stderr, exit codes.

use std::io::{self, Write};
use std::process::ExitCode;

const USAGE: &str = "usage: charter doctor";

#[must_use]
pub fn run() -> ExitCode {
    emit(&mut io::stderr().lock(), USAGE, ExitCode::from(2))
}

// A closed pipe (`charter Ada | head -0`) must not panic the way `println!` would.
fn emit(out: &mut impl Write, text: &str, code: ExitCode) -> ExitCode {
    match writeln!(out, "{text}") {
        Ok(()) => code,
        Err(_) => ExitCode::FAILURE,
    }
}
