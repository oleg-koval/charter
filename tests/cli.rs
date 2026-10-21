use std::io;
use std::process::{Command, Output};

fn charter(args: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_charter"))
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
fn greets_by_name() -> io::Result<()> {
    let out = charter(&["Ada", "Lovelace"])?;
    assert!(out.status.success());
    assert_eq!(stdout(&out), "Hello, Ada Lovelace!\n");
    Ok(())
}

#[test]
fn prints_the_package_version() -> io::Result<()> {
    let out = charter(&["version"])?;
    assert!(out.status.success());
    assert_eq!(
        stdout(&out),
        format!("charter {}\n", env!("CARGO_PKG_VERSION"))
    );
    Ok(())
}

#[test]
fn rejects_a_blank_name_on_stderr() -> io::Result<()> {
    let out = charter(&["   "])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stderr(&out), "name must not be empty\n");
    Ok(())
}

#[test]
fn shows_usage_without_arguments() -> io::Result<()> {
    let out = charter(&[])?;
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).starts_with("usage:"));
    Ok(())
}
