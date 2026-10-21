use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use super::{emit, load, load_baseline};
use crate::domain::{self, Check, Mechanism, Outcome, Output};

#[must_use]
pub fn check(dir: &Path, event: Option<&str>, json: bool) -> ExitCode {
    run_check(
        dir,
        event,
        json,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    )
}

fn run_check(
    dir: &Path,
    event: Option<&str>,
    json: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> ExitCode {
    let (policy, mechanism) = match load(dir) {
        Ok(loaded) => loaded,
        Err(message) => return emit(err, &message, ExitCode::FAILURE),
    };
    let gaps = domain::diagnose(&policy, &mechanism);
    if !gaps.is_empty() {
        let text = gaps.iter().map(ToString::to_string).collect::<Vec<_>>();
        return emit(out, &text.join("\n"), ExitCode::FAILURE);
    }
    match verdict(dir, &mechanism, event) {
        Ok(lines) => print_verdict(&lines, json, out),
        Err(message) => emit(err, &message, ExitCode::FAILURE),
    }
}

fn verdict(dir: &Path, mechanism: &Mechanism, event: Option<&str>) -> Result<Vec<String>, String> {
    let baseline = load_baseline(dir)?;
    let outcomes = domain::select(mechanism, event)
        .into_iter()
        .map(execute)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(domain::verdict(&baseline, &outcomes)
        .iter()
        .map(ToString::to_string)
        .collect())
}

fn print_verdict(lines: &[String], json: bool, out: &mut impl Write) -> ExitCode {
    let code = if lines.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    match (json, lines.is_empty()) {
        (true, _) => emit(out, &domain::render_verdict_json(lines), code),
        (false, true) => code,
        (false, false) => emit(out, &lines.join("\n"), code),
    }
}

// Output::Exit discards stdout: only the exit code is the proof.
pub(super) fn execute(check: &Check) -> Result<Outcome, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(&check.run)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("{}: cannot run: {e}", check.id))?;
    let check_id = check.id.clone();
    match check.output {
        Output::Exit => Ok(Outcome::Exit {
            check_id,
            code: output.status.code(),
        }),
        Output::Sarif => domain::parse_sarif(&String::from_utf8_lossy(&output.stdout))
            .map(|findings| Outcome::Sarif { check_id, findings })
            .map_err(|e| format!("{}: {e}", check.id)),
    }
}
