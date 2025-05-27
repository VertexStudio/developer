# Rig Integration Example

This example demonstrates how to integrate the Rig AI framework with Model Context Protocol (MCP) servers to create an intelligent chatbot that can dynamically use tools from MCP servers.

## Features

- **Dynamic Tool Integration**: Automatically discovers and uses tools from configured MCP servers
- **Vector Search**: Uses OpenAI embeddings to find relevant tools based on semantic similarity
- **AI Agent**: Uses OpenAI GPT-4o model with dynamic tool calling capabilities
- **Flexible Configuration**: Easy TOML-based configuration for MCP servers

## Setup

1. **Configure API Key**: Set your OpenAI API key as an environment variable:

   ```bash
   export OPENAI_API_KEY="your-openai-api-key"
   ```

2. **Configure MCP Servers**: The example is pre-configured to use the developer MCP server from this workspace. You can add additional MCP servers to the `config.toml` file:
   ```toml
   [[mcp.server]]
   name = "developer"
   protocol = "stdio"
   command = "cargo"
   args = ["run", "--bin", "developer"]
   
   # Optional: Add other MCP servers
   [[mcp.server]]
   name = "filesystem"
   protocol = "stdio"
   command = "npx"
   args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/directory"]
   ```

3. **MCP Server Dependencies**: The developer MCP server is built into this workspace, so no additional installation is needed. For other MCP servers:
   ```bash
   # For external MCP servers, install as needed
   npm install -g @modelcontextprotocol/server-filesystem
   ```

## Running

From the workspace root:

```bash
cargo run -p rig
```

## How It Works

1. **Initialization**: The application reads the configuration and starts all configured MCP servers
2. **Tool Discovery**: It retrieves all available tools from the MCP servers and creates embeddings for them
3. **Vector Store**: Tools are indexed in an in-memory vector store using OpenAI embeddings
4. **Agent Creation**: An OpenAI GPT-4o agent is created with dynamic tool access
5. **Interactive Chat**: The CLI chatbot allows you to interact with the agent, which can dynamically select and use the most relevant tools

## Architecture

- **`main.rs`**: Application entry point and setup
- **`config/`**: Configuration management for MCP servers
- **`mcp_adaptor.rs`**: Adapter for integrating MCP tools with Rig
- **`chat.rs`**: CLI chatbot implementation

## Example Usage

Once running, you can ask the chatbot to perform various tasks that will be handled by the developer MCP server tools:

- "Show me the contents of src/main.rs" (text editor)
- "Create a new file called hello.py with a simple hello world script" (text editor)
- "Run 'cargo build' to build the project" (shell)
- "Take a screenshot of my current desktop" (screen capture)
- "List all available windows for capture" (screen capture)
- "Replace the function name 'old_func' with 'new_func' in src/lib.rs" (text editor)
- "Execute 'git status' to check repository status" (shell)

The agent will automatically select the most relevant tools based on your request and execute them to provide helpful responses. The developer MCP server provides comprehensive development tools including:

- **Text Editor**: View, create, edit files with undo support
- **Shell**: Execute commands cross-platform
- **Screen Capture**: Take screenshots of displays or windows
- **Image Processing**: Process and display images
- **Workflow Management**: Multi-step problem solving with branching 