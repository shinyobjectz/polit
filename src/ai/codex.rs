use std::fmt;
use std::process::Command;

use super::provider::parse_dm_response;
use super::tools::DmResponse;
use super::{AiProvider, DmMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexError {
    BinaryMissing(String),
    AuthenticationRequired(String),
    HealthCheckFailed(String),
    CommandFailed(String),
    EmptyResponse,
}

impl fmt::Display for CodexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodexError::BinaryMissing(message)
            | CodexError::AuthenticationRequired(message)
            | CodexError::HealthCheckFailed(message)
            | CodexError::CommandFailed(message) => f.write_str(message),
            CodexError::EmptyResponse => f.write_str("codex returned an empty response"),
        }
    }
}

impl std::error::Error for CodexError {}

#[derive(Debug, Clone, Default)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub last_message: Option<String>,
}

pub trait CommandRunner: Send {
    fn run(&mut self, program: &str, args: &[&str]) -> Result<CommandOutput, CodexError>;
}

#[derive(Debug, Default)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&mut self, program: &str, args: &[&str]) -> Result<CommandOutput, CodexError> {
        let mut command = Command::new(program);
        let mut owned_args: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        let temp_file = if args.first() == Some(&"exec") {
            Some(tempfile::NamedTempFile::new().map_err(|error| {
                CodexError::CommandFailed(format!("failed to create temp file: {}", error))
            })?)
        } else {
            None
        };

        if let Some(file) = temp_file.as_ref() {
            let insert_at = owned_args.len().saturating_sub(1);
            owned_args.insert(insert_at, "--output-last-message".to_string());
            owned_args.insert(
                insert_at + 1,
                file.path().to_string_lossy().to_string(),
            );
        }

        command.args(&owned_args);

        let output = command.output().map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => CodexError::BinaryMissing(format!(
                "codex binary not found: {}",
                program
            )),
            _ => CodexError::CommandFailed(format!("failed to run {}: {}", program, error)),
        })?;

        let last_message = temp_file.as_ref().and_then(|file| {
            let content = std::fs::read_to_string(file.path()).ok();
            content
        });

        let last_message = last_message
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty());

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
            last_message,
        })
    }
}

#[derive(Debug)]
pub struct CodexProvider<R = SystemCommandRunner> {
    runner: R,
}

impl CodexProvider<SystemCommandRunner> {
    pub fn system() -> Result<Self, CodexError> {
        Self::new(SystemCommandRunner)
    }
}

impl<R: CommandRunner> CodexProvider<R> {
    pub fn new(mut runner: R) -> Result<Self, CodexError> {
        Self::validate(&mut runner)?;
        Ok(Self { runner })
    }

    fn validate(runner: &mut R) -> Result<(), CodexError> {
        let output = runner.run("codex", &["login", "status"])?;
        let status_text = format!("{}\n{}", output.stdout, output.stderr);

        if !output.success {
            return Err(CodexError::HealthCheckFailed(format!(
                "codex login status failed: {}",
                status_text.trim()
            )));
        }

        if is_authenticated_status(&status_text) {
            Ok(())
        } else {
            Err(CodexError::AuthenticationRequired(
                "codex is installed, but the local session is not authenticated".to_string(),
            ))
        }
    }

    fn run_exec(&mut self, prompt: &str) -> Result<CommandOutput, CodexError> {
        self.runner.run(
            "codex",
            &[
                "exec",
                "--json",
                "--skip-git-repo-check",
                "--sandbox",
                "read-only",
                prompt,
            ],
        )
    }
}

impl<R: CommandRunner> AiProvider for CodexProvider<R> {
    fn name(&self) -> &str {
        "codex"
    }

    fn generate(
        &mut self,
        prompt: &str,
        _mode: DmMode,
    ) -> Result<DmResponse, Box<dyn std::error::Error + Send + Sync>> {
        let output = self.run_exec(prompt)?;

        if !output.success {
            let message = if output.stderr.trim().is_empty() {
                output.stdout.trim().to_string()
            } else {
                output.stderr.trim().to_string()
            };
            return Err(CodexError::CommandFailed(format!(
                "codex exec failed: {}",
                message
            ))
            .into());
        }

        let raw = output
            .last_message
            .or_else(|| {
                let stdout = output.stdout.trim().to_string();
                if stdout.is_empty() {
                    None
                } else {
                    Some(stdout)
                }
            })
            .ok_or(CodexError::EmptyResponse)?;

        Ok(parse_dm_response(&raw))
    }
}

fn is_authenticated_status(status_text: &str) -> bool {
    let normalized = status_text.to_ascii_lowercase();
    normalized.contains("logged in")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MissingBinaryRunner;
    #[derive(Debug)]
    struct UnusableHealthRunner;
    #[derive(Debug)]
    struct ValidResponseRunner;

    impl CommandRunner for MissingBinaryRunner {
        fn run(&mut self, _program: &str, _args: &[&str]) -> Result<CommandOutput, CodexError> {
            Err(CodexError::BinaryMissing("codex binary missing".to_string()))
        }
    }

    impl CommandRunner for UnusableHealthRunner {
        fn run(&mut self, _program: &str, args: &[&str]) -> Result<CommandOutput, CodexError> {
            if args == ["login", "status"] {
                Ok(CommandOutput {
                    stdout: String::new(),
                    stderr: "not logged in".to_string(),
                    success: false,
                    last_message: None,
                })
            } else {
                panic!("unexpected codex command: {args:?}");
            }
        }
    }

    impl CommandRunner for ValidResponseRunner {
        fn run(&mut self, _program: &str, args: &[&str]) -> Result<CommandOutput, CodexError> {
            if args == ["login", "status"] {
                return Ok(CommandOutput {
                    stdout: "Logged in using ChatGPT".to_string(),
                    stderr: String::new(),
                    success: true,
                    last_message: None,
                });
            }

            if matches!(args.first(), Some(&"exec")) {
                return Ok(CommandOutput {
                    stdout: String::new(),
                    stderr: String::new(),
                    success: true,
                    last_message: Some(
                        r#"{"narration":"Codex says hello.","tool_calls":[]}"#.to_string(),
                    ),
                });
            }

            panic!("unexpected codex command: {args:?}");
        }
    }

    #[test]
    fn missing_codex_binary_returns_dedicated_validation_error() {
        let err = CodexProvider::new(MissingBinaryRunner).unwrap_err();
        assert!(matches!(err, CodexError::BinaryMissing(_)));
    }

    #[test]
    fn unusable_codex_health_check_returns_setup_blocking_error() {
        let err = CodexProvider::new(UnusableHealthRunner).unwrap_err();
        assert!(matches!(
            err,
            CodexError::AuthenticationRequired(_) | CodexError::HealthCheckFailed(_)
        ));
    }

    #[test]
    fn valid_codex_response_maps_into_dm_response() {
        let mut provider = CodexProvider::new(ValidResponseRunner).unwrap();

        let response = provider.generate("prompt", DmMode::DungeonMaster).unwrap();

        assert_eq!(response.narration, "Codex says hello.");
        assert!(response.tool_calls.is_empty());
    }
}
