#[path = "support/git.rs"]
mod git;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use std::{fs, io, path::Path, process::Output};

fn fixture(output: &str, event: &str) -> io::Result<temp_dir::TempDir> {
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
        format!(
            "[[check]]\nid='ast'\nrules=['C1']\nrun='printf x >> marker; cat findings.sarif'\noutput='{output}'\n{event}"
        ),
    )?;
    Ok(temp)
}

fn run(root: &Path, args: &[&str]) -> io::Result<Output> {
    git::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(root)
        .arg("check")
        .args(args)
        .output()
}

fn report(root: &Path, args: &[&str]) -> io::Result<serde_json::Value> {
    let output = run(root, &[&["--report"], args].concat())?;
    serde_json::from_slice(&output.stdout).map_err(io::Error::other)
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
fn event_skips_execute_nothing_and_matching_event_executes_once() -> io::Result<()> {
    let temp = fixture("exit", "on='ci'\n")?;
    let root = &temp.0;
    let conflict = run(root, &["--report", "--json"])?;
    assert_eq!(conflict.status.code(), Some(2));
    assert!(!root.join("marker").exists());
    let skipped = report(root, &[])?;
    assert_eq!(skipped["counts"]["selected"], 0);
    assert_eq!(skipped["counts"]["completed"], 0);
    assert_eq!(skipped["checks"][0]["state"], "skipped");
    assert!(!root.join("marker").exists());
    let ran = report(root, &["--on", "ci"])?;
    assert_eq!(ran["event"], "ci");
    assert_eq!(ran["counts"]["completed"], 1);
    assert_eq!(fs::read_to_string(root.join("marker"))?, "x");
    assert_eq!(ran["enforcement_evidence"], "unknown");
    Ok(())
}

#[test]
fn sarif_scope_preserves_raw_counts_and_reports_baseline_separately() -> io::Result<()> {
    let temp = fixture("sarif", "")?;
    let root = &temp.0;
    fs::write(
        root.join(".charter/baseline.json"),
        include_str!("fixtures/check-baselined/baseline.json"),
    )?;
    init_git(root)?;
    let committed = report(root, &["--changed", "HEAD"])?;
    assert_eq!(committed["scope"], "changed:HEAD");
    assert_eq!(committed["checks"][0]["result"]["raw"], 1);
    assert_eq!(committed["checks"][0]["result"]["effective"], 0);
    assert_eq!(committed["checks"][0]["result"]["filtered"], 1);
    fs::write(root.join("src/a.rs"), "changed")?;
    let working = report(root, &["--working-tree"])?;
    assert_eq!(working["scope"], "working_tree");
    assert_eq!(working["checks"][0]["result"]["baselined"], 1);
    assert_eq!(working["checks"][0]["result"]["new"], 0);
    assert_eq!(working["baseline"]["stored_fingerprints"], 1);
    assert_eq!(fs::read_to_string(root.join("marker"))?, "xx");
    fs::remove_file(root.join(".charter/baseline.json"))?;
    let output = run(root, &["--report", "--working-tree"])?;
    assert_eq!(output.status.code(), Some(1));
    let fresh: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(fresh["checks"][0]["result"]["new"], 1);
    Ok(())
}

#[test]
fn configuration_and_baseline_errors_stop_before_execution() -> io::Result<()> {
    for (name, file, contents) in [
        ("config", "rules.toml", "invalid"),
        ("baseline", "baseline.json", "[]"),
        ("gaps", "checks.toml", ""),
        ("duplicate", "checks.toml", ""),
    ] {
        let temp = fixture("exit", "")?;
        let root = &temp.0;
        let path = root.join(".charter").join(file);
        let contents = if name == "duplicate" {
            fs::read_to_string(&path)?.repeat(2)
        } else {
            contents.to_owned()
        };
        fs::write(path, contents)?;
        let output = run(root, &["--report"])?;
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert!(!root.join("marker").exists());
    }
    Ok(())
}

#[test]
fn legacy_plain_and_json_bytes_remain_unchanged() -> io::Result<()> {
    let temp = fixture("exit", "")?;
    let root = &temp.0;
    let plain = run(root, &[])?;
    assert!(plain.status.success());
    assert_eq!(plain.stdout, b"");
    let json = run(root, &["--json"])?;
    assert!(json.status.success());
    assert_eq!(json.stdout, b"{\"ok\":true,\"lines\":[]}\n");
    Ok(())
}
