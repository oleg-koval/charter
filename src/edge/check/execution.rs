use std::io;
use std::process::{Command, Stdio};

use crate::domain::{self, Check, Outcome, Output};

// Output::Exit sends stdout to stderr; only the process status is its proof.
pub(in crate::edge) fn execute(check: &Check) -> Result<Outcome, String> {
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
