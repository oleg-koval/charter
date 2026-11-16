//! The only place with side effects: files, stdout, stderr, exit codes.

mod agents;
mod baseline;
mod check;
mod git_paths;
mod hook;
mod init;
mod pr;
mod review;

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use crate::domain::{self, Baseline, Mechanism, Policy};

pub use agents::{agents_install, agents_uninstall};
pub use baseline::{baseline, baseline_with_options};
pub use check::{check, check_with_report, check_with_report_v2, check_working_tree};
pub use hook::hook;
pub use init::{init, init_with_checks, init_with_detection, init_with_options};
pub use pr::verify_pr;
pub use review::{rubric, verify_review};

#[must_use]
pub fn doctor(dir: &Path, json: bool) -> ExitCode {
    doctor_with_explanation(dir, json, false)
}

#[must_use]
pub fn doctor_with_explanation(dir: &Path, json: bool, explain: bool) -> ExitCode {
    run_doctor(
        dir,
        json,
        explain,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    )
}

fn run_doctor(
    dir: &Path,
    json: bool,
    explain: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> ExitCode {
    match load(dir) {
        Ok((policy, mechanism)) => report(&policy, &mechanism, json, explain, out),
        Err(message) => emit(err, &message, ExitCode::FAILURE),
    }
}

fn report(
    policy: &Policy,
    mechanism: &Mechanism,
    json: bool,
    explain: bool,
    out: &mut impl Write,
) -> ExitCode {
    let gaps = domain::diagnose(policy, mechanism);
    let code = if gaps.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    let text = match (json, gaps.is_empty()) {
        (true, _) => domain::render_json(&gaps, &mechanism.waive),
        (false, true) => domain::summary(policy, mechanism),
        (false, false) => {
            let lines = gaps.iter().map(ToString::to_string).collect::<Vec<_>>();
            lines.join("\n") + &domain::waived_lines(&mechanism.waive)
        }
    };
    let text = if explain && !json {
        text + "\n" + &domain::explain_configuration(mechanism)
    } else {
        text
    };
    emit(out, &text, code)
}

fn load(dir: &Path) -> Result<(Policy, Mechanism), String> {
    let policy = read(&dir.join("rules.toml"))
        .and_then(|(path, text)| domain::parse_policy(&text).map_err(|e| format!("{path}: {e}")))?;
    let mechanism = read(&dir.join("checks.toml")).and_then(|(path, text)| {
        domain::parse_mechanism(&text).map_err(|e| format!("{path}: {e}"))
    })?;
    Ok((policy, mechanism))
}

// A missing baseline.json means nothing is accepted yet; an unparsable one is an error.
fn load_baseline(dir: &Path) -> Result<Baseline, String> {
    let path = dir.join("baseline.json");
    match fs::read_to_string(&path) {
        Ok(text) => domain::parse_baseline(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Baseline::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn read(path: &Path) -> Result<(String, String), String> {
    let shown = path.display().to_string();
    fs::read_to_string(path)
        .map(|text| (shown.clone(), text))
        .map_err(|e| format!("{shown}: {e}"))
}

// A closed pipe (`charter doctor | head -0`) must not panic the way `println!` would.
fn emit(out: &mut impl Write, text: &str, code: ExitCode) -> ExitCode {
    match writeln!(out, "{text}") {
        Ok(()) => code,
        Err(_) => ExitCode::FAILURE,
    }
}
