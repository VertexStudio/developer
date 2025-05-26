use rmcp::{
    Error as McpError, RoleServer, ServerHandler, model::*, schemars, service::RequestContext, tool,
};
use serde_json::json;

pub mod image_processor;
pub mod lang;
pub mod screen_capture;
pub mod shell;
pub mod text_editor;

pub use image_processor::ImageProcessor;
pub use screen_capture::ScreenCapture;
pub use shell::Shell;
pub use text_editor::TextEditor;

#[derive(Clone)]
pub struct Developer {
    text_editor: TextEditor,
    shell: Shell,
    screen_capture: ScreenCapture,
    image_processor: ImageProcessor,
}

#[tool(tool_box)]
impl Developer {
    pub fn new() -> Self {
        Self {
            text_editor: TextEditor::new(),
            shell: Shell::new(),
            screen_capture: ScreenCapture::new(),
            image_processor: ImageProcessor::new(),
        }
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    fn get_shell_description() -> &'static str {
        match std::env::consts::OS {
            "windows" => include_str!("descriptions/shell_windows.md"),
            _ => include_str!("descriptions/shell_unix.md"),
        }
    }

    // Text Editor Tool
    #[tool(description = include_str!("descriptions/text_editor.md"))]
    async fn text_editor(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Allowed options are: `view`, `write`, `str_replace`, `undo_edit`."
        )]
        command: String,
        #[tool(param)]
        #[schemars(
            description = "Absolute path to file or directory, e.g. `/repo/file.py` or `/repo`."
        )]
        path: String,
        #[tool(param)]
        #[schemars(description = "Content to write to the file (required for write command)")]
        file_text: Option<String>,
        #[tool(param)]
        #[schemars(description = "String to replace (required for str_replace command)")]
        old_str: Option<String>,
        #[tool(param)]
        #[schemars(description = "New string to replace with (required for str_replace command)")]
        new_str: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        match command.as_str() {
            "view" => self.text_editor.view(path).await,
            "write" => {
                let file_text = file_text.ok_or_else(|| {
                    McpError::invalid_params("file_text is required for write command", None)
                })?;
                self.text_editor.write(path, file_text).await
            }
            "str_replace" => {
                let old_str = old_str.ok_or_else(|| {
                    McpError::invalid_params("old_str is required for str_replace command", None)
                })?;
                let new_str = new_str.ok_or_else(|| {
                    McpError::invalid_params("new_str is required for str_replace command", None)
                })?;
                self.text_editor.str_replace(path, old_str, new_str).await
            }
            "undo_edit" => self.text_editor.undo_edit(path).await,
            _ => Err(McpError::invalid_params(
                "Unknown command. Allowed commands are: view, write, str_replace, undo_edit",
                None,
            )),
        }
    }

    // Shell Tool
    #[tool(description = Self::get_shell_description())]
    async fn shell(
        &self,
        #[tool(param)]
        #[schemars(description = "Command to execute")]
        command: String,
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
        #[tool(param)]
        #[schemars(description = "The display number to capture (0 is main display)")]
        display: Option<i32>,
        #[tool(param)]
        #[schemars(
            description = "Optional: the exact title of the window to capture. use the list_windows tool to find the available windows."
        )]
        window_title: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        self.screen_capture.capture(display, window_title).await
    }

    // Image Processor Tool
    #[tool(
        description = "Process an image file from disk. The image will be:\n1. Resized if larger than max width while maintaining aspect ratio\n2. Converted to PNG format\n3. Returned as base64 encoded data\n\nThis allows processing image files for use in the conversation."
    )]
    async fn image_processor(
        &self,
        #[tool(param)]
        #[schemars(description = "Absolute path to the image file to process")]
        path: String,
    ) -> Result<CallToolResult, McpError> {
        self.image_processor.process(path).await
    }
}

#[tool(tool_box)]
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
            instructions: Some("This server provides developer tools including text editing, shell command execution, and screen capture capabilities. Use the text_editor tools to view and modify files, shell tools to execute commands, and screen_capture tools to take screenshots or record the screen.".to_string()),
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

    // Note: RequestContext tests are complex due to the structure requirements
    // These would need proper setup in integration tests
}
