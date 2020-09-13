use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct Commands {
    commands: BTreeMap<String, CommandEntry>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct CommandEntry {
    pub command: String,
    pub args: Option<String>,
    pub working_dir: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let entries: Commands = toml::from_str(
            r#"
    [commands.cat]
    command = "cat"

    [commands.netcat]
    command = "nc"
    args ="-l 7000"
    working_dir = "~/"
    "#,
        )
        .unwrap();

        assert_eq!(
            entries,
            Commands {
                commands: vec![
                    (
                        "cat".to_string(),
                        CommandEntry {
                            command: "cat".to_string(),
                            args: None,
                            working_dir: None,
                        }
                    ),
                    (
                        "netcat".to_string(),
                        CommandEntry {
                            command: "nc".to_string(),
                            args: Some("-l 7000".to_string()),
                            working_dir: Some("~/".to_string()),
                        }
                    )
                ]
                .into_iter()
                .collect()
            }
        );
    }
}
