#[path = "support/mod.rs"]
mod command;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use std::io;

fn run(args: &[&str]) -> io::Result<std::process::Output> {
    command::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .args(args)
        .output()
}

fn fixture() -> io::Result<temp_dir::TempDir> {
    let temp = temp_dir::TempDir::create()?;
    let root = &temp.0;
    std::fs::write(
        root.join("rules.toml"),
        "version=1\n[[rule]]\nid='C1'\nsection='Code'\ntext='rule'\nenforce=['check']",
    )?;
    let marker = root.join("ran");
    std::fs::write(
        root.join("checks.toml"),
        format!(
            "[[check]]\nid='native'\nrules=['C1']\nrun=\"touch '{}'\"\noutput='exit'\n[[check]]\nid='release-scan'\nrules=['C1']\nrun='secret-command'\noutput='sarif'\non='release'",
            marker.display()
        ),
    )?;
    Ok(temp)
}

#[test]
fn explanation_lists_declarations_without_executing_commands() -> io::Result<()> {
    let temp = fixture()?;
    let root = &temp.0;
    let marker = root.join("ran");
    let dir = root.to_string_lossy();
    let out = run(&["doctor", "--explain", "--dir", &dir])?;
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Configuration only: mappings and waivers; no checks executed."));
    assert!(text.contains("Enforcement fixture evidence: not assessed by doctor."));
    assert!(text.contains("check  native  output=exit  on=always  rules=C1"));
    assert!(text.contains("check  release-scan  output=sarif  on=release  rules=C1"));
    assert!(!text.contains("secret-command"));
    assert!(!text.contains("touch"));
    assert!(!marker.exists());
    let conflict = run(&["doctor", "--explain", "--json", "--dir", &dir])?;
    assert_eq!(conflict.status.code(), Some(2));
    assert!(!marker.exists());
    Ok(())
}

#[test]
fn explanation_preserves_waivers_and_gap_exit_status() -> io::Result<()> {
    for (fixture, code, detail) in [
        (
            "waived",
            0,
            "waived  C2  no ast-grep rule for this language yet",
        ),
        (
            "unmapped",
            1,
            "C2  rules.toml  promises check, but no [[check]] lists it",
        ),
    ] {
        let dir = format!("{}/tests/fixtures/{fixture}", env!("CARGO_MANIFEST_DIR"));
        let out = run(&["doctor", "--explain", "--dir", &dir])?;
        assert_eq!(out.status.code(), Some(code));
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains(detail));
        assert!(text.contains("no checks executed"));
    }
    Ok(())
}

#[test]
fn new_mapping_fields_escape_terminal_controls() -> io::Result<()> {
    let temp = fixture()?;
    let id = serde_json::to_string("C\n\u{1b}")?;
    std::fs::write(
        temp.0.join("rules.toml"),
        format!("version=1\n[[rule]]\nid={id}\nsection='Code'\ntext='rule'\nenforce=['check']"),
    )?;
    std::fs::write(
        temp.0.join("checks.toml"),
        format!("[[check]]\nid={id}\nrules=[{id}]\nrun='never execute'\noutput='exit'\non={id}"),
    )?;
    let out = run(&["doctor", "--explain", "--dir", &temp.0.to_string_lossy()])?;
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(!text.contains('\u{1b}'));
    assert_eq!(text.lines().count(), 4);
    assert!(text.contains("check  C\\n\\u{1b}  output=exit  on=C\\n\\u{1b}  rules=C\\n\\u{1b}"));
    Ok(())
}
