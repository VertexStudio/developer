use base64::Engine;
use rmcp::{
    Error as McpError,
    model::CallToolResult,
    model::{Content, Role},
};
use std::io::Cursor;
use xcap::{Monitor, Window};

#[derive(Clone)]
pub struct ScreenCapture;

impl ScreenCapture {
    pub fn new() -> Self {
        Self
    }

    pub async fn capture(
        &self,
        display: Option<i32>,
        window_title: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let mut image = if let Some(window_title) = window_title {
            // Try to find and capture the specified window
            let windows = Window::all().map_err(|_| {
                McpError::internal_error("Failed to list windows".to_string(), None)
            })?;

            let window = windows
                .into_iter()
                .find(|w| w.title() == window_title)
                .ok_or_else(|| {
                    McpError::invalid_params(
                        format!("No window found with title '{}'", window_title),
                        None,
                    )
                })?;

            window.capture_image().map_err(|e| {
                McpError::internal_error(
                    format!("Failed to capture window '{}': {}", window_title, e),
                    None,
                )
            })?
        } else {
            // Default to display capture if no window title is specified
            let display_num = display.unwrap_or(0) as usize;

            let monitors = Monitor::all().map_err(|_| {
                McpError::internal_error("Failed to access monitors".to_string(), None)
            })?;
            let monitor = monitors.get(display_num).ok_or_else(|| {
                McpError::invalid_params(
                    format!(
                        "{} was not an available monitor, {} found.",
                        display_num,
                        monitors.len()
                    ),
                    None,
                )
            })?;

            monitor.capture_image().map_err(|e| {
                McpError::internal_error(
                    format!("Failed to capture display {}: {}", display_num, e),
                    None,
                )
            })?
        };

        // Resize the image to a reasonable width while maintaining aspect ratio
        let max_width = 768;
        if image.width() > max_width {
            let scale = max_width as f32 / image.width() as f32;
            let new_height = (image.height() as f32 * scale) as u32;
            image = xcap::image::imageops::resize(
                &image,
                max_width,
                new_height,
                xcap::image::imageops::FilterType::Lanczos3,
            )
        };

        let mut bytes: Vec<u8> = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
            .map_err(|e| {
                McpError::internal_error(format!("Failed to write image buffer {}", e), None)
            })?;

        // Convert to base64
        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        Ok(CallToolResult::success(vec![
            Content::text("Screenshot captured").with_audience(vec![Role::Assistant]),
            Content::image(data, "image/png").with_priority(0.0),
        ]))
    }

    pub async fn list_windows(&self) -> Result<CallToolResult, McpError> {
        let windows = Window::all()
            .map_err(|_| McpError::internal_error("Failed to list windows".to_string(), None))?;

        let mut window_info: Vec<String> = Vec::new();

        for window in windows.iter() {
            // Skip minimized windows as they can't be captured anyway
            if window.is_minimized() {
                continue;
            }

            let title = window.title();

            // Only add non-empty titles
            if !title.is_empty() && title != "<No Title>" {
                window_info.push(title.to_string());
            }
        }

        let content = if window_info.is_empty() {
            "No windows found".to_string()
        } else {
            format!("Available windows:\n{}", window_info.join("\n"))
        };

        Ok(CallToolResult::success(vec![
            Content::text(content.clone()).with_audience(vec![Role::Assistant]),
            Content::text(content)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_windows() {
        let screen_capture = ScreenCapture::new();
        let result = screen_capture.list_windows().await;
        assert!(result.is_ok());
        let call_result = result.unwrap();
        assert!(!call_result.content.is_empty());

        // Print the window list to see what's detected
        // Re-create the window list for printing
        let windows = Window::all().unwrap();
        let mut window_titles: Vec<String> = Vec::new();

        for window in windows.iter() {
            if window.is_minimized() {
                continue;
            }
            let title = window.title();
            if !title.is_empty() && title != "<No Title>" {
                window_titles.push(title.to_string());
            }
        }

        println!("=== WINDOW LIST ===");
        if window_titles.is_empty() {
            println!("No windows found");
        } else {
            println!("Available windows:");
            for title in window_titles {
                println!("{}", title);
            }
        }
        println!("=== END WINDOW LIST ===");

        // Check that the content includes window information
        assert!(!call_result.content.is_empty());
    }

    #[tokio::test]
    async fn test_capture_default_display() {
        let screen_capture = ScreenCapture::new();
        let result = screen_capture.capture(None, None).await;
        // This test might fail in CI environments without displays, so we just check it doesn't panic
        // In a real environment with displays, this should succeed
        match result {
            Ok(call_result) => {
                assert!(!call_result.content.is_empty());
                // Should have both text and image content
                assert!(call_result.content.len() >= 2);
            }
            Err(_) => {
                // Expected in headless environments
            }
        }
    }

    #[tokio::test]
    async fn test_capture_invalid_window() {
        let screen_capture = ScreenCapture::new();
        let result = screen_capture
            .capture(None, Some("NonExistentWindow12345".to_string()))
            .await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("No window found"));
        }
    }
}
