use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use super::check::execute;
use super::{emit, load_baseline, read};
use crate::domain::{self, Baseline, Outcome, Output};

#[must_use]
pub fn baseline(dir: &Path) -> ExitCode {
    match write_baseline(dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => emit(&mut io::stderr().lock(), &message, ExitCode::FAILURE),
    }
}

fn write_baseline(dir: &Path) -> Result<(), String> {
    let (_, text) = read(&dir.join("checks.toml"))?;
    let mechanism = domain::parse_mechanism(&text).map_err(|e| format!("checks.toml: {e}"))?;
    let outcomes = mechanism
        .check
        .iter()
        .filter(|c| c.output == Output::Sarif)
        .map(execute)
        .collect::<Result<Vec<_>, _>>()?;
    let recorded = outcomes
        .into_iter()
        .fold(load_baseline(dir)?, |acc, outcome| accept(acc, &outcome));
    let path = dir.join("baseline.json");
    let json = domain::render_baseline(&recorded).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("{}: {e}", path.display()))
}

fn accept(baseline: Baseline, outcome: &Outcome) -> Baseline {
    match outcome {
        Outcome::Sarif { check_id, findings } => domain::record(baseline, check_id, findings),
        Outcome::Exit { .. } => baseline,
    }
}
