use ignore::gitignore::Gitignore;
use rmcp::{
    Error as McpError,
    model::CallToolResult,
    model::{Content, Role},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::developer::lang;
use crate::developer::normalize_line_endings;

const DEFAULT_MAX_UNDO_HISTORY: usize = 10;
const MAX_WRITE_CHAR_COUNT: usize = 400_000;

#[derive(Clone)]
pub struct TextEditor {
    // Store file history for undo functionality
    file_history: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
    // Optional gitignore patterns for file access control
    ignore_patterns: Option<Arc<Gitignore>>,
    // Maximum number of undo states to keep per file
    max_history_per_file: usize,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            file_history: Arc::new(Mutex::new(HashMap::new())),
            ignore_patterns: None,
            max_history_per_file: DEFAULT_MAX_UNDO_HISTORY,
        }
    }

    pub fn new_with_history_limit(max_history: usize) -> Self {
        Self {
            file_history: Arc::new(Mutex::new(HashMap::new())),
            ignore_patterns: None,
            max_history_per_file: max_history,
        }
    }

    pub fn with_ignore_patterns(mut self, ignore_patterns: Arc<Gitignore>) -> Self {
        self.ignore_patterns = Some(ignore_patterns);
        self
    }

    fn check_ignore_patterns(&self, path: &Path) -> Result<(), McpError> {
        if let Some(ignore_patterns) = &self.ignore_patterns {
            if ignore_patterns.matched(path, false).is_ignore() {
                return Err(McpError::invalid_request(
                    format!(
                        "The file '{}' is restricted by ignore patterns",
                        path.display()
                    ),
                    None,
                ));
            }
        }
        Ok(())
    }

    pub async fn view(&self, path: String) -> Result<CallToolResult, McpError> {
        let path = PathBuf::from(path);

        // Check ignore patterns first
        self.check_ignore_patterns(&path)?;

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

            Ok(CallToolResult::success(vec![
                Content::text(formatted.clone()).with_audience(vec![Role::Assistant]),
                Content::text(formatted)
                    .with_audience(vec![Role::User])
                    .with_priority(0.0),
            ]))
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

        // Check ignore patterns first
        self.check_ignore_patterns(&path)?;

        // Check if path is an existing directory
        if path.is_dir() {
            return Err(McpError::invalid_params(
                format!(
                    "The path '{}' is an existing directory. The 'write' command can only target files.",
                    path.display()
                ),
                None,
            ));
        }

        // Check character count limit
        if file_text.chars().count() > MAX_WRITE_CHAR_COUNT {
            return Err(McpError::invalid_params(
                format!(
                    "Input content for '{}' has too many characters ({}). Maximum allowed is {}.",
                    path.display(),
                    file_text.chars().count(),
                    MAX_WRITE_CHAR_COUNT
                ),
                None,
            ));
        }

        // Save current file state for undo functionality
        self.save_file_history(&path)?;

        // Normalize line endings based on platform
        let normalized_text = normalize_line_endings(&file_text);

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                McpError::internal_error(format!("Failed to create directories: {}", e), None)
            })?;
        }

        // Write to the file
        std::fs::write(&path, &normalized_text)
            .map_err(|e| McpError::internal_error(format!("Failed to write file: {}", e), None))?;

        // Try to detect the language from the file extension
        let language = lang::get_language_identifier(&path);

        let success_message = format!("Successfully wrote to {}", path.display());
        let formatted_output = format!(
            "### {}\n```{}\n{}\n```",
            path.display(),
            language,
            file_text
        );

        Ok(CallToolResult::success(vec![
            Content::text(success_message).with_audience(vec![Role::Assistant]),
            Content::text(formatted_output)
                .with_audience(vec![Role::User])
                .with_priority(0.2),
        ]))
    }

    pub async fn str_replace(
        &self,
        path: String,
        old_str: String,
        new_str: String,
    ) -> Result<CallToolResult, McpError> {
        let path = PathBuf::from(path);

        // Check ignore patterns first
        self.check_ignore_patterns(&path)?;

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

        Ok(CallToolResult::success(vec![
            Content::text(success_message).with_audience(vec![Role::Assistant]),
            Content::text(output)
                .with_audience(vec![Role::User])
                .with_priority(0.2),
        ]))
    }

    pub async fn undo_edit(&self, path: String) -> Result<CallToolResult, McpError> {
        let path = PathBuf::from(path);

        // Check ignore patterns first
        self.check_ignore_patterns(&path)?;

        let mut history = self.file_history.lock().unwrap();
        if let Some(contents) = history.get_mut(&path) {
            if let Some(previous_content) = contents.pop() {
                // Write previous content back to file
                std::fs::write(&path, previous_content).map_err(|e| {
                    McpError::internal_error(format!("Failed to write file: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(
                    "Undid the last edit",
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
            if path.is_dir() {
                // Don't save history for directories
                return Ok(());
            }
            std::fs::read_to_string(path).map_err(|e| {
                McpError::internal_error(format!("Failed to read file for history: {}", e), None)
            })?
        } else {
            String::new() // Represents a non-existent file
        };

        let file_specific_history = history.entry(path.clone()).or_default();
        file_specific_history.push(content);

        // Enforce history limit
        if file_specific_history.len() > self.max_history_per_file && self.max_history_per_file > 0
        {
            let excess = file_specific_history.len() - self.max_history_per_file;
            file_specific_history.drain(0..excess);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ignore::gitignore::GitignoreBuilder;
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

    #[tokio::test]
    async fn test_text_editor_with_ignore_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create ignore patterns
        let mut builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        builder.add_line(None, "secret.txt").unwrap();
        builder.add_line(None, "*.env").unwrap();
        let ignore_patterns = Arc::new(builder.build().unwrap());

        let editor = TextEditor::new().with_ignore_patterns(ignore_patterns);

        // Create ignored files
        let secret_file = temp_dir.path().join("secret.txt");
        let env_file = temp_dir.path().join("test.env");
        let normal_file = temp_dir.path().join("normal.txt");

        // Try to write to ignored files
        let result = editor
            .write(
                secret_file.to_string_lossy().to_string(),
                "secret content".to_string(),
            )
            .await;
        assert!(
            result.is_err(),
            "Should not be able to write to ignored file"
        );
        if let Err(e) = result {
            assert!(e.to_string().contains("restricted by ignore patterns"));
        }

        let result = editor
            .write(
                env_file.to_string_lossy().to_string(),
                "env content".to_string(),
            )
            .await;
        assert!(
            result.is_err(),
            "Should not be able to write to ignored file"
        );

        // Should be able to write to normal file
        let result = editor
            .write(
                normal_file.to_string_lossy().to_string(),
                "normal content".to_string(),
            )
            .await;
        assert!(result.is_ok(), "Should be able to write to normal file");

        // Create the secret file externally and try to view it
        std::fs::write(&secret_file, "secret content").unwrap();
        let result = editor.view(secret_file.to_string_lossy().to_string()).await;
        assert!(result.is_err(), "Should not be able to view ignored file");

        // Should be able to view normal file
        let result = editor.view(normal_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok(), "Should be able to view normal file");

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_write_undo_functionality() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        let editor = TextEditor::new();

        // Write initial content
        editor
            .write(
                test_file.to_string_lossy().to_string(),
                "Initial content".to_string(),
            )
            .await
            .unwrap();

        // Write new content (should be undoable)
        editor
            .write(
                test_file.to_string_lossy().to_string(),
                "New content".to_string(),
            )
            .await
            .unwrap();

        // Verify new content
        let view_result = editor.view(test_file.to_string_lossy().to_string()).await;
        let call_result = view_result.unwrap();
        let text = call_result.content[0].as_text().unwrap();
        assert!(text.text.contains("New content"));

        // Undo the write
        let undo_result = editor
            .undo_edit(test_file.to_string_lossy().to_string())
            .await;
        assert!(undo_result.is_ok());

        // Verify content reverted
        let view_result = editor.view(test_file.to_string_lossy().to_string()).await;
        let call_result = view_result.unwrap();
        let text = call_result.content[0].as_text().unwrap();
        assert!(text.text.contains("Initial content"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_write_to_directory_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir_path = temp_dir.path().join("test_dir");
        std::fs::create_dir(&dir_path).unwrap();

        let editor = TextEditor::new();

        // Try to write to a directory
        let result = editor
            .write(
                dir_path.to_string_lossy().to_string(),
                "content".to_string(),
            )
            .await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("is an existing directory"));
        }

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_write_character_limit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        let editor = TextEditor::new();

        // Create content exceeding the character limit
        let large_content = "x".repeat(MAX_WRITE_CHAR_COUNT + 1);

        let result = editor
            .write(test_file.to_string_lossy().to_string(), large_content)
            .await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("too many characters"));
        }

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_history_limit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // Create editor with small history limit
        let editor = TextEditor::new_with_history_limit(2);

        // Write initial content
        editor
            .write(
                test_file.to_string_lossy().to_string(),
                "Content 1".to_string(),
            )
            .await
            .unwrap();

        // Make multiple edits to exceed history limit
        for i in 2..=5 {
            editor
                .str_replace(
                    test_file.to_string_lossy().to_string(),
                    format!("Content {}", i - 1),
                    format!("Content {}", i),
                )
                .await
                .unwrap();
        }

        // Should only be able to undo 2 times (the limit)
        for _ in 0..2 {
            let undo_result = editor
                .undo_edit(test_file.to_string_lossy().to_string())
                .await;
            assert!(undo_result.is_ok());
        }

        // Third undo should fail
        let undo_result = editor
            .undo_edit(test_file.to_string_lossy().to_string())
            .await;
        assert!(undo_result.is_err());
        if let Err(e) = undo_result {
            assert!(e.to_string().contains("No edit history available"));
        }

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_undo_write_to_new_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("new_file.txt");

        let editor = TextEditor::new();

        // Write to a new file
        editor
            .write(
                test_file.to_string_lossy().to_string(),
                "New file content".to_string(),
            )
            .await
            .unwrap();

        // Verify file exists and has content
        assert!(test_file.exists());
        let content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "New file content");

        // Undo the write
        let undo_result = editor
            .undo_edit(test_file.to_string_lossy().to_string())
            .await;
        assert!(undo_result.is_ok());

        // File should now be empty (representing the non-existent state)
        let content = std::fs::read_to_string(&test_file).unwrap();
        assert!(content.is_empty());

        temp_dir.close().unwrap();
    }
}
