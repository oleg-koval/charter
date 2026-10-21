use std::io;
use std::process::{Command, Output};

fn run(args: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_charter"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(args)
        .output()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn check_changed_json_success_keeps_stdout_machine_readable() -> io::Result<()> {
    let dir = format!("{}/tests/fixtures/check-pass", env!("CARGO_MANIFEST_DIR"));
    let out = run(&["check", "--json", "--changed=HEAD", "--dir", &dir])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "{\"ok\":true,\"lines\":[]}\n");
    Ok(())
}

#[test]
fn check_changed_rejects_option_like_base() -> io::Result<()> {
    let dir = format!("{}/tests/fixtures/check-pass", env!("CARGO_MANIFEST_DIR"));
    let out = run(&["check", "--changed=-bad", "--dir", &dir])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    assert!(stderr(&out).contains("invalid --changed base: -bad"));
    Ok(())
}
