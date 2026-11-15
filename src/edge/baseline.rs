use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use super::check::execute;
use super::{emit, load_baseline, read};
use crate::domain::{self, Baseline, Outcome};

#[must_use]
pub fn baseline(dir: &Path) -> ExitCode {
    baseline_with_options(dir, None, false)
}

#[must_use]
pub fn baseline_with_options(dir: &Path, selected: Option<&str>, preview: bool) -> ExitCode {
    match propose_baseline(dir, selected) {
        Ok(json) if preview => emit(&mut io::stdout().lock(), &json, ExitCode::SUCCESS),
        Ok(json) => match fs::write(dir.join("baseline.json"), json) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => emit(
                &mut io::stderr().lock(),
                &format!("{}: {error}", dir.join("baseline.json").display()),
                ExitCode::FAILURE,
            ),
        },
        Err(message) => emit(&mut io::stderr().lock(), &message, ExitCode::FAILURE),
    }
}

fn propose_baseline(dir: &Path, selected: Option<&str>) -> Result<String, String> {
    let (_, text) = read(&dir.join("checks.toml"))?;
    let mechanism = domain::parse_mechanism(&text).map_err(|e| format!("checks.toml: {e}"))?;
    let baseline = load_baseline(dir)?;
    let outcomes = domain::baseline_checks(&mechanism, selected)
        .map_err(|error| {
            emit(
                &mut io::stderr().lock(),
                &domain::baseline_selection_failure(&error),
                ExitCode::FAILURE,
            );
            error.to_string()
        })?
        .into_iter()
        .map(execute)
        .collect::<Result<Vec<_>, _>>()?;
    let recorded = outcomes
        .into_iter()
        .fold(baseline, |acc, outcome| accept(acc, &outcome));
    domain::render_baseline(&recorded).map_err(|e| e.to_string())
}

fn accept(baseline: Baseline, outcome: &Outcome) -> Baseline {
    match outcome {
        Outcome::Sarif { check_id, findings } => domain::record(baseline, check_id, findings),
        Outcome::Exit { .. } => baseline,
    }
}
