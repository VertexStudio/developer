use anyhow::Result;
use ignore::gitignore::Gitignore;
use rmcp::{
    Error as McpError,
    model::CallToolResult,
    model::{Content, Role},
};

use std::{env, path::Path, process::Stdio, sync::Arc};
use tokio::process::Command;

// Import utilities from parent module
use crate::developer::normalize_line_endings;

#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub executable: String,
    pub arg: String,
    pub redirect_syntax: String,
}

impl Default for ShellConfig {
    fn default() -> Self {
        if cfg!(windows) {
            // Execute PowerShell commands directly
            Self {
                executable: "powershell.exe".to_string(),
                arg: "-NoProfile -NonInteractive -Command".to_string(),
                redirect_syntax: "2>&1".to_string(),
            }
        } else {
            // Use the user's preferred shell from the SHELL environment variable
            let shell = env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
            Self {
                executable: shell,
                arg: "-c".to_string(),
                redirect_syntax: "2>&1".to_string(),
            }
        }
    }
}

#[derive(Clone)]
pub struct Shell {
    // Shell configuration
    config: ShellConfig,
    // Optional gitignore patterns for file access control
    ignore_patterns: Option<Arc<Gitignore>>,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            config: ShellConfig::default(),
            ignore_patterns: None,
        }
    }

    pub fn with_ignore_patterns(mut self, ignore_patterns: Arc<Gitignore>) -> Self {
        self.ignore_patterns = Some(ignore_patterns);
        self
    }

    pub fn get_shell_config(&self) -> &ShellConfig {
        &self.config
    }

    pub fn format_command_for_platform(&self, command: &str) -> String {
        if cfg!(windows) {
            // For PowerShell, wrap the command in braces to handle special characters
            format!("{{ {} }} {}", command, self.config.redirect_syntax)
        } else {
            // For other shells, no braces needed
            format!("{} {}", command, self.config.redirect_syntax)
        }
    }

    fn check_ignore_patterns(&self, command: &str) -> Result<(), McpError> {
        if let Some(ignore_patterns) = &self.ignore_patterns {
            // Check if command might access ignored files and return early if it does
            let cmd_parts: Vec<&str> = command.split_whitespace().collect();
            for arg in &cmd_parts[1..] {
                // Skip command flags
                if arg.starts_with('-') {
                    continue;
                }
                // Skip invalid paths
                let path = Path::new(arg);
                if !path.exists() {
                    continue;
                }

                if ignore_patterns.matched(path, false).is_ignore() {
                    return Err(McpError::invalid_request(
                        format!(
                            "The command attempts to access '{}' which is restricted by ignore patterns",
                            arg
                        ),
                        None,
                    ));
                }
            }
        }
        Ok(())
    }

    pub async fn execute(&self, command: String) -> Result<CallToolResult, McpError> {
        // Check ignore patterns if configured
        self.check_ignore_patterns(&command)?;

        // Get platform-specific shell configuration
        let cmd_with_redirect = self.format_command_for_platform(&command);

        // Execute the command using platform-specific shell
        let child = Command::new(&self.config.executable)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .arg(&self.config.arg)
            .arg(cmd_with_redirect)
            .spawn()
            .map_err(|e| {
                McpError::invalid_request(format!("Failed to spawn command: {}", e), None)
            })?;

        // Wait for the command to complete and get output
        let output = child.wait_with_output().await.map_err(|e| {
            McpError::invalid_request(format!("Failed to wait for command: {}", e), None)
        })?;

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        // Combine stdout and stderr as they would appear in terminal
        let combined_output = if stderr_str.is_empty() {
            stdout_str.to_string()
        } else if stdout_str.is_empty() {
            stderr_str.to_string()
        } else {
            format!("{}{}", stdout_str, stderr_str)
        };

        let normalized_output = normalize_line_endings(&combined_output);

        // Check the character count of the output
        const MAX_CHAR_COUNT: usize = 400_000; // 400KB
        let char_count = normalized_output.chars().count();
        if char_count > MAX_CHAR_COUNT {
            return Err(McpError::invalid_request(
                format!(
                    "Shell output from command '{}' has too many characters ({}). Maximum character count is {}.",
                    command, char_count, MAX_CHAR_COUNT
                ),
                None,
            ));
        }

        Ok(CallToolResult::success(vec![
            Content::text(normalized_output.clone()).with_audience(vec![Role::Assistant]),
            Content::text(normalized_output)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ignore::gitignore::GitignoreBuilder;
    use serial_test::serial;
    use tempfile;

    #[tokio::test]
    #[serial]
    async fn test_shell_basic_execution() {
        let shell = Shell::new();

        let result = if cfg!(windows) {
            shell.execute("echo hello".to_string()).await
        } else {
            shell.execute("echo hello".to_string()).await
        };

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_shell_with_ignore_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create ignore patterns
        let mut builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        builder.add_line(None, "secret.txt").unwrap();
        let ignore_patterns = Arc::new(builder.build().unwrap());

        let shell = Shell::new().with_ignore_patterns(ignore_patterns);

        // Create an ignored file
        let secret_file_path = temp_dir.path().join("secret.txt");
        std::fs::write(&secret_file_path, "secret content").unwrap();

        // Try to cat the ignored file
        let result = shell
            .execute(format!("cat {}", secret_file_path.to_str().unwrap()))
            .await;
        assert!(result.is_err(), "Should not be able to cat ignored file");

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_shell_config_creation() {
        let shell = Shell::new();
        let config = shell.get_shell_config();

        if cfg!(windows) {
            assert_eq!(config.executable, "powershell.exe");
            assert!(config.arg.contains("-NoProfile"));
        } else {
            // Check that it uses the SHELL environment variable or defaults to bash
            let expected_shell = env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
            assert_eq!(config.executable, expected_shell);
            assert_eq!(config.arg, "-c");
        }
    }
}
