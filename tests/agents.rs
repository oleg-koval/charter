#[path = "support/temp_dir.rs"]
mod temp_dir;

use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

use temp_dir::TempDir;

fn install(root: &Path, extra: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_charter"))
        .args(["agents", "install", "--root"])
        .arg(root)
        .args(extra)
        .output()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn install_writes_once_and_rerun_is_a_no_op() -> io::Result<()> {
    let temp = TempDir::create()?;
    let root = temp.0.as_path();
    fs::write(root.join("AGENTS.md"), "# Own rules\n")?;
    let preview = install(root, &["--dry-run"])?;
    assert!(preview.status.success());
    assert_eq!(
        stdout(&preview),
        "would update  AGENTS.md\nwould create  CLAUDE.md\n"
    );
    assert!(!root.join("CLAUDE.md").exists());
    let first = install(root, &[])?;
    assert!(first.status.success());
    assert_eq!(stdout(&first), "update  AGENTS.md\ncreate  CLAUDE.md\n");
    let agents = fs::read_to_string(root.join("AGENTS.md"))?;
    assert!(agents.starts_with("# Own rules\n\n<!-- charter:begin -->\n## Charter"));
    assert_eq!(fs::read_to_string(root.join("CLAUDE.md"))?, "@AGENTS.md\n");
    let again = install(root, &[])?;
    assert_eq!(
        stdout(&again),
        "unchanged  AGENTS.md\nunchanged  CLAUDE.md\n"
    );
    assert_eq!(fs::read_to_string(root.join("AGENTS.md"))?, agents);
    Ok(())
}

#[test]
fn install_refuses_broken_markers_and_writes_nothing() -> io::Result<()> {
    let temp = TempDir::create()?;
    let root = temp.0.as_path();
    fs::write(
        root.join("AGENTS.md"),
        "<!-- charter:begin -->\nhalf a block\n",
    )?;
    let refused = install(root, &[])?;
    assert_eq!(refused.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("AGENTS.md: `<!-- charter:begin -->`")
    );
    assert!(!root.join("CLAUDE.md").exists());
    Ok(())
}
