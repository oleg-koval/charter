use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use super::{emit, load};
use crate::domain::{self, Mechanism};

#[must_use]
pub fn verify_pr(file: &Path, dir: &Path, json: bool) -> ExitCode {
    let mechanism = match mechanism(dir) {
        Ok(mechanism) => mechanism,
        Err(message) => return emit(&mut io::stderr().lock(), &message, ExitCode::FAILURE),
    };
    let body = match fs::read_to_string(file) {
        Ok(body) => body,
        Err(error) => {
            return emit(
                &mut io::stderr().lock(),
                &format!("{}: {error}", file.display()),
                ExitCode::FAILURE,
            );
        }
    };
    report(&mechanism, &body, json)
}

fn report(mechanism: &Mechanism, body: &str, json: bool) -> ExitCode {
    let lines = domain::verify_pr(mechanism, body)
        .iter()
        .map(|gap| format!("{}  {}", gap.heading, gap.message))
        .collect::<Vec<_>>();
    let code = if lines.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    let text = if json {
        domain::render_verdict_json(&lines)
    } else {
        lines.join("\n")
    };
    if lines.is_empty() && !json {
        code
    } else {
        emit(&mut io::stdout().lock(), &text, code)
    }
}

fn mechanism(dir: &Path) -> Result<Mechanism, String> {
    let (policy, mechanism) = load(dir)?;
    let gaps = domain::diagnose(&policy, &mechanism);
    if gaps.is_empty() {
        Ok(mechanism)
    } else {
        Err(gaps
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}
