use anyhow::Result;
use ignore::gitignore::Gitignore;
use rmcp::{Error as McpError, model::CallToolResult, model::Content};

use std::{env, path::Path, process::Stdio, sync::Arc};
use tokio::process::Command;

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
            Self {
                executable: "bash".to_string(),
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

    pub fn expand_path(&self, path_str: &str) -> String {
        if cfg!(windows) {
            // Expand Windows environment variables (%VAR%)
            let with_userprofile = path_str.replace(
                "%USERPROFILE%",
                &env::var("USERPROFILE").unwrap_or_default(),
            );
            // Add more Windows environment variables as needed
            with_userprofile.replace("%APPDATA%", &env::var("APPDATA").unwrap_or_default())
        } else {
            // Unix-style expansion
            shellexpand::tilde(path_str).into_owned()
        }
    }

    pub fn is_absolute_path(&self, path_str: &str) -> bool {
        if cfg!(windows) {
            // Check for Windows absolute paths (drive letters and UNC)
            path_str.contains(":\\") || path_str.starts_with("\\\\")
        } else {
            // Unix absolute paths start with /
            path_str.starts_with('/')
        }
    }

    pub fn normalize_line_endings(&self, text: &str) -> String {
        if cfg!(windows) {
            // Ensure CRLF line endings on Windows
            text.replace("\r\n", "\n").replace("\n", "\r\n")
        } else {
            // Ensure LF line endings on Unix
            text.replace("\r\n", "\n")
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

        let normalized_output = self.normalize_line_endings(&combined_output);

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

        // Include exit status information
        let status_info = if output.status.success() {
            "Command completed successfully".to_string()
        } else {
            format!(
                "Command failed with exit code: {}",
                output.status.code().unwrap_or(-1)
            )
        };

        let final_output = if normalized_output.is_empty() {
            status_info
        } else {
            format!("{}\n\n{}", normalized_output.trim(), status_info)
        };

        Ok(CallToolResult::success(vec![Content::text(final_output)]))
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
            assert_eq!(config.executable, "bash");
            assert_eq!(config.arg, "-c");
        }
    }

    #[test]
    fn test_path_expansion() {
        let shell = Shell::new();

        if cfg!(windows) {
            // Test Windows path expansion
            let path = "%USERPROFILE%\\test";
            let expanded = shell.expand_path(path);
            assert!(!expanded.contains("%USERPROFILE%"));
        } else {
            // Test Unix path expansion
            let path = "~/test";
            let expanded = shell.expand_path(path);
            assert!(!expanded.starts_with('~'));
        }
    }

    #[test]
    fn test_absolute_path_detection() {
        let shell = Shell::new();

        if cfg!(windows) {
            assert!(shell.is_absolute_path("C:\\test"));
            assert!(shell.is_absolute_path("\\\\server\\share"));
            assert!(!shell.is_absolute_path("relative\\path"));
        } else {
            assert!(shell.is_absolute_path("/absolute/path"));
            assert!(!shell.is_absolute_path("relative/path"));
        }
    }

    #[test]
    fn test_line_ending_normalization() {
        let shell = Shell::new();
        let input = "line1\r\nline2\nline3";
        let normalized = shell.normalize_line_endings(input);

        if cfg!(windows) {
            assert_eq!(normalized, "line1\r\nline2\r\nline3");
        } else {
            assert_eq!(normalized, "line1\nline2\nline3");
        }
    }
}
