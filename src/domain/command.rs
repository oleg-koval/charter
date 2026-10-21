#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Greet(String),
    Version,
    Usage,
}

/// Maps the arguments after the program name to a [`Command`].
#[must_use]
pub fn parse_command(args: &[String]) -> Command {
    match args {
        [single] if single == "version" => Command::Version,
        [] => Command::Usage,
        names => Command::Greet(names.join(" ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).to_owned()).collect()
    }

    #[test]
    fn no_arguments_is_usage() {
        assert_eq!(parse_command(&[]), Command::Usage);
    }

    #[test]
    fn version_alone_is_version() {
        assert_eq!(parse_command(&args(&["version"])), Command::Version);
    }

    #[test]
    fn several_words_form_one_name() {
        assert_eq!(
            parse_command(&args(&["Ada", "Lovelace"])),
            Command::Greet("Ada Lovelace".to_owned())
        );
    }
}
