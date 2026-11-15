use std::io;
use std::os::unix::process::ExitStatusExt;
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
    let (outcomes, evidence): (Vec<_>, Vec<_>) = observations.into_iter().unzip();
    Ok((outcomes, evidence.into_iter().flatten().collect()))
}

pub(in crate::edge) fn execute(check: &Check) -> Result<Outcome, String> {
    execute_observed(check).map(|(outcome, _)| outcome)
}

fn execute_observed(check: &Check) -> Result<(Outcome, Option<NativeExitEvidence>), String> {
    // Exit checks send stdout to stderr; only their process status is proof.
    let stdout = match check.output {
        Output::Exit => Stdio::from(io::stderr()),
        Output::Sarif => Stdio::piped(),
    };
    let output = Command::new("sh")
        .arg("-c")
        .arg(&check.run)
        .stdout(stdout)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("{}: cannot run: {error}", check.id))?;
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

fn observe_sarif(
    check: &Check,
    output: &std::process::Output,
) -> Result<(Outcome, Option<NativeExitEvidence>), String> {
    let status = output.status.code().map_or_else(
        || NativeExitStatus::Signalled(output.status.signal().map_or(0, |signal| signal)),
        NativeExitStatus::Exited,
    );
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
        .map_err(|error| format!("{}: {error} (process {status:?})", check.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_sarif_keeps_nonzero_and_signalled_process_statuses() {
        let check = |run: &str| Check {
            id: "ast".to_owned(),
            rules: vec![],
            run: run.to_owned(),
            output: Output::Sarif,
            on: None,
        };
        for (command, status) in [
            ("exit 3", NativeExitStatus::Exited(3)),
            ("kill -9 $$", NativeExitStatus::Signalled(9)),
        ] {
            let run = format!("echo '{{\"runs\":[]}}'; {command}");
            assert_eq!(
                execute_observed(&check(&run)).unwrap().1.unwrap().status,
                status
            );
        }
    }
}
