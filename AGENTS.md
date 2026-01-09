# Project Context: proxmox-mcp-rs

## Project Overview
This project aims to be a **Rust implementation of a Proxmox MCP (Model Context Protocol) server**. It is designed to interface with a Proxmox virtualization environment, likely to expose its resources or control capabilities via the MCP standard.

**Current Status:** Functional Rust implementation with core MCP tools, configuration file support, and comprehensive unit tests.
*   **Language:** Rust.
*   **Transport:** Stdio (JSON-RPC 2.0) and HTTP (SSE/POST).

## Key Files
*   `README.md`: User documentation and tool list.
*   `Cargo.toml`: Project dependencies.
*   `src/main.rs`: Entry point and argument parsing.
*   `src/proxmox.rs`: Proxmox API client.
*   `src/mcp.rs`: MCP Server implementation.
*   `src/http_server.rs`: HTTP Server implementation (SSE/POST).
*   `src/settings.rs`: Configuration management.
*   `src/tests.rs`: Unit tests with WireMock.
*   `.github/workflows/ci.yml`: GitHub Actions CI workflow.
*   `.github/CONTRIBUTING.md`: Contribution guidelines.

## Building and Running
1.  **Build:** `cargo build --release`
2.  **Run (Stdio):** `./target/release/proxmox-mcp-rs --host <host> --user <user> --password <pw>`
3.  **Run (HTTP):** `./target/release/proxmox-mcp-rs --server-type http --http-host 0.0.0.0 --http-port 3000 --host <host> ...`
4.  **Test:** `cargo test`

### TODOs
*   Expand toolset (backups, snapshots, network management).
*   Implement better error reporting for MCP clients.
*   Refine async task handling for long-running Proxmox operations.


## Development Conventions
*   **Language:** Rust
*   **Style:** Standard Rust formatting (`rustfmt`) and linting (`clippy`) are expected.
*   **Documentation:** All Proxmox functions and tools must be documented in the `README.md`.
*   **Testing:** Every Proxmox function and MCP tool must have corresponding unit tests in `src/tests.rs` (or relevant module).

