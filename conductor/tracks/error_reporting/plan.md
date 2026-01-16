# Implementation Plan: Enhanced Error Reporting

## Phase 1: Analysis & Design
- [x] Task: Analyze current error propagation in `src/proxmox/client.rs` and `src/mcp.rs`.
- [x] Task: Design a `ProxmoxError` enum in `src/proxmox/mod.rs` or `client.rs` to capture:
    - ApiError(StatusCode, String)
    - AuthError
    - TimeoutError
    - TaskError(String) // UPID
    - ParseError
    - NotFound(String) // Resource

## Phase 2: Implementation
- [x] Task: Implement `ProxmoxError` and `Result` type alias.
    - Added `src/proxmox/error.rs` using `thiserror`.
- [x] Task: Refactor `ProxmoxClient::request` to return `ProxmoxError`.
    - Updated `src/proxmox/client.rs` and fixed all call sites in `src/proxmox/*.rs`.
- [x] Task: Update `mcp.rs` to handle `ProxmoxError` and map it to `JsonRpcError` with appropriate codes:
    - -32603 (Internal) for unexpected
    - -32001 for Auth (401/403)
    - -32004 for NotFound (404)
    - -32000 for other API errors (mapped to -32603 for now with explicit message)
- [x] Task: Add context to `anyhow` usage in `src/mcp.rs` tool handlers (e.g., "Failed to start VM {vmid} on node {node}").
    - *Note: Most context is already provided by the specific `ProxmoxError` variants or existing anyhow context.*

## Phase 3: Verification
- [x] Task: Write unit tests in `src/tests.rs` simulating API errors (401, 404, 500) and verifying the error message format.
- [x] Task: Manual verification with a client (if possible) or `cargo run`.