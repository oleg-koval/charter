use serde_json::{Value, json};

use super::{
    Baseline, Check, ExecutionError, Mechanism, NativeExitEvidence, NativeExitStatus, Outcome,
    Output, new_findings, select, validate_execution, validate_native_exit_evidence, verdict,
};

/// Projects a completed run; verdicts describe commands, not rule effectiveness.
/// # Errors
/// Rejects inconsistent or incomplete execution observations.
pub fn execution_report(
    mechanism: &Mechanism,
    baseline: &Baseline,
    raw: &[Outcome],
    effective: &[Outcome],
    event: Option<&str>,
    scope: &str,
) -> Result<Value, ExecutionError> {
    execution_report_inner(mechanism, baseline, raw, effective, event, scope, None)
}

/// Projects a completed run including observed native SARIF command statuses.
/// # Errors
/// Rejects inconsistent execution observations or missing native status evidence.
pub fn execution_report_v2(
    mechanism: &Mechanism,
    baseline: &Baseline,
    raw: &[Outcome],
    effective: &[Outcome],
    native_exit: &[NativeExitEvidence],
    event: Option<&str>,
    scope: &str,
) -> Result<Value, ExecutionError> {
    execution_report_inner(
        mechanism,
        baseline,
        raw,
        effective,
        event,
        scope,
        Some(native_exit),
    )
}

fn execution_report_inner(
    mechanism: &Mechanism,
    baseline: &Baseline,
    raw: &[Outcome],
    effective: &[Outcome],
    event: Option<&str>,
    scope: &str,
    native_exit: Option<&[NativeExitEvidence]>,
) -> Result<Value, ExecutionError> {
    validate_report_inputs(mechanism, event, raw, effective, native_exit)?;
    let selected = select(mechanism, event);
    let passed = effective
        .iter()
        .filter(|o| {
            verdict(baseline, std::slice::from_ref(o)).is_empty()
                && native_succeeded(native_exit, raw, o)
        })
        .count();
    Ok(json!({
        "version": if native_exit.is_some() { 2 } else { 1 }, "event": event, "scope": scope,
        "ok": verdict(baseline, effective).is_empty()
            && effective.iter().all(|o| native_succeeded(native_exit, raw, o)),
        "counts": {"configured": mechanism.check.len(), "selected": selected.len(),
            "completed": raw.len(), "skipped": mechanism.check.len() - selected.len(),
            "pass": passed, "fail": effective.len() - passed},
        "checks": mechanism.check.iter().map(|check| check_report(check, selected.contains(&check), baseline, raw, effective, native_exit)).collect::<Vec<_>>(),
        "waivers": mechanism.waive.iter().map(|w| json!({"rule":w.rule,"reason":w.reason})).collect::<Vec<_>>(),
        "baseline": {"stored_checks": baseline.checks.len(),
            "stored_fingerprints": baseline.checks.values().map(std::collections::BTreeSet::len).sum::<usize>()},
        "fixture_evidence":"unknown", "enforcement_evidence":"unknown",
        "review_evidence":"not_assessed", "artifact_evidence":"not_assessed",
        "sarif_native_exit": if native_exit.is_some() { "observed" } else { "unavailable" }
    }))
}

fn validate_report_inputs(
    mechanism: &Mechanism,
    event: Option<&str>,
    raw: &[Outcome],
    effective: &[Outcome],
    native_exit: Option<&[NativeExitEvidence]>,
) -> Result<(), ExecutionError> {
    validate_execution(mechanism, event, raw, effective)?;
    if let Some(native_exit) = native_exit {
        validate_native_exit_evidence(mechanism, event, native_exit)?;
    }
    Ok(())
}

fn native_succeeded(
    native_exit: Option<&[NativeExitEvidence]>,
    raw: &[Outcome],
    effective: &Outcome,
) -> bool {
    let (Some(native_exit), Outcome::Sarif { check_id, .. }) = (native_exit, effective) else {
        return true;
    };
    native_exit
        .iter()
        .find(|item| item.check_id == *check_id)
        .is_some_and(|item| match item.status {
            NativeExitStatus::Exited(0) => true,
            NativeExitStatus::Exited(_) => {
                matches!(outcome(raw, check_id), Some(Outcome::Sarif { findings, .. }) if !findings.is_empty())
            }
            NativeExitStatus::Signalled(_) => false,
        })
}

fn outcome<'a>(outcomes: &'a [Outcome], id: &str) -> Option<&'a Outcome> {
    outcomes.iter().find(|o| match o {
        Outcome::Exit { check_id, .. } | Outcome::Sarif { check_id, .. } => check_id == id,
    })
}

fn check_report(
    check: &Check,
    selected: bool,
    baseline: &Baseline,
    raw: &[Outcome],
    effective: &[Outcome],
    native_exit: Option<&[NativeExitEvidence]>,
) -> Value {
    let result = selected.then(|| outcome(effective, &check.id)).flatten();
    let state = result.map_or("skipped", |o| {
        if verdict(baseline, std::slice::from_ref(o)).is_empty()
            && native_succeeded(native_exit, raw, o)
        {
            "pass"
        } else {
            "fail"
        }
    });
    let output = match check.output {
        Output::Exit => "exit",
        Output::Sarif => "sarif",
    };
    json!({"id":check.id,"rules":check.rules,"output":output,"event":check.on,"state":state,
        "result":result.map(|o| outcome_report(baseline, outcome(raw, &check.id), o, native_exit))})
}

fn outcome_report(
    baseline: &Baseline,
    raw: Option<&Outcome>,
    effective: &Outcome,
    native_exit: Option<&[NativeExitEvidence]>,
) -> Value {
    match effective {
        Outcome::Exit { code, .. } => json!({"exit_code":code}),
        Outcome::Sarif { check_id, findings } => {
            let raw_count = match raw {
                Some(Outcome::Sarif { findings, .. }) => findings.len(),
                _ => 0,
            };
            let new = new_findings(baseline, check_id, findings).len();
            let status =
                native_exit.and_then(|items| items.iter().find(|item| item.check_id == *check_id));
            let native = status.map(|item| match item.status {
                NativeExitStatus::Exited(code) => json!({"kind":"exited","code":code}),
                NativeExitStatus::Signalled(signal) => {
                    json!({"kind":"signalled","signal":signal})
                }
            });
            json!({"raw":raw_count,"effective":findings.len(),"new":new,
                "baselined":findings.len() - new,"filtered":raw_count - findings.len(),
                "native_exit":native.unwrap_or_else(|| json!("unavailable"))})
        }
    }
}

#[cfg(test)]
mod tests;
