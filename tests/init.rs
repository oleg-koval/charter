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
fn starter_requires_real_checks_before_reporting_success() -> io::Result<()> {
    let temp = TempDir::create()?;
    let dir = temp.0.as_path();
    let path = dir.display().to_string();
    assert!(charter(&["init", "--dir", &path])?.status.success());
    let unconfigured = charter(&["check", "--dir", &path])?;
    assert_eq!(unconfigured.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&unconfigured.stderr)
            .contains("Configure real checks in checks.toml")
    );
    verify_configured_check(dir, &path)?;
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

fn configure_content_check(dir: &std::path::Path) -> io::Result<()> {
    let checks = std::fs::read_to_string(dir.join("checks.toml"))?;
    let configured = checks
        .lines()
        .map(|line| {
            if line.starts_with("run = ") {
                r#"run = 'test "$(cat sample.txt)" = compliant'"#
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(dir.join("checks.toml"), configured)?;
    Ok(())
}

fn verify_configured_check(dir: &std::path::Path, path: &str) -> io::Result<()> {
    configure_content_check(dir)?;
    std::fs::write(dir.join("sample.txt"), "compliant")?;
    let passing = Command::new(env!("CARGO_BIN_EXE_charter"))
        .current_dir(dir)
        .args(["check", "--dir", path])
        .output()?;
    assert!(passing.status.success());
    std::fs::write(dir.join("sample.txt"), "violation")?;
    let failing = Command::new(env!("CARGO_BIN_EXE_charter"))
        .current_dir(dir)
        .args(["check", "--dir", path])
        .output()?;
    assert_eq!(failing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failing.stdout).contains("starter  exited 1"));
    Ok(())
}

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn create() -> io::Result<Self> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "charter-init-enforcement-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!("could not remove {}: {error}", self.0.display());
        }
    }
}
