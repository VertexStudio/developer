use ignore::gitignore::GitignoreBuilder;
use rmcp::{
    RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::ErrorData as McpError,
    model::*,
    schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde_json::json;
use std::env;
use std::sync::Arc;

// Tool descriptions (condensed from original markdown)

// Parameter structs for tools
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TextEditorParams {
    #[schemars(description = "Allowed options are: `view`, `write`, `str_replace`, `undo_edit`.")]
    pub command: String,
    #[schemars(
        description = "Absolute path to the file to operate on, e.g. `/repo/file.py`. For the `write` command, parent directories will be created if they do not exist."
    )]
    pub path: String,
    #[schemars(description = "Content to write to the file (required for write command)")]
    pub file_text: Option<String>,
    #[schemars(description = "String to replace (required for str_replace command)")]
    pub old_str: Option<String>,
    #[schemars(description = "New string to replace with (required for str_replace command)")]
    pub new_str: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShellParams {
    #[schemars(description = "Command to execute")]
    pub command: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScreenCaptureParams {
    #[schemars(description = "The display number to capture (0 is main display)")]
    pub display: Option<i32>,
    #[schemars(
        description = "Optional: the exact title of the window to capture. use the list_windows tool to find the available windows."
    )]
    pub window_title: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ImageProcessorParams {
    #[schemars(description = "Absolute path to the image file to process")]
    pub path: String,
    #[schemars(
        description = "Optional resize factor to reduce image size. Allowed values: \"1/2\", \"1/4\""
    )]
    pub resize: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WorkflowParams {
    #[schemars(description = "Detailed description of what this step accomplishes")]
    pub step_description: String,
    #[schemars(description = "Current position in the workflow sequence (e.g., 1 for first step)")]
    pub step_number: i32,
    #[schemars(description = "Estimated total number of steps in the complete workflow")]
    pub total_steps: i32,
    #[schemars(
        description = "Set to true if another step will follow this one, false if this is the final step"
    )]
    pub next_step_needed: bool,
    #[schemars(description = "Set to true if this step revises a previous step")]
    pub is_step_revision: Option<bool>,
    #[schemars(
        description = "If revising a previous step, specify which step number is being revised"
    )]
    pub revises_step: Option<i32>,
    #[schemars(
        description = "If creating a branch, specify which step number this branch starts from"
    )]
    pub branch_from_step: Option<i32>,
    #[schemars(
        description = "A unique identifier for this branch (required when creating a branch)"
    )]
    pub branch_id: Option<String>,
    #[schemars(
        description = "Indicates whether additional steps are required to complete the workflow"
    )]
    pub needs_more_steps: Option<bool>,
}

pub mod image_processor;
pub mod lang;
pub mod screen_capture;
pub mod shell;
pub mod text_editor;
pub mod workflow;

pub use image_processor::ImageProcessor;
pub use screen_capture::ScreenCapture;
pub use shell::Shell;
pub use text_editor::TextEditor;
pub use workflow::Workflow;

// Path utility functions
pub(crate) fn expand_path(path_str: &str) -> String {
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

pub(crate) fn is_absolute_path(path_str: &str) -> bool {
    if cfg!(windows) {
        // Check for Windows absolute paths (drive letters and UNC)
        path_str.contains(":\\") || path_str.starts_with("\\\\")
    } else {
        // Unix absolute paths start with /
        path_str.starts_with('/')
    }
}

pub(crate) fn normalize_line_endings(text: &str) -> String {
    if cfg!(windows) {
        // Ensure CRLF line endings on Windows
        text.replace("\r\n", "\n").replace("\n", "\r\n")
    } else {
        // Ensure LF line endings on Unix
        text.replace("\r\n", "\n")
    }
}

#[derive(Clone)]
pub struct Developer {
    text_editor: TextEditor,
    shell: Shell,
    screen_capture: ScreenCapture,
    image_processor: ImageProcessor,
    workflow: Workflow,
    tool_router: ToolRouter<Developer>,
}

#[tool_router]
impl Developer {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().expect("should have a current working dir");

        // Initialize gitignore patterns from .gitignore files
        let mut builder = GitignoreBuilder::new(&cwd);

        // Add .gitignore file if it exists
        let gitignore_path = cwd.join(".gitignore");
        if gitignore_path.exists() {
            let _ = builder.add(&gitignore_path);
        }

        // Build the ignore patterns
        let ignore_patterns = Arc::new(builder.build().unwrap_or_else(|_| {
            // Fallback to empty gitignore if building fails
            GitignoreBuilder::new(&cwd)
                .build()
                .expect("Failed to create empty gitignore")
        }));

        // Configure text editor history limit from environment or use default
        let text_editor_max_history = std::env::var("TEXT_EDITOR_MAX_HISTORY")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10);

        Self {
            text_editor: TextEditor::new_with_history_limit(text_editor_max_history)
                .with_ignore_patterns(ignore_patterns.clone()),
            shell: Shell::new().with_ignore_patterns(ignore_patterns),
            screen_capture: ScreenCapture::new(),
            image_processor: ImageProcessor::new(),
            workflow: Workflow::new(true, None, true),
            tool_router: Self::tool_router(),
        }
    }

    pub fn get_tools_schema_as_json() -> String {
        let tools: Vec<rmcp::model::Tool> = Self::tool_router().list_all();
        match serde_json::to_string_pretty(&tools) {
            Ok(json_string) => json_string,
            Err(e) => {
                format!(
                    "{{\"error\": \"Failed to serialize tools to JSON: {}\"}}",
                    e
                )
            }
        }
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    // Helper method to resolve a path relative to cwd with platform-specific handling
    fn resolve_path(&self, path_str: &str) -> Result<std::path::PathBuf, McpError> {
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let expanded = expand_path(path_str);
        let path = std::path::Path::new(&expanded);

        let suggestion = cwd.join(path);

        match is_absolute_path(&expanded) {
            true => Ok(path.to_path_buf()),
            false => Err(McpError::invalid_params(
                format!(
                    "The path {} is not an absolute path, did you possibly mean {}?",
                    path_str,
                    suggestion.to_string_lossy(),
                ),
                None,
            )),
        }
    }

    // Text Editor Tool
    #[tool(description = "Text Editor Tool: File Content Manipulation

Provides commands to perform text editing operations on files, such as viewing, creating, overwriting, and modifying content, along with an undo capability for recent changes.

Commands:
- view: View the content of a file
- write: Create or overwrite a file with the given content  
- str_replace: Replace a specific string in a file with a new string
- undo_edit: Undo the last edit made by write or str_replace to a file

Parameters:
- command (required): One of view, write, str_replace, undo_edit
- path (required): Absolute path to the file to operate on
- file_text (for write): The entire new content for the file
- old_str (for str_replace): The exact string to be replaced (must be unique)
- new_str (for str_replace): The string that will replace old_str

Important Notes:
- Files are limited to 400KB in size and 400,000 characters
- write command completely replaces file content
- str_replace requires exact and unique match of old_str
- Undo history is maintained for recent changes per file")]
    async fn text_editor(
        &self,
        Parameters(TextEditorParams {
            command,
            path,
            file_text,
            old_str,
            new_str,
        }): Parameters<TextEditorParams>,
    ) -> Result<CallToolResult, McpError> {
        // Validate and resolve the path
        let resolved_path = self.resolve_path(&path)?;
        let path_str = resolved_path.to_string_lossy().to_string();

        match command.as_str() {
            "view" => self.text_editor.view(path_str).await,
            "write" => {
                let file_text = file_text.ok_or_else(|| {
                    McpError::invalid_params("file_text is required for write command", None)
                })?;
                self.text_editor.write(path_str, file_text).await
            }
            "str_replace" => {
                let old_str = old_str.ok_or_else(|| {
                    McpError::invalid_params("old_str is required for str_replace command", None)
                })?;
                let new_str = new_str.ok_or_else(|| {
                    McpError::invalid_params("new_str is required for str_replace command", None)
                })?;
                self.text_editor
                    .str_replace(path_str, old_str, new_str)
                    .await
            }
            "undo_edit" => self.text_editor.undo_edit(path_str).await,
            _ => Err(McpError::invalid_params(
                "Unknown command. Allowed commands are: view, write, str_replace, undo_edit",
                None,
            )),
        }
    }

    // Shell Tool
    #[tool(description = "Execute shell commands on the system")]
    async fn shell(
        &self,
        Parameters(ShellParams { command }): Parameters<ShellParams>,
    ) -> Result<CallToolResult, McpError> {
        self.shell.execute(command).await
    }

    // Screen Capture Tools
    #[tool(
        description = "List all available window titles that can be used with screen_capture.\nReturns a list of window titles that can be used with the window_title parameter\nof the screen_capture tool."
    )]
    async fn list_windows(&self) -> Result<CallToolResult, McpError> {
        self.screen_capture.list_windows().await
    }

    #[tool(
        description = "Capture a screenshot of a specified display or window.\nYou can capture either:\n1. A full display (monitor) using the display parameter\n2. A specific window by its title using the window_title parameter\n\nOnly one of display or window_title should be specified."
    )]
    async fn screen_capture(
        &self,
        Parameters(ScreenCaptureParams {
            display,
            window_title,
        }): Parameters<ScreenCaptureParams>,
    ) -> Result<CallToolResult, McpError> {
        self.screen_capture.capture(display, window_title).await
    }

    // Image Processor Tool
    #[tool(
        description = "Process an image file from disk. The image will be:\n1. Resized if larger than max width while maintaining aspect ratio\n2. Optionally resized further by 1/2 or 1/4 to reduce file size\n3. Preserved in original format (JPEG stays JPEG, PNG stays PNG) for optimal compression\n4. Returned as base64 encoded data\n\nThis allows processing image files for use in the conversation."
    )]
    async fn image_processor(
        &self,
        Parameters(ImageProcessorParams { path, resize }): Parameters<ImageProcessorParams>,
    ) -> Result<CallToolResult, McpError> {
        // Validate and resolve the path
        let resolved_path = self.resolve_path(&path)?;
        let path_str = resolved_path.to_string_lossy().to_string();

        self.image_processor.process(path_str, resize).await
    }

    // Workflow Tools
    #[tool(description = "Workflow Tool: Guiding Complex Problem-Solving

Manages multi-step problem-solving processes with support for sequential progression, branching paths, and step revisions. This tool helps structure reasoning, explore alternatives, and adapt approaches as understanding evolves.

When to Use:
- Deconstruct complex problems into manageable steps
- Plan and design iteratively with potential revisions
- Explore multiple solution paths via branching
- Perform in-depth analysis with course correction
- Handle problems with unclear scope
- Maintain long-term context across interactions

Key Features:
- Sequential Progression: Steps are tracked in order
- Dynamic total_steps: Can be adjusted as workflow progresses
- Branching: Create and switch between alternative solution paths
- Step Revision: Mark steps that update or correct prior steps
- Context Preservation: Workflow state maintained across calls

Parameters:
- step_description (required): Detailed description of what this step accomplishes
- step_number (required): Current position in workflow sequence (≥1)
- total_steps (required): Current best estimate of total steps needed
- next_step_needed (required): True if another step will immediately follow
- is_step_revision (optional): True if this step revises a previous step
- revises_step (optional): Step number being revised if is_step_revision is true
- branch_from_step (optional): Step number from which new branch originates
- branch_id (optional): Unique identifier for the branch
- needs_more_steps (optional): True if more steps needed for overall problem")]
    async fn workflow(
        &self,
        Parameters(WorkflowParams {
            step_description,
            step_number,
            total_steps,
            next_step_needed,
            is_step_revision,
            revises_step,
            branch_from_step,
            branch_id,
            needs_more_steps,
        }): Parameters<WorkflowParams>,
    ) -> Result<CallToolResult, McpError> {
        use workflow::WorkflowStep;

        let step = WorkflowStep {
            step_description,
            step_number,
            total_steps,
            next_step_needed,
            is_step_revision,
            revises_step,
            branch_from_step,
            branch_id,
            needs_more_steps,
        };

        self.workflow.execute_step(step).await
    }
}

#[tool_handler]
impl ServerHandler for Developer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides developer tools including text editing, shell command execution, screen capture capabilities, and workflow management. Use the text_editor tools to view and modify files, shell tools to execute commands, screen_capture tools to take screenshots, and workflow tools to manage multi-step problem-solving processes with branching and revision support.".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                self._create_resource_text("file://workspace", "workspace"),
                self._create_resource_text("shell://history", "shell-history"),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "file://workspace" => {
                let workspace_info =
                    "Developer workspace with text editing, shell, and screen capture tools";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(workspace_info, uri)],
                })
            }
            "shell://history" => {
                let history = "Shell command history placeholder";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(history, uri)],
                })
            }
            _ => Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({
                    "uri": uri
                })),
            )),
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![Prompt::new(
                "developer_workflow",
                Some("A prompt for common developer workflows"),
                Some(vec![PromptArgument {
                    name: "task".to_string(),
                    description: Some("The development task to perform".to_string()),
                    required: Some(true),
                }]),
            )],
        })
    }

    async fn get_prompt(
        &self,
        GetPromptRequestParam { name, arguments }: GetPromptRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        match name.as_str() {
            "developer_workflow" => {
                let task = arguments
                    .and_then(|json| json.get("task")?.as_str().map(|s| s.to_string()))
                    .ok_or_else(|| {
                        McpError::invalid_params("No task provided to developer_workflow", None)
                    })?;

                let prompt = format!(
                    "You are a developer assistant. Help with this task: '{task}'. You have access to text editing, shell commands, and screen capture tools."
                );
                Ok(GetPromptResult {
                    description: None,
                    messages: vec![PromptMessage {
                        role: PromptMessageRole::User,
                        content: PromptMessageContent::text(prompt),
                    }],
                })
            }
            _ => Err(McpError::invalid_params("prompt not found", None)),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        if let Some(http_request_part) = context.extensions.get::<axum::http::request::Parts>() {
            let initialize_headers = &http_request_part.headers;
            let initialize_uri = &http_request_part.uri;
            tracing::info!(?initialize_headers, %initialize_uri, "initialize from http server");
        }
        Ok(self.get_info())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_developer_creation() {
        let _developer = Developer::new();
        // Just ensure it can be created without panicking
        assert!(true);
    }

    #[test]
    fn test_get_info() {
        let developer = Developer::new();
        let info = developer.get_info();
        assert_eq!(info.protocol_version, ProtocolVersion::V_2024_11_05);
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.prompts.is_some());
        assert!(info.capabilities.resources.is_some());
    }

    #[test]
    fn test_resolve_path_absolute() {
        let developer = Developer::new();

        if cfg!(windows) {
            let result = developer.resolve_path("C:\\test\\file.txt");
            assert!(result.is_ok());
            let path = result.unwrap();
            assert_eq!(path.to_string_lossy(), "C:\\test\\file.txt");
        } else {
            let result = developer.resolve_path("/test/file.txt");
            assert!(result.is_ok());
            let path = result.unwrap();
            assert_eq!(path.to_string_lossy(), "/test/file.txt");
        }
    }

    #[test]
    fn test_resolve_path_relative_error() {
        let developer = Developer::new();
        let result = developer.resolve_path("relative/path.txt");
        assert!(result.is_err());
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("not an absolute path"));
            assert!(error_msg.contains("did you possibly mean"));
        }
    }

    #[test]
    fn test_expand_path() {
        if cfg!(windows) {
            // Test Windows path expansion
            let path = "%USERPROFILE%\\test";
            let expanded = expand_path(path);
            assert!(!expanded.contains("%USERPROFILE%"));
        } else {
            // Test Unix path expansion
            let path = "~/test";
            let expanded = expand_path(path);
            assert!(!expanded.starts_with('~'));
        }
    }

    #[test]
    fn test_is_absolute_path() {
        if cfg!(windows) {
            assert!(is_absolute_path("C:\\test"));
            assert!(is_absolute_path("\\\\server\\share"));
            assert!(!is_absolute_path("relative\\path"));
        } else {
            assert!(is_absolute_path("/absolute/path"));
            assert!(!is_absolute_path("relative/path"));
        }
    }

    #[test]
    fn test_normalize_line_endings() {
        let input = "line1\r\nline2\nline3";
        let normalized = normalize_line_endings(input);

        if cfg!(windows) {
            assert_eq!(normalized, "line1\r\nline2\r\nline3");
        } else {
            assert_eq!(normalized, "line1\nline2\nline3");
        }
    }

    // Note: RequestContext tests are complex due to the structure requirements
    // These would need proper setup in integration tests
}
