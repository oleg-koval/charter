use std::io::Write;
use std::process::ExitCode;

use super::{Execution, ReportMode, Scope};
use crate::domain::{self, Mechanism};
use crate::edge::emit;

pub(super) fn report_gaps(
    policy: &domain::Policy,
    mechanism: &Mechanism,
    json: bool,
    report: ReportMode,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Option<ExitCode> {
    let gaps = domain::diagnose(policy, mechanism);
    if gaps.is_empty() {
        return None;
    }
    let text = if json {
        domain::render_json(&gaps, &mechanism.waive)
    } else {
        gaps.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    };
    Some(if matches!(report, ReportMode::None) {
        emit(out, &text, ExitCode::FAILURE)
    } else {
        emit(err, &text, ExitCode::FAILURE)
    })
}

pub(super) fn print_report(
    execution: &Execution,
    mechanism: &Mechanism,
    event: Option<&str>,
    changed: Option<Scope<'_>>,
    out: &mut impl Write,
    err: &mut impl Write,
) -> ExitCode {
    let scope = scope_name(changed);
    let code = if execution.lines().is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    match domain::execution_report(
        mechanism,
        &execution.baseline,
        &execution.raw,
        execution.effective(),
        event,
        &scope,
    ) {
        Ok(report) => emit(out, &report.to_string(), code),
        Err(error) => emit(err, &error.to_string(), ExitCode::FAILURE),
    }
}

pub(super) fn print_report_v2(
    execution: &Execution,
    mechanism: &Mechanism,
    event: Option<&str>,
    changed: Option<Scope<'_>>,
    out: &mut impl Write,
    err: &mut impl Write,
) -> ExitCode {
    match domain::execution_report_v2(
        mechanism,
        &execution.baseline,
        &execution.raw,
        execution.effective(),
        &execution.native_exit,
        event,
        &scope_name(changed),
    ) {
        Ok(report) => {
            let code = if report["ok"].as_bool() == Some(true) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
            emit(out, &report.to_string(), code)
        }
        Err(error) => emit(err, &error.to_string(), ExitCode::FAILURE),
    }
}

fn scope_name(changed: Option<Scope<'_>>) -> String {
    match changed {
        None => "all".to_owned(),
        Some(Scope::WorkingTree) => "working_tree".to_owned(),
        Some(Scope::Committed(base)) => format!("changed:{base}"),
    }
}
