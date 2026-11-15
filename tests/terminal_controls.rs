#[path = "support/temp_dir.rs"]
mod temp_dir;

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

fn write_fixture(root: &Path) -> io::Result<String> {
    fs::write(
        root.join("rules.toml"),
        include_str!("fixtures/check-fail/rules.toml"),
    )?;
    fs::write(
        root.join("checks.toml"),
        "[[check]]\nid = 'lint'\nrules = ['C1']\nrun = 'cat findings.json'\noutput = 'sarif'\n",
    )?;
    let controls: String = (0_u8..=31).chain(127..=159).map(char::from).collect();
    let file = format!("src/東京{controls}.rs");
    let rule = "rule\x1b[31m";
    let message = format!("café ☕{controls}FORGED SUCCESS");
    let sarif = serde_json::json!({"runs": [{"results": [{
        "ruleId": rule, "message": {"text": message},
        "locations": [{"physicalLocation": {"artifactLocation": {"uri": file}, "region": {"startLine": 3}}}]
    }]}]});
    fs::write(root.join("findings.json"), sarif.to_string())?;
    Ok(format!("lint  {file}:3  {rule}: {message}"))
}

#[test]
fn plain_sarif_escapes_controls_without_changing_json_or_unicode() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    let root = temp.0.as_path();
    let raw = write_fixture(root)?;
    let plain = Command::new(env!("CARGO_BIN_EXE_charter"))
        .current_dir(root)
        .args(["check", "--dir", "."])
        .output()?;
    assert_eq!(plain.status.code(), Some(1));
    let text = String::from_utf8_lossy(&plain.stdout);
    assert_plain(&text);
    let json = Command::new(env!("CARGO_BIN_EXE_charter"))
        .current_dir(root)
        .args(["check", "--dir", ".", "--json"])
        .output()?;
    assert_eq!(json.status.code(), Some(1));
    let parsed: serde_json::Value = serde_json::from_slice(&json.stdout)?;
    assert_eq!(parsed, serde_json::json!({"ok": false, "lines": [raw]}));
    assert!(!json.stdout.contains(&0x1b));
    Ok(())
}

fn assert_plain(text: &str) {
    let controls = r"\u{0}\u{1}\u{2}\u{3}\u{4}\u{5}\u{6}\u{7}\u{8}\t\n\u{b}\u{c}\r\u{e}\u{f}\u{10}\u{11}\u{12}\u{13}\u{14}\u{15}\u{16}\u{17}\u{18}\u{19}\u{1a}\u{1b}\u{1c}\u{1d}\u{1e}\u{1f}\u{7f}\u{80}\u{81}\u{82}\u{83}\u{84}\u{85}\u{86}\u{87}\u{88}\u{89}\u{8a}\u{8b}\u{8c}\u{8d}\u{8e}\u{8f}\u{90}\u{91}\u{92}\u{93}\u{94}\u{95}\u{96}\u{97}\u{98}\u{99}\u{9a}\u{9b}\u{9c}\u{9d}\u{9e}\u{9f}";
    let expected = format!(
        "lint  src/東京{controls}.rs:3  rule\\u{{1b}}[31m: café ☕{controls}FORGED SUCCESS\n"
    );
    assert_eq!(text, expected);
    assert_eq!(text.lines().count(), 1, "{text:?}");
    assert!(
        !text.trim_end_matches('\n').chars().any(char::is_control),
        "{text:?}"
    );
    assert!(text.contains("src/東京") && text.contains("café ☕"));
    assert!(text.contains(r"\u{1b}[31m"));
    assert!(
        text.contains(r"\t")
            && text.contains(r"\n")
            && text.contains(r"\r")
            && text.contains(r"\u{9f}")
    );
}
