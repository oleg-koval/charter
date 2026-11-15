use super::{
    Check, ImportError, Mechanism, Output, import_failure, parse_mechanism, terminal_text,
};
use std::fmt;

mod node;

const RECIPES: &[(&str, &str)] = &[
    (
        "node-npm",
        include_str!("../../recipes/node-npm/checks.toml"),
    ),
    (
        "clojure-cli-test",
        include_str!("../../recipes/clojure-cli-test/checks.toml"),
    ),
    (
        "go-module",
        include_str!("../../recipes/go-module/checks.toml"),
    ),
    (
        "java-maven",
        include_str!("../../recipes/java-maven/checks.toml"),
    ),
];

#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum InitOptionsError {
    ConflictingSources,
    UnknownRecipe(String),
    NoDetectedRecipe,
    ProjectRootRead(String),
    Import(ImportError),
}

impl fmt::Display for InitOptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingSources => f.write_str("--recipe conflicts with --checks-from"),
            Self::UnknownRecipe(name) => write!(f, "unknown recipe: {}", terminal_text(name)),
            Self::NoDetectedRecipe => {
                f.write_str("no supported project manifest found in the current directory")
            }
            Self::ProjectRootRead(message) => f.write_str(message),
            Self::Import(error) => error.fmt(f),
        }
    }
}

/// # Errors
/// Rejects combining recipe/import sources and names outside the explicit catalog.
pub fn recipe_checks(name: &str, has_import: bool) -> Result<&'static str, InitOptionsError> {
    if has_import {
        return Err(InitOptionsError::ConflictingSources);
    }
    RECIPES
        .iter()
        .find(|(id, _)| *id == name)
        .map(|(_, text)| *text)
        .ok_or_else(|| InitOptionsError::UnknownRecipe(name.to_owned()))
}

/// Selects or composes bundled recipes from the recognized root manifests.
///
/// # Errors
/// No supported manifest exists, or the embedded recipe catalog is invalid.
pub fn detect_recipes(
    manifest_names: &[String],
    package_json: Option<&str>,
) -> Result<String, InitOptionsError> {
    let detected = [
        ("node-npm", &["package.json"][..]),
        ("clojure-cli-test", &["deps.edn"][..]),
        ("go-module", &["go.mod"][..]),
        ("java-maven", &["pom.xml"][..]),
    ]
    .iter()
    .filter(|(_, markers)| {
        markers
            .iter()
            .any(|marker| manifest_names.iter().any(|name| name == marker))
    })
    .map(|(name, _)| *name)
    .collect::<Vec<_>>();
    let node_command = detected
        .contains(&"node-npm")
        .then(|| node::native_command(package_json.unwrap_or_default()))
        .transpose()?;
    match detected.as_slice() {
        [] => Err(InitOptionsError::NoDetectedRecipe),
        [name] => detected_recipe(name, node_command.as_deref()),
        names => compose_recipes(names, node_command.as_deref()),
    }
}

fn detected_recipe(name: &str, node_command: Option<&str>) -> Result<String, InitOptionsError> {
    let text = recipe_checks(name, false)?;
    if node_command.is_none_or(|command| command == "npm run build && npm test") {
        return Ok(text.to_owned());
    }
    let mechanism = recipe_mechanism(name, node_command)?;
    serialize_mechanism(&mechanism)
}

fn compose_recipes(names: &[&str], node_command: Option<&str>) -> Result<String, InitOptionsError> {
    let mechanisms = names
        .iter()
        .map(|name| recipe_mechanism(name, node_command))
        .collect::<Result<Vec<_>, _>>()?;
    let composed = Mechanism {
        check: composed_checks(&mechanisms),
        artifact: mechanisms
            .first()
            .map_or_else(Vec::new, |mechanism| mechanism.artifact.clone()),
        waive: Vec::new(),
    };
    serialize_mechanism(&composed)
}

fn recipe_mechanism(name: &str, node_command: Option<&str>) -> Result<Mechanism, InitOptionsError> {
    let mechanism = raw_recipe_mechanism(name)?;
    Ok(match node_command {
        Some(command) => node::with_command(&mechanism, command),
        None => mechanism,
    })
}

fn serialize_mechanism(mechanism: &Mechanism) -> Result<String, InitOptionsError> {
    toml::to_string(mechanism)
        .map_err(|error| InitOptionsError::Import(ImportError::Parse(error.to_string())))
}

fn raw_recipe_mechanism(name: &str) -> Result<Mechanism, InitOptionsError> {
    let text = recipe_checks(name, false)
        .map_err(|error| InitOptionsError::Import(ImportError::Parse(error.to_string())))?;
    parse_mechanism(text)
        .map_err(|error| InitOptionsError::Import(ImportError::Parse(error.to_string())))
}

fn composed_checks(mechanisms: &[Mechanism]) -> Vec<Check> {
    mechanisms
        .iter()
        .flat_map(|mechanism| mechanism.check.iter())
        .filter(|check| check.id != "configure-enforcement")
        .cloned()
        .chain(std::iter::once(enforcement_check()))
        .collect()
}

fn enforcement_check() -> Check {
    Check {
        id: "configure-enforcement".to_owned(),
        rules: vec!["C1", "C2", "C3", "C4", "C5", "C10", "C23"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        run: "echo 'Configure project source and secret checks in checks.toml for C1 C2 C3 C4 C5 C10 C23; see bundled recipe guides.' >&2; exit 1".to_owned(),
        output: Output::Exit,
        on: None,
    }
}

#[must_use]
pub fn init_options_failure(error: &InitOptionsError) -> String {
    match error {
        InitOptionsError::Import(error) => import_failure(error),
        _ => serde_json::json!({"event":"charter.init_options.failed", "metric":"charter_init_options_failures", "count":1, "error":error}).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ImportError, parse_mechanism};

    #[test]
    fn catalog_retains_native_commands_and_pending_enforcement()
    -> Result<(), Box<dyn std::error::Error>> {
        for (name, command) in [
            ("node-npm", "npm run build && npm test"),
            ("clojure-cli-test", "clojure -M:test"),
            ("go-module", "go test ./..."),
            ("java-maven", "mvn test"),
        ] {
            let text = recipe_checks(name, false).map_err(|error| error.to_string())?;
            let mechanism = parse_mechanism(text)?;
            assert_eq!(
                mechanism.check.first().map(|check| check.run.as_str()),
                Some(command)
            );
            assert!(
                mechanism.check.iter().any(
                    |check| check.id == "configure-enforcement" && check.run.contains("exit 1")
                )
            );
        }
        Ok(())
    }
    #[test]
    fn unknown_names_and_conflicting_sources_are_typed_errors() {
        assert!(matches!(
            recipe_checks("missing", false),
            Err(InitOptionsError::UnknownRecipe(_))
        ));
        assert!(matches!(
            recipe_checks("node-npm", true),
            Err(InitOptionsError::ConflictingSources)
        ));
    }

    #[test]
    fn detection_does_not_assign_clojure_cli_recipe_to_other_tool_manifests() {
        for manifest in ["project.clj", "bb.edn"] {
            assert!(matches!(
                detect_recipes(&[manifest.to_owned()], None),
                Err(InitOptionsError::NoDetectedRecipe)
            ));
        }
    }

    #[test]
    fn detection_composes_multiple_recipes_with_one_enforcement_barrier()
    -> Result<(), Box<dyn std::error::Error>> {
        let text = detect_recipes(
            &["go.mod".to_owned(), "package.json".to_owned()],
            Some(r#"{"scripts":{"build":"make build","test:functions":"make test-functions"}}"#),
        )
        .map_err(|error| error.to_string())?;
        let mechanism = parse_mechanism(&text)?;
        let ids = mechanism
            .check
            .iter()
            .map(|check| check.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            ["node-build-test", "go-tests", "configure-enforcement"]
        );
        assert_eq!(mechanism.check.last(), Some(&enforcement_check()));
        assert_eq!(
            mechanism.check.first().map(|check| check.run.as_str()),
            Some("npm run build && npm run 'test:functions'")
        );
        Ok(())
    }

    #[test]
    fn new_errors_are_observable_and_existing_import_events_stay_compatible()
    -> Result<(), serde_json::Error> {
        let error = InitOptionsError::UnknownRecipe("bad\n\u{1b}".to_owned());
        let event: serde_json::Value = serde_json::from_str(&init_options_failure(&error))?;
        assert_eq!(event["event"], "charter.init_options.failed");
        assert_eq!(event["metric"], "charter_init_options_failures");
        assert_eq!(event["count"], 1);
        assert_eq!(event["error"]["kind"], "unknown_recipe");
        assert!(!error.to_string().contains(['\n', '\u{1b}']));
        let imported = InitOptionsError::Import(ImportError::Read("missing".to_owned()));
        let event: serde_json::Value = serde_json::from_str(&init_options_failure(&imported))?;
        assert_eq!(event["event"], "charter.import.failed");
        assert_eq!(event["error"]["kind"], "read");
        assert_eq!(imported.to_string(), "missing");
        Ok(())
    }
}
