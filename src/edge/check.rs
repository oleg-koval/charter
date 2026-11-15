use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitCode};

use super::{emit, load, load_baseline};
use crate::domain::{self, Mechanism, NativeExitEvidence, Outcome};

mod execution;
mod report;

pub(in crate::edge) use execution::execute;
use execution::observe_checks;
use report::{print_report, print_report_v2, report_gaps};

#[derive(Clone, Copy)]
enum Scope<'a> {
    Committed(&'a str),
    WorkingTree,
}

#[derive(Clone, Copy)]
enum ReportMode {
    None,
    Legacy,
    NativeExit,
}

#[must_use]
pub fn check(dir: &Path, event: Option<&str>, json: bool, changed: Option<&str>) -> ExitCode {
    run_scoped(dir, event, json, false, changed.map(Scope::Committed))
}

#[must_use]
pub fn check_working_tree(dir: &Path, event: Option<&str>, json: bool) -> ExitCode {
    run_scoped(dir, event, json, false, Some(Scope::WorkingTree))
}

#[must_use]
pub fn check_with_report(
    dir: &Path,
    event: Option<&str>,
    changed: Option<&str>,
    working_tree: bool,
) -> ExitCode {
    let scope = if working_tree {
        Some(Scope::WorkingTree)
    } else {
        changed.map(Scope::Committed)
    };
    run_scoped(dir, event, false, true, scope)
}

#[must_use]
pub fn check_with_report_v2(
    dir: &Path,
    event: Option<&str>,
    changed: Option<&str>,
    working_tree: bool,
) -> ExitCode {
    let scope = if working_tree {
        Some(Scope::WorkingTree)
    } else {
        changed.map(Scope::Committed)
    };
    run_scoped_v2(dir, event, scope)
}

fn run_scoped(
    dir: &Path,
    event: Option<&str>,
    json: bool,
    report: bool,
    changed: Option<Scope<'_>>,
) -> ExitCode {
    run_check(
        dir,
        event,
        json,
        if report {
            ReportMode::Legacy
        } else {
            ReportMode::None
        },
        changed,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    )
}

fn run_scoped_v2(dir: &Path, event: Option<&str>, changed: Option<Scope<'_>>) -> ExitCode {
    run_check(
        dir,
        event,
        false,
        ReportMode::NativeExit,
        changed,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    )
}

fn run_check(
    dir: &Path,
    event: Option<&str>,
    json: bool,
    report: ReportMode,
    changed: Option<Scope<'_>>,
    out: &mut impl Write,
    err: &mut impl Write,
) -> ExitCode {
    let (policy, mechanism) = match load(dir) {
        Ok(loaded) => loaded,
        Err(message) => return emit(err, &message, ExitCode::FAILURE),
    };
    if let Some(code) = report_gaps(&policy, &mechanism, json, report, out, err) {
        return code;
    }
    match collect_execution(dir, &mechanism, event, changed) {
        Ok(execution) if matches!(report, ReportMode::NativeExit) => {
            print_report_v2(&execution, &mechanism, event, changed, out, err)
        }
        Ok(execution) if matches!(report, ReportMode::Legacy) => {
            print_report(&execution, &mechanism, event, changed, out, err)
        }
        Ok(execution) => print_verdict(&execution.lines(), json, out),
        Err(message) => emit(err, &message, ExitCode::FAILURE),
    }
}

struct Execution {
    baseline: domain::Baseline,
    raw: Vec<Outcome>,
    filtered: Option<Vec<Outcome>>,
    native_exit: Vec<NativeExitEvidence>,
}

impl Execution {
    fn effective(&self) -> &[Outcome] {
        self.filtered.as_deref().unwrap_or(&self.raw)
    }

    fn lines(&self) -> Vec<String> {
        domain::verdict(&self.baseline, self.effective())
            .iter()
            .map(ToString::to_string)
            .collect()
    }
}

fn collect_execution(
    dir: &Path,
    mechanism: &Mechanism,
    event: Option<&str>,
    changed: Option<Scope<'_>>,
) -> Result<Execution, String> {
    let baseline = load_baseline(dir)?;
    let working = matches!(changed, Some(Scope::WorkingTree))
        .then(working_selection)
        .transpose()
        .inspect_err(log_working_failure)?;
    let (outcomes, native_exit) = observe_checks(mechanism, event)?;
    let filtered = match changed {
        Some(Scope::Committed(base)) => {
            let root = repository_root()?;
            Some(domain::changed_outcomes(
                &outcomes,
                &changed_files(base, &root)?,
                &root,
            ))
        }
        _ => working.map(|(files, root)| domain::changed_outcomes(&outcomes, &files, &root)),
    };
    Ok(Execution {
        baseline,
        raw: outcomes,
        filtered,
        native_exit,
    })
}

fn log_working_failure(message: &String) {
    let event = serde_json::json!({"event":"charter.working_selection.failed", "metric":"charter_working_selection_failures", "count":1, "message":message});
    emit(
        &mut io::stderr().lock(),
        &event.to_string(),
        ExitCode::FAILURE,
    );
}

fn working_selection() -> Result<(Vec<String>, String), String> {
    let root = repository_root()?;
    let records = super::git_paths::paths(
        ".",
        &[
            "-c",
            "core.fsmonitor=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--no-renames",
            "--ignore-submodules=none",
        ],
    )
    .map_err(|error| error.to_string())?;
    let files = domain::parse_working_status(&records).map_err(|error| {
        emit(
            &mut io::stderr().lock(),
            &domain::working_status_failure(&error),
            ExitCode::FAILURE,
        );
        error.to_string()
    })?;
    Ok((files, root))
}

fn changed_files(base: &str, root: &str) -> Result<Vec<String>, String> {
    if base.starts_with('-') {
        return Err(format!("invalid --changed base: {base}"));
    }
    let range = format!("{base}...HEAD");
    super::git_paths::paths(root, &["diff", "--name-only", &range, "-z"])
        .map_err(|error| error.to_string())
}

fn repository_root() -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| format!("git rev-parse --show-toplevel: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse --show-toplevel: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
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
        (false, false) => {
            let text = lines
                .iter()
                .map(|line| domain::terminal_text(line))
                .collect::<Vec<_>>()
                .join("\n");
            emit(out, &text, code)
        }
    }
}

#[cfg(test)]
mod tests;
