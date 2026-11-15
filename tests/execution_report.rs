mod support;
use std::process::Output;

fn run(fixture: &str, flags: &[&str]) -> std::io::Result<Output> {
    support::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["check", "--report", "--dir"])
        .arg(format!("tests/fixtures/{fixture}"))
        .args(flags)
        .output()
}

#[test]
fn completed_native_checks_report_verdicts_without_enforcement_claims() -> std::io::Result<()> {
    for (fixture, success) in [("check-pass", true), ("check-fail", false)] {
        let output = run(fixture, &[])?;
        assert_eq!(output.status.success(), success);
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["version"], 1);
        assert_eq!(report["ok"], success);
        assert_eq!(report["scope"], "all");
        assert_eq!(report["counts"]["completed"], 1);
        assert_eq!(
            report["checks"][0]["result"]["exit_code"],
            i32::from(!success)
        );
        assert_eq!(report["fixture_evidence"], "unknown");
        assert_eq!(report["enforcement_evidence"], "unknown");
    }
    Ok(())
}

#[test]
fn report_conflicts_with_legacy_json_before_loading_configuration() -> std::io::Result<()> {
    let output = run("does-not-exist", &["--json"])?;
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("cannot be used with"), "{error}");
    assert!(!error.contains("rules.toml"));
    Ok(())
}
