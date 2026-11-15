#[path = "support/temp_dir.rs"]
mod temp_dir;

use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

fn import(root: &Path) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_charter"))
        .current_dir(root)
        .args(["init", "--checks-from", "recipe.toml", "--dir", "output"])
        .output()
}

fn recipe() -> String {
    include_str!("../.charter/checks.toml").replace("run = \"", "run = \"touch executed; ")
}

#[test]
fn imports_exact_bytes_without_running_commands_and_protects_existing_files() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    let root = temp.0.as_path();
    let source = format!("# preserved comment\n{}", recipe());
    fs::write(root.join("recipe.toml"), &source)?;
    let output = import(root)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(root.join("output/checks.toml"))?, source);
    assert!(!root.join("executed").exists());
    let doctor = Command::new(env!("CARGO_BIN_EXE_charter"))
        .current_dir(root)
        .args(["doctor", "--dir", "output"])
        .output()?;
    assert!(doctor.status.success());
    let rules = fs::read(root.join("output/rules.toml"))?;
    assert_eq!(import(root)?.status.code(), Some(1));
    assert_eq!(fs::read(root.join("output/rules.toml"))?, rules);
    assert_eq!(fs::read_to_string(root.join("output/checks.toml"))?, source);
    Ok(())
}

#[test]
fn invalid_recipes_leave_no_destination() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    let root = temp.0.as_path();
    for source in ["[[check", "", "[[waive]]\nrule = 'C1'\nreason = ''"] {
        fs::write(root.join("recipe.toml"), source)?;
        let output = import(root)?;
        assert_eq!(output.status.code(), Some(1));
        assert!(!output.stderr.is_empty());
        assert!(!root.join("output").exists());
    }
    fs::remove_file(root.join("recipe.toml"))?;
    assert_eq!(import(root)?.status.code(), Some(1));
    assert!(!root.join("output").exists());
    Ok(())
}
