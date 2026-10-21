use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use super::{emit, load};
use crate::domain::{self, Policy};

#[must_use]
pub fn rubric(dir: &Path, json: bool) -> ExitCode {
    let policy = match policy(dir) {
        Ok(policy) => policy,
        Err(message) => return emit(&mut io::stderr().lock(), &message, ExitCode::FAILURE),
    };
    let text = domain::render_rubric(&policy);
    let text = if json {
        match serde_json::to_string(&text) {
            Ok(encoded) => encoded,
            Err(error) => {
                return emit(
                    &mut io::stderr().lock(),
                    &error.to_string(),
                    ExitCode::FAILURE,
                );
            }
        }
    } else {
        text
    };
    emit(&mut io::stdout().lock(), &text, ExitCode::SUCCESS)
}

#[must_use]
pub fn verify_review(file: &Path, dir: &Path, json: bool) -> ExitCode {
    let policy = match policy(dir) {
        Ok(policy) => policy,
        Err(message) => return emit(&mut io::stderr().lock(), &message, ExitCode::FAILURE),
    };
    let rows = match read_rows(file) {
        Ok(rows) => rows,
        Err(error) => return emit(&mut io::stderr().lock(), &error, ExitCode::FAILURE),
    };
    report_review(&policy, &rows, json)
}

fn report_review(policy: &Policy, rows: &[domain::Row], json: bool) -> ExitCode {
    let lines = domain::verify_review(policy, rows)
        .iter()
        .map(|gap| format!("{}  {}", gap.rule, gap.message))
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

fn read_rows(file: &Path) -> Result<Vec<domain::Row>, String> {
    let text = fs::read_to_string(file).map_err(|error| format!("{}: {error}", file.display()))?;
    domain::parse_review(&text).map_err(|error| error.to_string())
}

fn policy(dir: &Path) -> Result<Policy, String> {
    let (policy, mechanism) = load(dir)?;
    let gaps = domain::diagnose(&policy, &mechanism);
    if gaps.is_empty() {
        Ok(policy)
    } else {
        Err(gaps
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}
