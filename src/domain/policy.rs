use std::fmt;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Check,
    Review,
    Artifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Rule {
    pub id: String,
    pub section: String,
    pub text: String,
    pub enforce: Vec<Layer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Policy {
    pub version: u32,
    #[serde(default)]
    pub rule: Vec<Rule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Output {
    Exit,
    Sarif,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Check {
    pub id: String,
    pub rules: Vec<String>,
    pub run: String,
    pub output: Output,
    pub on: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Artifact {
    pub heading: String,
    pub rules: Vec<String>,
    #[serde(default)]
    pub first: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Waiver {
    pub rule: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct Mechanism {
    #[serde(default)]
    pub check: Vec<Check>,
    #[serde(default)]
    pub artifact: Vec<Artifact>,
    #[serde(default)]
    pub waive: Vec<Waiver>,
}

impl Mechanism {
    pub(super) fn lists_check(&self, rule: &str) -> bool {
        self.check.iter().any(|c| c.rules.iter().any(|r| r == rule))
    }

    pub(super) fn lists_artifact(&self, rule: &str) -> bool {
        self.artifact
            .iter()
            .any(|a| a.rules.iter().any(|r| r == rule))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub(super) String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// # Errors
/// The text is not valid TOML for a policy. Unknown keys are allowed so the schema can grow.
pub fn parse_policy(text: &str) -> Result<Policy, ParseError> {
    toml::from_str(text).map_err(|err| ParseError(describe(text, &err)))
}

/// # Errors
/// The text is not valid TOML for a mechanism file.
pub fn parse_mechanism(text: &str) -> Result<Mechanism, ParseError> {
    toml::from_str(text).map_err(|err| ParseError(describe(text, &err)))
}

// toml's Display leads with the position and buries the reason in a source excerpt;
// the gap format is one line, so rebuild it from the message and the span.
fn describe(text: &str, err: &toml::de::Error) -> String {
    match err.span() {
        Some(span) => {
            let before = text.get(..span.start).unwrap_or_default();
            let line = 1 + before.matches('\n').count();
            format!("line {line}: {}", err.message())
        }
        None => err.message().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_layers_in_lowercase() {
        let text = "version = 1\n[[rule]]\nid = \"A\"\nsection = \"S\"\ntext = \"t\"\nenforce = [\"check\", \"review\", \"artifact\"]\nextra = 1\n";
        let policy = parse_policy(text).map(|p| p.rule.len());
        assert_eq!(policy, Ok(1));
    }

    #[test]
    fn rejects_an_unknown_layer() {
        let text = "version = 1\n[[rule]]\nid = \"A\"\nsection = \"S\"\ntext = \"t\"\nenforce = [\"vibes\"]\n";
        let message = parse_policy(text).err().map(|e| e.to_string());
        let message = message.unwrap_or_default();
        assert!(message.contains("unknown variant"), "{message}");
        assert!(message.contains("line "), "{message}");
    }

    #[test]
    fn empty_mechanism_defaults_to_no_checks() {
        assert_eq!(parse_mechanism(""), Ok(Mechanism::default()));
    }
}
