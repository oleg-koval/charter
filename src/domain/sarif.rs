use std::collections::BTreeMap;

use serde::Deserialize;

use super::ParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule_id: String,
    pub file: String,
    pub line: u32,
    pub message: String,
    pub fingerprint: String,
}

#[derive(Deserialize)]
struct Log {
    #[serde(default)]
    runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Run {
    #[serde(default)]
    results: Vec<RawResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawResult {
    #[serde(default)]
    rule_id: String,
    #[serde(default)]
    message: Text,
    #[serde(default)]
    locations: Vec<Location>,
    #[serde(default)]
    partial_fingerprints: BTreeMap<String, String>,
}

#[derive(Deserialize, Default)]
struct Text {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Location {
    physical_location: Physical,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Physical {
    artifact_location: Artifact,
    #[serde(default)]
    region: Region,
}

#[derive(Deserialize)]
struct Artifact {
    #[serde(default)]
    uri: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Region {
    #[serde(default)]
    start_line: u32,
    #[serde(default)]
    snippet: Text,
}

/// # Errors
/// The text is not a SARIF log: invalid JSON or a result with the wrong shape.
pub fn parse_sarif(text: &str) -> Result<Vec<Finding>, ParseError> {
    let log: Log =
        serde_json::from_str(text).map_err(|e| ParseError(format!("invalid SARIF: {e}")))?;
    Ok(log
        .runs
        .into_iter()
        .flat_map(|run| run.results)
        .map(finding)
        .collect())
}

fn finding(raw: RawResult) -> Finding {
    let (file, line, snippet) = raw
        .locations
        .into_iter()
        .next()
        .map(|l| l.physical_location)
        .map(|p| {
            (
                p.artifact_location.uri,
                p.region.start_line,
                p.region.snippet.text,
            )
        })
        .unwrap_or_default();
    // Without a tool-provided fingerprint the key is the finding's content, not its line,
    // so unrelated edits above it do not make an old finding look new.
    let fingerprint = raw
        .partial_fingerprints
        .into_values()
        .next()
        .unwrap_or_else(|| format!("{}|{file}|{}", raw.rule_id, snippet.trim()));
    Finding {
        rule_id: raw.rule_id,
        file,
        line,
        message: raw.message.text,
        fingerprint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(result_extra: &str) -> String {
        format!(
            r#"{{"runs":[{{"results":[{{"ruleId":"r1","message":{{"text":"bad"}},
            "locations":[{{"physicalLocation":{{"artifactLocation":{{"uri":"a.rs"}},
            "region":{{"startLine":7,"snippet":{{"text":"  let mut x;  "}}}}}}}}]{result_extra}}}]}}]}}"#
        )
    }

    #[test]
    fn uses_the_first_partial_fingerprint() {
        let text = log(r#","partialFingerprints":{"primary":"abc"}"#);
        let found = parse_sarif(&text).unwrap_or_default();
        assert_eq!(found.len(), 1);
        assert_eq!(found.first().map(|f| f.fingerprint.as_str()), Some("abc"));
        assert_eq!(found.first().map(|f| f.line), Some(7));
        assert_eq!(found.first().map(|f| f.file.as_str()), Some("a.rs"));
    }

    #[test]
    fn falls_back_to_rule_file_and_trimmed_snippet() {
        let found = parse_sarif(&log("")).unwrap_or_default();
        let key = found.first().map(|f| f.fingerprint.as_str());
        assert_eq!(key, Some("r1|a.rs|let mut x;"));
    }

    #[test]
    fn no_results_is_no_findings() {
        assert_eq!(parse_sarif(r#"{"runs":[{"results":[]}]}"#), Ok(vec![]));
    }

    #[test]
    fn malformed_json_names_a_reason() {
        let message = parse_sarif("{not json").err().map(|e| e.to_string());
        let message = message.unwrap_or_default();
        assert!(message.starts_with("invalid SARIF: "), "{message}");
    }
}
