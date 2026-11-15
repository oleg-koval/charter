#![cfg(unix)]
use std::{fs, io, path::Path};
mod support;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use temp_dir::TempDir;

fn run(root: &Path, args: &[&str]) -> io::Result<std::process::Output> {
    support::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(root)
        .env("PATH", root.join("bin"))
        .args(args)
        .output()
}

fn fixture() -> io::Result<TempDir> {
    use std::os::unix::fs::PermissionsExt;
    let temp = TempDir::create()?;
    let root = &temp.0;
    fs::create_dir_all(root.join("bin"))?;
    for tool in ["npm", "clojure", "go", "mvn"] {
        let path = root.join("bin").join(tool);
        fs::write(&path, "#!/bin/sh\nprintf executed > executed\nexit 0\n")?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    std::os::unix::fs::symlink("/bin/sh", root.join("bin/sh"))?;
    let probe = support::isolated_command("sh")
        .current_dir(root)
        .env("PATH", root.join("bin"))
        .args(["-c", "npm run build"])
        .output()?;
    assert_eq!(probe.status.code(), Some(0));
    assert_eq!(fs::read_to_string(root.join("executed"))?, "executed");
    fs::remove_file(root.join("executed"))?;
    Ok(temp)
}

#[test]
fn named_recipes_preserve_source_bytes_without_executing_tools() -> io::Result<()> {
    let temp = fixture()?;
    let root = &temp.0;
    for name in ["node-npm", "clojure-cli-test", "go-module", "java-maven"] {
        let args = ["init", "--recipe", name, "--dir", name];
        assert!(run(root, &args)?.status.success());
        let source = fs::read(format!(
            "{}/recipes/{name}/checks.toml",
            env!("CARGO_MANIFEST_DIR")
        ))?;
        let checks = root.join(name).join("checks.toml");
        assert_eq!(fs::read(&checks)?, source);
        assert!(run(root, &["doctor", "--dir", name])?.status.success());
        assert!(!root.join("executed").exists());
        assert_eq!(run(root, &args)?.status.code(), Some(1));
        assert_eq!(fs::read(checks)?, source);
        assert_pending_barrier(root, name)?;
    }
    Ok(())
}

fn assert_pending_barrier(root: &Path, name: &str) -> io::Result<()> {
    let out = run(root, &["check", "--dir", name, "--json"])?;
    assert_eq!(out.status.code(), Some(1));
    let verdict: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(
        verdict["lines"],
        serde_json::json!(["configure-enforcement  exited 1"])
    );
    assert_eq!(fs::read_to_string(root.join("executed"))?, "executed");
    fs::remove_file(root.join("executed"))?;
    Ok(())
}

#[test]
fn unknown_or_conflicting_recipe_options_never_create_destination() -> io::Result<()> {
    let temp = TempDir::create()?;
    let root = &temp.0;
    for args in [
        vec!["init", "--recipe", "unknown", "--dir", "output"],
        vec![
            "init",
            "--recipe",
            "node-npm",
            "--checks-from",
            "missing",
            "--dir",
            "output",
        ],
    ] {
        let out = run(root, &args)?;
        assert!(!out.status.success());
        assert!(!out.stderr.is_empty());
        if args[2] == "unknown" {
            let event: serde_json::Value =
                serde_json::from_slice(out.stderr.split(|b| *b == b'\n').next().unwrap())?;
            assert_eq!(event["error"]["kind"], "unknown_recipe");
        }
        assert!(!root.join("output").exists());
    }
    Ok(())
}

#[test]
fn detection_selects_or_composes_root_recipes_and_rejects_missing_manifests() -> io::Result<()> {
    let temp = fixture()?;
    let root = &temp.0;
    fs::write(root.join("go.mod"), "module example.invalid/app\n")?;
    assert_detected_go_recipe(root)?;

    fs::write(
        root.join("package.json"),
        r#"{"scripts":{"build":"echo build","test:unit":"echo unit","test:assets":"echo assets"}}"#,
    )?;
    assert_detection_composes_root_recipes(root)?;
    fs::remove_file(root.join("go.mod"))?;
    let node_only = run(root, &["init", "--detect", "--dir", "node-only"])?;
    assert!(node_only.status.success());
    let checks = fs::read_to_string(root.join("node-only/checks.toml"))?;
    assert!(checks.contains("npm run build && npm run 'test:assets' && npm run 'test:unit'"));

    let empty = TempDir::create()?;
    assert_detection_rejects_empty_root(&empty.0)?;
    Ok(())
}

fn assert_detected_go_recipe(root: &Path) -> io::Result<()> {
    let out = run(root, &["init", "--detect", "--dir", "detected"])?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read(root.join("detected/checks.toml"))?,
        fs::read(format!(
            "{}/recipes/go-module/checks.toml",
            env!("CARGO_MANIFEST_DIR")
        ))?
    );
    Ok(())
}

fn assert_detection_composes_root_recipes(root: &Path) -> io::Result<()> {
    let out = run(root, &["init", "--detect", "--dir", "composed"])?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = fs::read_to_string(root.join("composed/checks.toml"))?;
    assert!(text.contains("id = \"node-build-test\""));
    assert!(text.contains("id = \"go-tests\""));
    assert!(text.contains("npm run 'test:assets' && npm run 'test:unit'"));
    assert_eq!(text.matches("id = \"configure-enforcement\"").count(), 1);
    Ok(())
}

fn assert_detection_rejects_empty_root(root: &Path) -> io::Result<()> {
    let out = run(root, &["init", "--detect", "--dir", "missing"])?;
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no supported project manifest"));
    assert!(!root.join("missing").exists());
    Ok(())
}

#[test]
fn detection_selects_java_maven_from_pom() -> io::Result<()> {
    let temp = fixture()?;
    let root = &temp.0;
    fs::write(root.join("pom.xml"), "<project></project>")?;
    let out = run(root, &["init", "--detect", "--dir", "detected"])?;
    assert!(out.status.success());
    assert_eq!(
        fs::read(root.join("detected/checks.toml"))?,
        fs::read(format!(
            "{}/recipes/java-maven/checks.toml",
            env!("CARGO_MANIFEST_DIR")
        ))?
    );
    Ok(())
}

#[test]
fn help_lists_available_recipes() -> io::Result<()> {
    let help = run(Path::new(env!("CARGO_MANIFEST_DIR")), &["init", "--help"])?;
    let text = String::from_utf8_lossy(&help.stdout);
    for name in ["node-npm", "clojure-cli-test", "go-module", "java-maven"] {
        assert!(text.contains(name));
    }
    Ok(())
}
