use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct Commands {
    commands: BTreeMap<String, CommandEntry>,
}

impl Commands {
    pub fn into_iter(self) -> impl Iterator<Item = (String, CommandEntry)> {
        self.commands.into_iter()
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct CommandEntry {
    pub command: String,
    pub args: Option<String>,
    pub working_dir: Option<String>,
}

pub async fn load_entries() -> Result<Option<Commands>, Box<dyn std::error::Error>> {
    let config_path: Option<std::path::PathBuf> = std::env::var("TOPSPIN_CONFIG")
        .map(Into::into)
        .ok()
        .or_else(|| {
            std::env::home_dir().map(|mut home| {
                home.push(".config/topspin.toml");
                home
            })
        });
    let config_path = if let Some(config_path) = config_path {
        config_path
    } else {
        return Ok(None);
    };
    let file_contents = tokio::fs::read_to_string(config_path).await?;
    let commands = toml::from_str(&file_contents)?;

    Ok(Some(commands))
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
