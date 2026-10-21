//! The only place with side effects: process arguments, stdout, stderr, exit codes.

use std::io::{self, Write};
use std::process::ExitCode;

use crate::domain::{self, Command};

const USAGE: &str = "usage: app <name> | app version";

#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    match domain::parse_command(args) {
        Command::Greet(name) => match domain::greeting(&name) {
            Ok(text) => emit(&mut io::stdout().lock(), &text, ExitCode::SUCCESS),
            Err(err) => emit(
                &mut io::stderr().lock(),
                &err.to_string(),
                ExitCode::FAILURE,
            ),
        },
        Command::Version => emit(&mut io::stdout().lock(), &version(), ExitCode::SUCCESS),
        Command::Usage => emit(&mut io::stderr().lock(), USAGE, ExitCode::from(2)),
    }
}

fn version() -> String {
    format!("app {}", env!("CARGO_PKG_VERSION"))
}

// A closed pipe (`app Ada | head -0`) must not panic the way `println!` would.
fn emit(out: &mut impl Write, text: &str, code: ExitCode) -> ExitCode {
    match writeln!(out, "{text}") {
        Ok(()) => code,
        Err(_) => ExitCode::FAILURE,
    }
}
