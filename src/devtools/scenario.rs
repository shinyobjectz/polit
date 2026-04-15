use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub name: String,
    pub mode: ScenarioMode,
    pub terminal: ScenarioTerminal,
    pub startup: ScenarioStartup,
    pub steps: Vec<ScenarioStep>,
    pub expect: ScenarioExpect,
}

impl Scenario {
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let yaml = fs::read_to_string(path)?;
        Ok(Self::from_yaml(&yaml)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioMode {
    InProcess,
    Pty,
    Both,
}

impl fmt::Display for ScenarioMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ScenarioMode::InProcess => "in_process",
            ScenarioMode::Pty => "pty",
            ScenarioMode::Both => "both",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioTerminal {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioStartup {
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioExpect {
    pub running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioStep {
    AssertText { assert_text: String },
    AssertNotText { assert_not_text: String },
    Press { press: String },
    Type { type_text: String },
    Snapshot { snapshot: String },
}

impl<'de> Deserialize<'de> for ScenarioStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct AssertTextStep {
            assert_text: String,
        }

        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct AssertNotTextStep {
            assert_not_text: String,
        }

        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct PressStep {
            press: String,
        }

        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct TypeStep {
            #[serde(rename = "type")]
            type_text: String,
        }

        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SnapshotStep {
            snapshot: String,
        }

        let value = serde_yaml::Value::deserialize(deserializer)?;
        let mapping = value
            .as_mapping()
            .ok_or_else(|| D::Error::custom("scenario steps must be YAML mappings"))?;

        let Some(first_key) = mapping.keys().next() else {
            return Err(D::Error::custom("scenario steps cannot be empty"));
        };

        let Some(step_name) = first_key.as_str() else {
            return Err(D::Error::custom(
                "scenario step names must be YAML string keys",
            ));
        };

        match step_name {
            "assert_text" => {
                let step: AssertTextStep =
                    serde_yaml::from_value(value).map_err(D::Error::custom)?;
                Ok(Self::AssertText {
                    assert_text: step.assert_text,
                })
            }
            "assert_not_text" => {
                let step: AssertNotTextStep =
                    serde_yaml::from_value(value).map_err(D::Error::custom)?;
                Ok(Self::AssertNotText {
                    assert_not_text: step.assert_not_text,
                })
            }
            "press" => {
                let step: PressStep = serde_yaml::from_value(value).map_err(D::Error::custom)?;
                Ok(Self::Press { press: step.press })
            }
            "type" => {
                let step: TypeStep = serde_yaml::from_value(value).map_err(D::Error::custom)?;
                Ok(Self::Type {
                    type_text: step.type_text,
                })
            }
            "snapshot" => {
                let step: SnapshotStep = serde_yaml::from_value(value).map_err(D::Error::custom)?;
                Ok(Self::Snapshot {
                    snapshot: step.snapshot,
                })
            }
            other => Err(D::Error::custom(format!("unknown scenario step '{other}'"))),
        }
    }
}
