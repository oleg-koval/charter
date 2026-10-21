use std::io;
use std::process::Command;

#[test]
fn shows_usage_and_exits_2() -> io::Result<()> {
    let out = Command::new(env!("CARGO_BIN_EXE_charter")).output()?;
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("usage:"));
    Ok(())
}
