# Developer MCP Server

A general purpose Model Context Protocol (MCP) server that provides comprehensive developer tools for file editing, shell command execution, and screen capture capabilities. Built using the [rmcp](https://github.com/modelcontextprotocol/rmcp) crate.

## 🚀 Features

### 📝 Text Editor
- **View files** with language detection for markdown formatting
- **Write/create files** with automatic directory creation
- **String replacement** with precise matching
- **Undo functionality** with edit history
- **File size protection** (400KB limit for text files)

### 🖥️ Shell Integration
- **Cross-platform command execution** (PowerShell on Windows, bash/zsh on Unix)
- **Combined stdout/stderr output** as it appears in terminal
- **Output size protection** (400KB limit)
- **Platform-specific optimizations**

### 📸 Screen Capture
- **Full display screenshots** with monitor selection
- **Window-specific capture** by title
- **Automatic image optimization** (768px max width)
- **Base64 encoded PNG output**

### 🖼️ Image Processing
- **Image file processing** from disk
- **Automatic resizing** while maintaining aspect ratio
- **Format conversion** to PNG
- **macOS screenshot filename handling**

### 🔒 Security Features
- **Gitignore integration** - respects `.gitignore` patterns for file access control
- **Path validation** - requires absolute paths to prevent directory traversal
- **File size limits** - prevents memory exhaustion attacks
- **Access pattern filtering** - blocks access to sensitive files

## 📋 Requirements

- **Rust** 1.70+ (for building from source)
- **Claude Desktop** or compatible MCP client
- **Operating System**: macOS, Linux, or Windows

## 🛠️ Installation

### Option 1: Build from Source (Recommended)

1. **Clone the repository:**
   ```bash
   git clone git@github.com:VertexStudio/developer.git
   cd developer
   ```

2. **Build the project:**
   ```bash
   cargo build --release
   ```

3. **The binary will be available at:**
   ```
   target/release/developer
   ```

### Option 2: Development Build

For development/testing purposes:
```bash
cargo build
# Binary at: target/debug/developer
```

## ⚙️ Configuration

### Claude Desktop Setup

1. **Open Claude Desktop configuration file:**

   **macOS/Linux:**
   ```bash
   code ~/Library/Application\ Support/Claude/claude_desktop_config.json
   ```

   **Windows:**
   ```bash
   code %APPDATA%\Claude\claude_desktop_config.json
   ```

2. **Add the developer server configuration:**

   ```json
   {
     "mcpServers": {
       "developer": {
         "command": "/path/to/your/developer/target/release/developer",
         "args": []
       }
     }
   }
   ```

   **Example configurations:**

   **Development build:**
   ```json
   {
     "mcpServers": {
       "developer": {
         "command": "/Users/rozgo/vertex/developer/target/debug/developer",
         "args": []
       }
     }
   }
   ```

   **Production build:**
   ```json
   {
     "mcpServers": {
       "developer": {
         "command": "/Users/rozgo/vertex/developer/target/release/developer",
         "args": []
       }
     }
   }
   ```

3. **Restart Claude Desktop** to load the new configuration.

### File Access Control (Optional)

Create a `.gitignore` file in your working directory to control which files the server can access:

```gitignore
# Sensitive files
.env
.env.*
secrets.*
private/
*.key
*.pem

# Build artifacts
target/
node_modules/
dist/
```

The server will automatically respect these patterns and block access to matching files.

## 🎯 Usage Examples

Once configured, you can use these tools directly in Claude Desktop:

### Text Editing
```
"Can you view the contents of /path/to/my/file.rs?"

"Please create a new file at /path/to/hello.py with a simple hello world script"

"Replace the line 'old_function()' with 'new_function()' in /path/to/main.rs"

"Undo the last edit to /path/to/main.rs"
```

### Shell Commands
```
"Run 'ls -la' to show me the current directory contents"

"Execute 'cargo test' to run the test suite"

"Run 'git status' to check the repository status"
```

### Screen Capture
```
"Take a screenshot of my main display"

"Capture a screenshot of the window titled 'VS Code'"

"Show me what windows are available for capture"
```

### Image Processing
```
"Process the image at /path/to/screenshot.png and show it to me"

"Load and display the image from /Users/me/Desktop/diagram.jpg"
```

## 🏗️ Architecture

```
Developer MCP Server
├── Text Editor      → File viewing, editing, string replacement, undo
├── Shell           → Cross-platform command execution  
├── Screen Capture  → Display and window screenshots
├── Image Processor → File-based image processing
└── Security Layer  → Gitignore integration, path validation
```

## 🔧 Tool Reference

### text_editor
- **Commands:** `view`, `write`, `str_replace`, `undo_edit`
- **Parameters:** `path` (required), `file_text`, `old_str`, `new_str`
- **Limits:** 400KB file size, absolute paths only

### shell  
- **Parameters:** `command` (required)
- **Features:** Platform detection, output redirection, size limits
- **Limits:** 400KB output size

### screen_capture
- **Parameters:** `display` (optional), `window_title` (optional)
- **Output:** Base64 PNG image, 768px max width

### list_windows
- **Parameters:** None
- **Output:** List of capturable window titles

### image_processor
- **Parameters:** `path` (required)
- **Features:** Auto-resize, format conversion, macOS compatibility
- **Limits:** 10MB file size

## 🐛 Troubleshooting

### Common Issues

**"Tool not found" errors:**
- Ensure the binary path in your configuration is correct
- Verify the binary exists and is executable
- Check Claude Desktop logs for detailed error messages

**"File access denied" errors:**
- Check if the file is blocked by `.gitignore` patterns
- Ensure you're using absolute paths (not relative paths)
- Verify file permissions

**"Command failed" errors:**
- Ensure the command exists and is in your system PATH
- Check if the command requires special permissions
- Verify the command syntax for your operating system

### Debug Mode

Build with debug info for troubleshooting:
```bash
cargo build
# Use target/debug/developer in your configuration
```

### MCP Inspector

Use the official MCP inspector to debug and test tools:
```bash
npx @modelcontextprotocol/inspector target/debug/developer
```

This will open a web interface where you can:
- Inspect available tools and their schemas
- Test tool calls interactively
- Debug server responses
- Validate MCP protocol compliance

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Submit a pull request

## 📝 License

[MIT](LICENSE)

## 🔗 Related Projects

- [Model Context Protocol](https://modelcontextprotocol.io/) - The protocol specification
- [Claude Desktop](https://claude.ai/download) - Official Claude desktop application
- [MCP Servers](https://github.com/modelcontextprotocol/servers) - Official MCP server implementations
