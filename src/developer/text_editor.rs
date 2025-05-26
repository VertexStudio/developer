use rmcp::{Error as McpError, model::CallToolResult, model::Content};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::developer::lang;

/// Normalize line endings based on platform
fn normalize_line_endings(text: &str) -> String {
    if cfg!(windows) {
        text.replace('\n', "\r\n")
    } else {
        text.replace("\r\n", "\n")
    }
}

#[derive(Clone)]
pub struct TextEditor {
    // Store file history for undo functionality
    file_history: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            file_history: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn view(&self, path: String) -> Result<CallToolResult, McpError> {
        let path = PathBuf::from(path);

        if path.is_file() {
            // Check file size first (400KB limit)
            const MAX_FILE_SIZE: u64 = 400 * 1024; // 400KB in bytes
            const MAX_CHAR_COUNT: usize = 400_000; // 409600 chars = 400KB

            let file_size = std::fs::metadata(&path)
                .map_err(|e| {
                    McpError::internal_error(format!("Failed to get file metadata: {}", e), None)
                })?
                .len();

            if file_size > MAX_FILE_SIZE {
                return Err(McpError::invalid_params(
                    format!(
                        "File '{}' is too large ({:.2}KB). Maximum size is 400KB to prevent memory issues.",
                        path.display(),
                        file_size as f64 / 1024.0
                    ),
                    None,
                ));
            }

            let content = std::fs::read_to_string(&path).map_err(|e| {
                McpError::internal_error(format!("Failed to read file: {}", e), None)
            })?;

            let char_count = content.chars().count();
            if char_count > MAX_CHAR_COUNT {
                return Err(McpError::invalid_params(
                    format!(
                        "File '{}' has too many characters ({}). Maximum character count is {}.",
                        path.display(),
                        char_count,
                        MAX_CHAR_COUNT
                    ),
                    None,
                ));
            }

            let language = lang::get_language_identifier(&path);
            let formatted = format!("### {}\n```{}\n{}\n```", path.display(), language, content);

            Ok(CallToolResult::success(vec![Content::text(formatted)]))
        } else {
            Err(McpError::invalid_params(
                format!(
                    "The path '{}' does not exist or is not a file.",
                    path.display()
                ),
                None,
            ))
        }
    }

    pub async fn write(&self, path: String, file_text: String) -> Result<CallToolResult, McpError> {
        let path = PathBuf::from(path);

        // Normalize line endings based on platform
        let normalized_text = normalize_line_endings(&file_text);

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                McpError::internal_error(format!("Failed to create directories: {}", e), None)
            })?;
        }

        // Write to the file
        std::fs::write(&path, normalized_text)
            .map_err(|e| McpError::internal_error(format!("Failed to write file: {}", e), None))?;

        // Try to detect the language from the file extension
        let language = lang::get_language_identifier(&path);

        let formatted = format!(
            "Successfully wrote to {}\n\n### {}\n```{}\n{}\n```",
            path.display(),
            path.display(),
            language,
            file_text
        );

        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    pub async fn str_replace(
        &self,
        path: String,
        old_str: String,
        new_str: String,
    ) -> Result<CallToolResult, McpError> {
        let path = PathBuf::from(path);

        // Check if file exists
        if !path.exists() {
            return Err(McpError::invalid_params(
                format!(
                    "File '{}' does not exist, you can write a new file with the `write` command",
                    path.display()
                ),
                None,
            ));
        }

        // Read content
        let content = std::fs::read_to_string(&path)
            .map_err(|e| McpError::internal_error(format!("Failed to read file: {}", e), None))?;

        // Ensure 'old_str' appears exactly once
        if content.matches(&old_str).count() > 1 {
            return Err(McpError::invalid_params(
                "'old_str' must appear exactly once in the file, but it appears multiple times"
                    .to_string(),
                None,
            ));
        }
        if content.matches(&old_str).count() == 0 {
            return Err(McpError::invalid_params(
                "'old_str' must appear exactly once in the file, but it does not appear in the file. Make sure the string exactly matches existing file content, including whitespace!".to_string(),
                None,
            ));
        }

        // Save history for undo
        self.save_file_history(&path)?;

        // Replace and write back with platform-specific line endings
        let new_content = content.replace(&old_str, &new_str);
        let normalized_content = normalize_line_endings(&new_content);
        std::fs::write(&path, &normalized_content)
            .map_err(|e| McpError::internal_error(format!("Failed to write file: {}", e), None))?;

        // Try to detect the language from the file extension
        let language = lang::get_language_identifier(&path);

        // Show a snippet of the changed content with context
        const SNIPPET_LINES: usize = 4;

        // Count newlines before the replacement to find the line number
        let replacement_line = content
            .split(&old_str)
            .next()
            .expect("should split on already matched content")
            .matches('\n')
            .count();

        // Calculate start and end lines for the snippet
        let start_line = replacement_line.saturating_sub(SNIPPET_LINES);
        let end_line = replacement_line + SNIPPET_LINES + new_str.matches('\n').count();

        // Get the relevant lines for our snippet
        let lines: Vec<&str> = new_content.lines().collect();
        let snippet = lines
            .iter()
            .skip(start_line)
            .take(end_line - start_line + 1)
            .cloned()
            .collect::<Vec<&str>>()
            .join("\n");

        let output = format!("```{}\n{}\n```", language, snippet);

        let success_message = format!(
            "The file {} has been edited, and the section now reads:\n{}\nReview the changes above for errors. Undo and edit the file again if necessary!",
            path.display(),
            output
        );

        Ok(CallToolResult::success(vec![Content::text(
            success_message,
        )]))
    }

    pub async fn undo_edit(&self, path: String) -> Result<CallToolResult, McpError> {
        let path = PathBuf::from(path);

        let mut history = self.file_history.lock().unwrap();
        if let Some(contents) = history.get_mut(&path) {
            if let Some(previous_content) = contents.pop() {
                // Write previous content back to file
                std::fs::write(&path, previous_content).map_err(|e| {
                    McpError::internal_error(format!("Failed to write file: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(
                    "Undid the last edit".to_string(),
                )]))
            } else {
                Err(McpError::invalid_params(
                    "No edit history available to undo".to_string(),
                    None,
                ))
            }
        } else {
            Err(McpError::invalid_params(
                "No edit history available to undo".to_string(),
                None,
            ))
        }
    }

    fn save_file_history(&self, path: &PathBuf) -> Result<(), McpError> {
        let mut history = self.file_history.lock().unwrap();
        let content = if path.exists() {
            std::fs::read_to_string(path).map_err(|e| {
                McpError::internal_error(format!("Failed to read file: {}", e), None)
            })?
        } else {
            String::new()
        };
        history.entry(path.clone()).or_default().push(content);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_text_editor_write_and_view_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        let editor = TextEditor::new();

        // Create a new file
        let result = editor
            .write(
                test_file.to_string_lossy().to_string(),
                "Hello, world!".to_string(),
            )
            .await;
        assert!(result.is_ok());

        // View the file
        let view_result = editor.view(test_file.to_string_lossy().to_string()).await;
        assert!(view_result.is_ok());
        let content = view_result.unwrap().content;
        assert!(!content.is_empty());
        let text = content[0].as_text().unwrap();
        assert!(text.text.contains("Hello, world!"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_text_editor_str_replace() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        let editor = TextEditor::new();

        // Create a new file
        editor
            .write(
                test_file.to_string_lossy().to_string(),
                "Hello, world!".to_string(),
            )
            .await
            .unwrap();

        // Replace string
        let replace_result = editor
            .str_replace(
                test_file.to_string_lossy().to_string(),
                "world".to_string(),
                "Rust".to_string(),
            )
            .await;
        assert!(replace_result.is_ok());

        // View the file to verify the change
        let view_result = editor.view(test_file.to_string_lossy().to_string()).await;
        let call_result = view_result.unwrap();
        let text = call_result.content[0].as_text().unwrap();
        assert!(text.text.contains("Hello, Rust!"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_text_editor_undo_edit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        let editor = TextEditor::new();

        // Create a new file
        editor
            .write(
                test_file.to_string_lossy().to_string(),
                "First line".to_string(),
            )
            .await
            .unwrap();

        // Replace string
        editor
            .str_replace(
                test_file.to_string_lossy().to_string(),
                "First line".to_string(),
                "Second line".to_string(),
            )
            .await
            .unwrap();

        // Undo the edit
        let undo_result = editor
            .undo_edit(test_file.to_string_lossy().to_string())
            .await;
        assert!(undo_result.is_ok());

        // View the file to verify the undo
        let view_result = editor.view(test_file.to_string_lossy().to_string()).await;
        let call_result = view_result.unwrap();
        let text = call_result.content[0].as_text().unwrap();
        assert!(text.text.contains("First line"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_text_editor_size_limits() {
        let temp_dir = tempfile::tempdir().unwrap();
        let large_file = temp_dir.path().join("large.txt");

        // Create a file larger than 400KB
        let mut file = std::fs::File::create(&large_file).unwrap();
        let large_data = "x".repeat(500 * 1024); // 500KB
        file.write_all(large_data.as_bytes()).unwrap();

        let editor = TextEditor::new();
        let result = editor.view(large_file.to_string_lossy().to_string()).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("too large"));
        }

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_text_editor_nonexistent_file() {
        let editor = TextEditor::new();
        let result = editor.view("/nonexistent/file.txt".to_string()).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("does not exist"));
        }
    }
}
