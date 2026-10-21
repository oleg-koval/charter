use std::io::{self, Write};
use std::process::{Command, Output, Stdio};

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn hook_ignores_malformed_input_and_reports_matching_findings() -> io::Result<()> {
    let malformed = run_hook("not json", None)?;
    assert!(malformed.status.success());
    assert_eq!(stdout(&malformed), "");
    let fixture_dir = fixture("check-new");
    let found = run_hook(
        r#"{"tool_input":{"file_path":"src/a.rs"}}"#,
        Some(&fixture_dir),
    )?;
    assert!(found.status.success());
    assert_eq!(stdout(&found), "no-mut  src/a.rs:4  mutation in domain\n");
    let absolute = format!("{}/src/a.rs", env!("CARGO_MANIFEST_DIR"));
    let found = run_hook(
        &format!(r#"{{"tool_input":{{"file_path":"{absolute}"}}}}"#),
        Some(&fixture_dir),
    )?;
    assert!(found.status.success());
    assert_eq!(stdout(&found), "no-mut  src/a.rs:4  mutation in domain\n");
    Ok(())
}

fn run_hook(input: &str, dir: Option<&str>) -> io::Result<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_charter"));
    command
        .arg("hook")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = dir {
        command.args(["--dir", dir]);
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("stdin unavailable"))?
        .write_all(input.as_bytes())?;
    child.wait_with_output()
}
