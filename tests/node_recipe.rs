#[path = "support/mod.rs"]
mod command;
#[path = "support/temp_dir.rs"]
mod temp_dir;

use std::{io, path::Path, process::Output};

fn charter(dir: &Path, args: &[&str]) -> io::Result<Output> {
    command::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(dir)
        .args(args)
        .output()
}
fn package(dir: &Path) -> io::Result<()> {
    std::fs::write(
        dir.join("package.json"),
        r#"{"private":true,"scripts":{"build":"node build.cjs","test":"node test.cjs"}}"#,
    )?;
    std::fs::write(
        dir.join("build.cjs"),
        "require('node:assert/strict').equal(require('node:fs').readFileSync('build-input','utf8'),'valid');",
    )?;
    std::fs::write(
        dir.join("test.cjs"),
        "const fs=require('node:fs'); fs.writeFileSync('test-ran','yes'); require('node:assert/strict').equal(fs.readFileSync('test-input','utf8'),'valid');",
    )?;
    for name in ["build-input", "test-input"] {
        std::fs::write(dir.join(name), "valid")?;
    }
    Ok(())
}
fn import(dir: &Path) -> io::Result<()> {
    let source = format!(
        "{}/recipes/node-npm/checks.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    let out = charter(dir, &["init", "--checks-from", &source])?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read(&source)?,
        std::fs::read(dir.join(".charter/checks.toml"))?
    );
    assert!(charter(dir, &["doctor"])?.status.success());
    let text = std::fs::read_to_string(source)?;
    let mechanism = charter_cli::domain::parse_mechanism(&text).map_err(io::Error::other)?;
    assert!(mechanism.waive.is_empty());
    assert_eq!(mechanism.check.len(), 2);
    assert_eq!(mechanism.check[0].rules, ["C7"]);
    assert_eq!(
        mechanism.check[1].rules,
        ["C1", "C2", "C3", "C4", "C5", "C10", "C23"]
    );
    Ok(())
}
fn check(dir: &Path, expected: &[&str]) -> io::Result<()> {
    let out = charter(dir, &["check", "--json"])?;
    assert_eq!(out.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(json["lines"], serde_json::json!(expected));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("Configure project source and secret checks")
    );
    Ok(())
}
#[test]
fn node_recipe_runs_native_build_and_tests_without_claiming_full_enforcement() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    let dir = &temp.0;
    import(dir)?;
    package(dir)?;
    check(dir, &["configure-enforcement  exited 1"])?;
    assert!(dir.join("test-ran").exists());
    std::fs::write(dir.join("test-input"), "invalid")?;
    check(
        dir,
        &[
            "node-build-test  exited 1",
            "configure-enforcement  exited 1",
        ],
    )?;
    std::fs::remove_file(dir.join("test-ran"))?;
    std::fs::write(dir.join("build-input"), "invalid")?;
    check(
        dir,
        &[
            "node-build-test  exited 1",
            "configure-enforcement  exited 1",
        ],
    )?;
    assert!(!dir.join("test-ran").exists());
    Ok(())
}
