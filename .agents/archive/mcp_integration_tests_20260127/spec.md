# Track Specification: MCP Tool Integration Testing

## Overview
This track implements end-to-end integration tests for the MCP tools. Unlike previous tests that verified raw API responses, these tests will instantiate the real `McpServer` and `ProxmoxClient`, directing them to a WireMock server. This ensures that the server logic (JSON-RPC handling, parameter mapping, task polling, and error transformation) works correctly with the Proxmox API.

## Functional Requirements
- **Integration Test Suite:** Create `tests/mcp_integration.rs`.
- **Server Orchestration:**
    - Initialize `ProxmoxClient` with the WireMock URI.
    - Instantiate `McpServer` with the client.
- **Tool Coverage:** Implement tests that invoke `McpServer::call_tool` for:
    - **Compute:** `list_nodes`, `list_vms`.
    - **Lifecycle:** `start_vm`, `stop_vm` (verifying `wait_for_task` logic).
    - **Configuration:** `add_disk`, `add_network`.
- **Protocol Verification:** Assert that the output of `call_tool` is a valid MCP `Value` containing the expected text or JSON structure.
- **Error Mapping:** Verify that API failures (e.g., 401 Unauthorized) are correctly mapped to MCP-standard JSON-RPC error codes by the server.

## Non-Functional Requirements
- **Maintainability:** Reuse the helper functions (like `mock_auth_success`) from the existing codebase where possible (may require moving them to a shared module or copying).
- **Hermeticity:** Tests must be fully self-contained and run against the local WireMock instance.

## Acceptance Criteria
- [ ] `tests/mcp_integration.rs` successfully compiles and runs.
- [ ] Tests verify that `McpServer` can successfully authenticate and execute at least one tool from each priority category.
- [ ] Tests verify that the async task polling logic in `McpServer` correctly handles mocked UPIDs and status updates.
- [ ] `task test:ci` passes with the new integration tests.

## Out of Scope
- Stdio transport testing (handling raw stdin/stdout streams).
- Real Proxmox VE connection (handled by live tests).
