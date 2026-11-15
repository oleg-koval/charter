use crate::domain::{GitPathsError, invalid_path_event};
use std::process::{Command, ExitCode};

pub(super) fn paths(root: &str, args: &[&str]) -> Result<Vec<String>, GitPathsError> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| GitPathsError::Spawn(format!("git {}: {error}", args.join(" "))))?;
    if !output.status.success() {
        return Err(GitPathsError::Exit(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| invalid_utf8())
        .map(|paths| {
            paths
                .split('\0')
                .filter(|path| !path.is_empty())
                .map(str::to_owned)
                .collect()
        })
}

fn invalid_utf8() -> GitPathsError {
    super::emit(
        &mut std::io::stderr().lock(),
        &invalid_path_event(),
        ExitCode::FAILURE,
    );
    GitPathsError::InvalidUtf8
}
