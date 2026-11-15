use serde_json::{Value, json};

use super::{
    Baseline, Check, ExecutionError, Mechanism, Outcome, Output, new_findings, select,
    validate_execution, verdict,
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
    validate_execution(mechanism, event, raw, effective)?;
    let selected = select(mechanism, event);
    let passed = effective
        .iter()
        .filter(|o| verdict(baseline, std::slice::from_ref(o)).is_empty())
        .count();
    Ok(json!({
        "version": 1, "event": event, "scope": scope,
        "ok": verdict(baseline, effective).is_empty(),
        "counts": {"configured": mechanism.check.len(), "selected": selected.len(),
            "completed": raw.len(), "skipped": mechanism.check.len() - selected.len(),
            "pass": passed, "fail": effective.len() - passed},
        "checks": mechanism.check.iter().map(|check| check_report(check, selected.contains(&check), baseline, raw, effective)).collect::<Vec<_>>(),
        "waivers": mechanism.waive.iter().map(|w| json!({"rule":w.rule,"reason":w.reason})).collect::<Vec<_>>(),
        "baseline": {"stored_checks": baseline.checks.len(),
            "stored_fingerprints": baseline.checks.values().map(std::collections::BTreeSet::len).sum::<usize>()},
        "fixture_evidence":"unknown", "enforcement_evidence":"unknown",
        "review_evidence":"not_assessed", "artifact_evidence":"not_assessed",
        "sarif_native_exit":"unavailable"
    }))
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
) -> Value {
    let result = selected.then(|| outcome(effective, &check.id)).flatten();
    let state = result.map_or("skipped", |o| {
        if verdict(baseline, std::slice::from_ref(o)).is_empty() {
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
        "result":result.map(|o| outcome_report(baseline, outcome(raw, &check.id), o))})
}

fn outcome_report(baseline: &Baseline, raw: Option<&Outcome>, effective: &Outcome) -> Value {
    match effective {
        Outcome::Exit { code, .. } => json!({"exit_code":code}),
        Outcome::Sarif { check_id, findings } => {
            let raw_count = match raw {
                Some(Outcome::Sarif { findings, .. }) => findings.len(),
                _ => 0,
            };
            let new = new_findings(baseline, check_id, findings).len();
            json!({"raw":raw_count,"effective":findings.len(),"new":new,
                "baselined":findings.len() - new,"filtered":raw_count - findings.len(),
                "native_exit":"unavailable"})
        }
    }
}

#[cfg(test)]
mod tests;
