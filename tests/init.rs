use std::io;
use std::process::{Command, Output};

fn charter(args: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_charter"))
        .args(args)
        .output()
}

#[test]
fn init_writes_a_doctor_valid_starter_and_refuses_overwrite() -> io::Result<()> {
    let dir = std::env::temp_dir().join(format!("charter-init-{}", std::process::id()));
    let path = dir.display().to_string();
    let initialized = charter(&["init", "--dir", &path])?;
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    let doctor = charter(&["doctor", "--dir", &path])?;
    assert!(
        doctor.status.success(),
        "{}",
        String::from_utf8_lossy(&doctor.stdout)
    );
    let rules = std::fs::read_to_string(dir.join("rules.toml"))?;
    let repeated = charter(&["init", "--dir", &path])?;
    assert_eq!(repeated.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&repeated.stderr).contains("rules.toml already exists"));
    assert_eq!(rules, std::fs::read_to_string(dir.join("rules.toml"))?);
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn init_removes_rules_when_checks_already_exists() -> io::Result<()> {
    let dir = std::env::temp_dir().join(format!("charter-init-existing-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("checks.toml"), "existing")?;
    let path = dir.display().to_string();
    let initialized = charter(&["init", "--dir", &path])?;
    assert_eq!(initialized.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&initialized.stderr).contains("checks.toml"));
    assert!(!dir.join("rules.toml").exists());
    std::fs::remove_dir_all(dir)?;
    Ok(())
}
