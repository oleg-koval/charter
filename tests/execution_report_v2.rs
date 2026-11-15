#[path = "support/git.rs"]
mod git;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use std::{fs, io, path::Path, process::Output};

fn fixture() -> io::Result<temp_dir::TempDir> {
    let temp = temp_dir::TempDir::create()?;
    let root = &temp.0;
    fs::create_dir_all(root.join(".charter"))?;
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("src/a.rs"), "original")?;
    fs::write(
        root.join("findings.sarif"),
        include_str!("fixtures/check-baselined/findings.sarif"),
    )?;
    fs::write(
        root.join(".charter/rules.toml"),
        "version=1\n[[rule]]\nid='C1'\nsection='Code'\ntext='test'\nenforce=['check']\n",
    )?;
    fs::write(
        root.join(".charter/checks.toml"),
        "[[check]]\nid='ast'\nrules=['C1']\nrun='printf x >> marker; cat findings.sarif'\noutput='sarif'\n",
    )?;
    Ok(temp)
}

fn run_v2(root: &Path, args: &[&str]) -> io::Result<Output> {
    git::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(root)
        .args(["check", "--report-v2", "--dir", ".charter"])
        .args(args)
        .output()
}

fn init_git(root: &Path) -> io::Result<()> {
    let excludes = root.join("empty-excludes");
    fs::write(&excludes, "")?;
    let excludes = excludes
        .to_str()
        .ok_or_else(|| io::Error::other("non-UTF8 fixture path"))?;
    for args in [
        vec!["init"],
        vec!["config", "core.excludesFile", excludes],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test"],
        vec!["add", "."],
        vec!["commit", "-m", "fixture"],
    ] {
        assert!(git::fixture_git(root, &args)?.status.success());
    }
    Ok(())
}

#[test]
fn report_v2_allows_nonzero_exit_when_findings_are_baselined() -> io::Result<()> {
    let temp = fixture()?;
    let root = &temp.0;
    fs::write(
        root.join(".charter/baseline.json"),
        include_str!("fixtures/check-baselined/baseline.json"),
    )?;
    let checks = root.join(".charter/checks.toml");
    fs::write(
        &checks,
        fs::read_to_string(&checks)?.replace("cat findings.sarif", "cat findings.sarif; exit 1"),
    )?;
    let output = run_v2(root, &[])?;
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["ok"], true);
    assert_eq!(report["checks"][0]["state"], "pass");
    assert_eq!(report["checks"][0]["result"]["new"], 0);
    assert_eq!(
        report["checks"][0]["result"]["native_exit"],
        serde_json::json!({"kind":"exited","code":1})
    );
    Ok(())
}

#[test]
fn report_v2_honors_changed_and_working_tree_scopes() -> io::Result<()> {
    assert_scope(&["--changed", "HEAD"], "changed:HEAD")?;
    assert_scope(&["--working-tree"], "working_tree")
}

fn assert_scope(args: &[&str], scope: &str) -> io::Result<()> {
    let temp = fixture()?;
    init_git(&temp.0)?;
    fs::write(temp.0.join("unrelated.txt"), "changed")?;
    let checks = temp.0.join(".charter/checks.toml");
    fs::write(
        &checks,
        fs::read_to_string(&checks)?.replace("cat findings.sarif", "cat findings.sarif; exit 1"),
    )?;
    let output = run_v2(&temp.0, args)?;
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["scope"], scope);
    assert_eq!(report["checks"][0]["result"]["raw"], 1);
    assert_eq!(report["checks"][0]["result"]["effective"], 0);
    Ok(())
}

#[test]
fn report_v2_rejects_other_output_modes() -> io::Result<()> {
    let temp = fixture()?;
    for args in [["--json"], ["--report"]] {
        let output = run_v2(&temp.0, &args)?;
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    Ok(())
}
