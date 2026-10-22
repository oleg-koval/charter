use std::io;
use std::process::Command;
#[test]
fn doctor_prints_each_waiver_and_exits_zero() -> io::Result<()> {
    let dir = format!("{}/tests/fixtures/waived", env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_charter"))
        .args(["doctor", "--dir", &dir])
        .output()?;
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "ok  1 rules, 0 checks, 0 artifacts\nwaived  C2  no ast-grep rule for this language yet\n"
    );
    Ok(())
}
