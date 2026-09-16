# Aaroneous MCP Client Configuration Guide

Project Aaroneous exposes a sovereign Machine-Native Model Context Protocol (MCP) server supporting stdio, HTTP POST (`/mcp`), and Server-Sent Events (`/sse`).

## Endpoints
- **HTTP JSON-RPC**: `http://127.0.0.1:8766/mcp`
- **SSE Stream**: `http://127.0.0.1:8766/sse`
- **Health Check**: `http://127.0.0.1:8766/health`
- **Authentication**: `Authorization: Bearer <AARONEOUS_API_KEY>`

## Quick Setup

### 1. Claude Desktop
Copy the contents of `claude_desktop_config.json` into `%APPDATA%\Claude\claude_desktop_config.json`.

### 2. Cursor IDE
Add the configuration from `cursor_mcp.json` to Cursor Settings -> MCP Servers.

### 3. VS Code / Windsurf
Add the configuration from `vscode_mcp.json` to `.vscode/mcp.json` or Global User Settings.

## Available Sovereign MCP Tools
- `read_code`: Reads source files safely with sandbox path containment.
- `search_code`: Fast text pattern search across the workspace repository.
- `list_files`: Lists directory contents and file metadata.
- `submit_intent`: Submits sovereign natural language instructions to Orchestrator & the Specialist Federation.
- `dispatch_specialist`: Direct point-to-point invocation of a specific specialist (Orchestrator, Synthesizer, Fabricator, Sentinel, Archivist, Router, Aligner, Perceiver, Presenter).
- `signal_wasms`: Dispatches broadcast signals across the federation bus.
