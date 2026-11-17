#[path = "support/temp_dir.rs"]
mod temp_dir;

use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

use temp_dir::TempDir;

fn install(root: &Path, extra: &[&str]) -> io::Result<Output> {
    agents("install", root, extra)
}

fn agents(action: &str, root: &Path, extra: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_charter"))
        .args(["agents", action, "--root"])
        .arg(root)
        .args(extra)
        .output()
}

const FILES: [&str; 4] = [
    "AGENTS.md",
    "CLAUDE.md",
    ".claude/settings.json",
    ".codex/hooks.json",
];

fn report(prefix: &str, verbs: &str) -> String {
    let lines: Vec<String> = verbs
        .split(' ')
        .zip(FILES)
        .map(|(verb, file)| format!("{prefix}{verb}  {file}\n"))
        .collect();
    lines.concat()
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
        report("would ", "update create create create")
    );
    assert!(!root.join("CLAUDE.md").exists());
    let first = install(root, &[])?;
    assert!(first.status.success());
    assert_eq!(stdout(&first), report("", "update create create create"));
    let agents = fs::read_to_string(root.join("AGENTS.md"))?;
    assert!(agents.starts_with("# Own rules\n\n<!-- charter:begin -->\n## Charter"));
    assert!(fs::read_to_string(root.join("CLAUDE.md"))?.contains("\n@AGENTS.md\n"));
    let again = install(root, &[])?;
    assert_eq!(
        stdout(&again),
        report("", "unchanged unchanged unchanged unchanged")
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

#[test]
fn uninstall_restores_the_repo_and_is_previewable() -> io::Result<()> {
    let temp = TempDir::create()?;
    let root = temp.0.as_path();
    fs::write(root.join("AGENTS.md"), "# Own rules\n")?;
    assert!(install(root, &[])?.status.success());
    let preview = agents("uninstall", root, &["--dry-run"])?;
    assert_eq!(
        stdout(&preview),
        report("would ", "update delete delete delete")
    );
    assert!(root.join("CLAUDE.md").exists());
    let removed = agents("uninstall", root, &[])?;
    assert!(removed.status.success());
    assert_eq!(stdout(&removed), report("", "update delete delete delete"));
    assert_eq!(fs::read_to_string(root.join("AGENTS.md"))?, "# Own rules\n");
    assert!(!root.join("CLAUDE.md").exists());
    assert!(!root.join(".codex/hooks.json").exists());
    let again = agents("uninstall", root, &[])?;
    assert_eq!(
        stdout(&again),
        report("", "unchanged unchanged unchanged unchanged")
    );
    Ok(())
}

#[test]
fn install_adds_the_hook_with_each_harness_command() -> io::Result<()> {
    let temp = TempDir::create()?;
    let root = temp.0.as_path();
    assert!(install(root, &[])?.status.success());
    let claude = fs::read_to_string(root.join(".claude/settings.json"))?;
    assert!(claude.contains(r#"cd \"$CLAUDE_PROJECT_DIR\" && charter hook"#));
    let codex = fs::read_to_string(root.join(".codex/hooks.json"))?;
    assert!(codex.contains(r#"cd \"$(git rev-parse --show-toplevel)\" && charter hook"#));
    Ok(())
}

#[test]
fn this_repos_dogfood_hooks_count_as_charters_own() -> io::Result<()> {
    let preview = install(Path::new(env!("CARGO_MANIFEST_DIR")), &["--dry-run"])?;
    let out = stdout(&preview);
    assert!(out.contains("unchanged  .claude/settings.json\n"), "{out}");
    assert!(out.contains("unchanged  .codex/hooks.json\n"), "{out}");
    Ok(())
}
