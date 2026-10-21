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

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct Mechanism {
    #[serde(default)]
    pub check: Vec<Check>,
    #[serde(default)]
    pub artifact: Vec<Artifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// # Errors
/// The text is not valid TOML for a policy. Unknown keys are allowed so the schema can grow.
pub fn parse_policy(text: &str) -> Result<Policy, ParseError> {
    toml::from_str(text).map_err(|err| ParseError(first_line(&err.to_string())))
}

/// # Errors
/// The text is not valid TOML for a mechanism file.
pub fn parse_mechanism(text: &str) -> Result<Mechanism, ParseError> {
    toml::from_str(text).map_err(|err| ParseError(first_line(&err.to_string())))
}

// toml errors print a multi-line source excerpt; the gap format is one line.
fn first_line(message: &str) -> String {
    message.lines().next().unwrap_or_default().to_owned()
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
        assert!(parse_policy(text).is_err());
    }

    #[test]
    fn empty_mechanism_defaults_to_no_checks() {
        assert_eq!(parse_mechanism(""), Ok(Mechanism::default()));
    }
}
