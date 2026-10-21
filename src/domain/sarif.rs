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

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Location {
    #[serde(default)]
    physical_location: Physical,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Physical {
    #[serde(default)]
    artifact_location: Artifact,
    #[serde(default)]
    region: Region,
}

#[derive(Deserialize, Default)]
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
    let keyed: Vec<Keyed> = log
        .runs
        .into_iter()
        .flat_map(|run| run.results)
        .map(keyed)
        .collect();
    Ok(keyed
        .iter()
        .enumerate()
        .map(|(i, k)| Finding {
            fingerprint: unique(k, keyed.get(..i).unwrap_or_default()),
            ..k.finding.clone()
        })
        .collect())
}

struct Keyed {
    finding: Finding,
    fallback: bool,
}

// Fallback keys carry the occurrence ordinal among earlier results with the same base, so two
// identical snippets in one file stay distinct without depending on line numbers.
fn unique(k: &Keyed, earlier: &[Keyed]) -> String {
    if !k.fallback {
        return k.finding.fingerprint.clone();
    }
    let n = earlier
        .iter()
        .filter(|o| o.fallback && o.finding.fingerprint == k.finding.fingerprint)
        .count();
    format!("{}#{n}", k.finding.fingerprint)
}

fn first_location(locations: Vec<Location>) -> (String, u32, String) {
    locations
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
        .unwrap_or_default()
}

fn keyed(raw: RawResult) -> Keyed {
    let (file, line, snippet) = first_location(raw.locations);
    // Without a tool-provided fingerprint the key is the finding's content, not its line,
    // so unrelated edits above it do not make an old finding look new.
    let provided = raw.partial_fingerprints.into_values().next();
    let fallback = provided.is_none();
    let fingerprint =
        provided.unwrap_or_else(|| format!("{}|{file}|{}", raw.rule_id, snippet.trim()));
    Keyed {
        finding: Finding {
            rule_id: raw.rule_id,
            file,
            line,
            message: raw.message.text,
            fingerprint,
        },
        fallback,
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
        assert_eq!(key, Some("r1|a.rs|let mut x;#0"));
    }

    fn two(file_a: &str, file_b: &str) -> String {
        let one = |file: &str, line: u32| {
            format!(
                r#"{{"ruleId":"r1","locations":[{{"physicalLocation":{{"artifactLocation":{{"uri":"{file}"}},"region":{{"startLine":{line},"snippet":{{"text":"mut"}}}}}}}}]}}"#
            )
        };
        let (a, b) = (one(file_a, 3), one(file_b, 9));
        format!(r#"{{"runs":[{{"results":[{a},{b}]}}]}}"#)
    }

    fn keys(text: &str) -> Vec<String> {
        parse_sarif(text)
            .unwrap_or_default()
            .into_iter()
            .map(|f| f.fingerprint)
            .collect()
    }

    #[test]
    fn identical_snippets_in_one_file_get_distinct_keys() {
        let text = two("a.rs", "a.rs");
        assert_eq!(keys(&text), vec!["r1|a.rs|mut#0", "r1|a.rs|mut#1"]);
        assert_eq!(keys(&text), keys(&text));
    }

    #[test]
    fn an_unrelated_earlier_finding_does_not_shift_keys() {
        let keys = keys(&two("z.rs", "a.rs"));
        assert_eq!(keys, vec!["r1|z.rs|mut#0", "r1|a.rs|mut#0"]);
    }

    #[test]
    fn a_result_without_a_location_body_parses_with_empty_file_and_line_zero() {
        let text = r#"{"runs":[{"results":[{"ruleId":"r1","message":{"text":"m"},"locations":[{}]},{"ruleId":"r2","locations":[{"physicalLocation":{}}]}]}]}"#;
        let found = parse_sarif(text).unwrap_or_default();
        assert!(found.len() == 2 && found.iter().all(|f| f.file.is_empty() && f.line == 0));
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
