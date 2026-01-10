# Project Context: proxmox-mcp-rs

## Project Overview
This project aims to be a **Rust implementation of a Proxmox MCP (Model Context Protocol) server**. It is designed to interface with a Proxmox virtualization environment, likely to expose its resources or control capabilities via the MCP standard.

**Current Status:** Functional Rust implementation with core MCP tools, configuration file support, and comprehensive unit tests.
**Implemented Features:**
*   **Authentication:** User/Password and API Token (with HTTP middleware support).
*   **Core Management:** Nodes, VMs, Containers (Lifecycle: Start, Stop, Clone, Migrate, etc.).
*   **Backup & Restore:** Full vzdump support (create, list, restore).
*   **Storage & Network:** List storage domains, ISOs, and network interfaces.
*   **Cluster Management:** Cluster status, logs, and firewall rules.
*   **Monitoring:** Async task tracking (UPID) and resource reading (MCP Resources).
*   **Language:** Rust.
*   **Transport:** Stdio (JSON-RPC 2.0) and HTTP (SSE/POST) with optional authentication.

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
*   Refine async task handling for long-running Proxmox operations.
*   Implement Console/Serial Terminal Access.
*   Enhance error reporting for MCP clients.


## Development Conventions
*   **Language:** Rust
*   **Style:** Standard Rust formatting (`cargo fmt`) and linting (`cargo clippy`) are expected.
*   **Post-Implementation:** Always run `cargo fmt` and `cargo clippy -- -D warnings` after adding new functions to ensure code quality and adherence to standards.
*   **CI Checks:** You can also run `task test:ci` to run all formatting, linting, and tests at once.
*   **Documentation:** All Proxmox functions and tools must be documented in the `README.md`.
*   **Testing:** Every Proxmox function and MCP tool must have corresponding unit tests in `src/tests.rs` (or relevant module).
*   **Versioning:** When asked to create a new git tag, always update the version in `Cargo.toml` to match the new tag version.

## Live Testing Workflow
When asked to test new functions since the last tag against a *live, connected* Proxmox server:

1.  **Identify Changes:** Run `git diff <last_tag> HEAD -- src/mcp.rs` to see which new tools were added.
2.  **Verify Config:** Ensure a `config.toml` exists or environment variables are set to connect to a real Proxmox instance.
3.  **Build:** Run `cargo build --release`.
4.  **Script Interaction:** Create a Python script (`test_mcp_live.py`) to spawn the binary and send JSON-RPC requests to the stdio transport.
    *   *Template Script:*
        ```python
        import subprocess, json, sys
        def rpc(method, params=None, id=1): return {"jsonrpc": "2.0", "method": method, "params": params, "id": id}
        cmd = ["./target/release/proxmox-mcp-rs"]
        p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=sys.stderr, text=True, bufsize=0)
        
        # 1. Init
        p.stdin.write(json.dumps(rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}})) + "\n")
        print(p.stdout.readline()) # Read Init Response

        # 2. Call Tool (Example)
        # p.stdin.write(json.dumps(rpc("tools/call", {"name": "new_tool_name", "arguments": {...}})) + "\n")
        # print(p.stdout.readline())
        
        p.terminate()
        ```
5.  **Execute:** Run the script and verify the JSON-RPC responses indicate success (no errors, expected data returned).
6.  **Cleanup:** Remove the test script.

## Release Summary Guidelines
*   When asked for a GitHub release summary from the previous git tag to the current one, only summarize the MCP server functionality. Chore and documentation updates should be excluded.

