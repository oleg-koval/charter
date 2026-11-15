use std::io;
use std::process::{Command, Stdio};

use crate::domain::{
    self, Check, Mechanism, NativeExitEvidence, NativeExitStatus, Outcome, Output,
};

pub(super) fn observe_checks(
    mechanism: &Mechanism,
    event: Option<&str>,
) -> Result<(Vec<Outcome>, Vec<NativeExitEvidence>), String> {
    let observations = domain::select(mechanism, event)
        .into_iter()
        .map(execute_observed)
        .collect::<Result<Vec<_>, _>>()?;
    let outcomes = observations
        .iter()
        .map(|(outcome, _)| outcome.clone())
        .collect();
    let evidence = observations
        .into_iter()
        .filter_map(|(_, evidence)| evidence)
        .collect();
    Ok((outcomes, evidence))
}

pub(in crate::edge) fn execute(check: &Check) -> Result<Outcome, String> {
    execute_observed(check).map(|(outcome, _)| outcome)
}

fn execute_observed(check: &Check) -> Result<(Outcome, Option<NativeExitEvidence>), String> {
    let output = run_native_check(check)?;
    match check.output {
        Output::Exit => Ok((
            Outcome::Exit {
                check_id: check.id.clone(),
                code: output.status.code(),
            },
            None,
        )),
        Output::Sarif => observe_sarif(check, &output),
    }
}

fn run_native_check(check: &Check) -> Result<std::process::Output, String> {
    let stdout = match check.output {
        Output::Exit => Stdio::from(io::stderr()),
        Output::Sarif => Stdio::piped(),
    };
    Command::new("sh")
        .arg("-c")
        .arg(&check.run)
        .stdout(stdout)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("{}: cannot run: {e}", check.id))
}

fn observe_sarif(
    check: &Check,
    output: &std::process::Output,
) -> Result<(Outcome, Option<NativeExitEvidence>), String> {
    let status = output
        .status
        .code()
        .map_or(NativeExitStatus::Signalled, NativeExitStatus::Exited);
    let evidence = NativeExitEvidence {
        check_id: check.id.clone(),
        status,
    };
    domain::parse_sarif(&String::from_utf8_lossy(&output.stdout))
        .map(|findings| {
            (
                Outcome::Sarif {
                    check_id: check.id.clone(),
                    findings,
                },
                Some(evidence),
            )
        })
        .map_err(|e| format!("{}: {e}", check.id))
}
