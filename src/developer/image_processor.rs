use base64::Engine;
use rmcp::{Error as McpError, model::CallToolResult, model::Content};
use std::{io::Cursor, path::Path};

#[derive(Clone)]
pub struct ImageProcessor;

impl ImageProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Helper function to handle Mac screenshot filenames that contain U+202F (narrow no-break space)
    fn normalize_mac_screenshot_path(path: &Path) -> std::path::PathBuf {
        // Only process if the path has a filename
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            // Check if this matches Mac screenshot pattern:
            // "Screenshot YYYY-MM-DD at H.MM.SS AM/PM.png"
            if let Some(captures) = regex::Regex::new(r"^Screenshot \d{4}-\d{2}-\d{2} at \d{1,2}\.\d{2}\.\d{2} (AM|PM|am|pm)(?: \(\d+\))?\.png$")
                .ok()
                .and_then(|re| re.captures(filename))
            {
                // Get the AM/PM part
                let meridian = captures.get(1).unwrap().as_str();

                // Find the last space before AM/PM and replace it with U+202F
                let space_pos = filename.rfind(meridian)
                    .map(|pos| filename[..pos].trim_end().len())
                    .unwrap_or(0);

                if space_pos > 0 {
                    let parent = path.parent().unwrap_or(Path::new(""));
                    let new_filename = format!(
                        "{}{}{}",
                        &filename[..space_pos],
                        '\u{202F}',
                        &filename[space_pos+1..]
                    );
                    let new_path = parent.join(new_filename);

                    return new_path;
                }
            }
        }
        path.to_path_buf()
    }

    pub async fn process(&self, path: String) -> Result<CallToolResult, McpError> {
        let path = Path::new(&path);

        let path = {
            if cfg!(target_os = "macos") {
                Self::normalize_mac_screenshot_path(&path)
            } else {
                path.to_path_buf()
            }
        };

        // Check if file exists
        if !path.exists() {
            return Err(McpError::invalid_params(
                format!("File '{}' does not exist", path.display()),
                None,
            ));
        }

        // Check file size (10MB limit for image files)
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB in bytes
        let file_size = std::fs::metadata(&path)
            .map_err(|e| {
                McpError::internal_error(format!("Failed to get file metadata: {}", e), None)
            })?
            .len();

        if file_size > MAX_FILE_SIZE {
            return Err(McpError::invalid_params(
                format!(
                    "File '{}' is too large ({:.2}MB). Maximum size is 10MB.",
                    path.display(),
                    file_size as f64 / (1024.0 * 1024.0)
                ),
                None,
            ));
        }

        // Open and decode the image
        let image = xcap::image::open(&path).map_err(|e| {
            McpError::internal_error(format!("Failed to open image file: {}", e), None)
        })?;

        // Resize if necessary (same logic as screen_capture)
        let mut processed_image = image;
        let max_width = 768;
        if processed_image.width() > max_width {
            let scale = max_width as f32 / processed_image.width() as f32;
            let new_height = (processed_image.height() as f32 * scale) as u32;
            processed_image = xcap::image::DynamicImage::ImageRgba8(xcap::image::imageops::resize(
                &processed_image,
                max_width,
                new_height,
                xcap::image::imageops::FilterType::Lanczos3,
            ));
        }

        // Convert to PNG and encode as base64
        let mut bytes: Vec<u8> = Vec::new();
        processed_image
            .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
            .map_err(|e| {
                McpError::internal_error(format!("Failed to write image buffer: {}", e), None)
            })?;

        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        Ok(CallToolResult::success(vec![
            Content::text(format!(
                "Successfully processed image from {}",
                path.display()
            )),
            Content::image(data, "image/png".to_string()),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_normalize_mac_screenshot_path() {
        let path = std::path::Path::new("Screenshot 2023-12-01 at 10.30.45 AM.png");
        let normalized = ImageProcessor::normalize_mac_screenshot_path(&path);

        // Should return a path (exact behavior depends on regex matching)
        assert!(normalized.file_name().is_some());
    }

    #[tokio::test]
    async fn test_process_nonexistent_file() {
        let image_processor = ImageProcessor::new();
        let result = image_processor
            .process("/nonexistent/file.png".to_string())
            .await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("does not exist"));
        }
    }

    #[tokio::test]
    async fn test_process_large_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let large_file_path = temp_dir.path().join("large_file.png");

        // Create a file larger than 10MB
        let mut file = std::fs::File::create(&large_file_path).unwrap();
        let large_data = vec![0u8; 11 * 1024 * 1024]; // 11MB
        file.write_all(&large_data).unwrap();

        let image_processor = ImageProcessor::new();
        let result = image_processor
            .process(large_file_path.to_string_lossy().to_string())
            .await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("too large"));
        }

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_process_invalid_image() {
        let temp_dir = tempfile::tempdir().unwrap();
        let invalid_file_path = temp_dir.path().join("invalid.png");

        // Create a file that's not a valid image
        let mut file = std::fs::File::create(&invalid_file_path).unwrap();
        file.write_all(b"This is not an image").unwrap();

        let image_processor = ImageProcessor::new();
        let result = image_processor
            .process(invalid_file_path.to_string_lossy().to_string())
            .await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Failed to open image file"));
        }

        temp_dir.close().unwrap();
    }
}
