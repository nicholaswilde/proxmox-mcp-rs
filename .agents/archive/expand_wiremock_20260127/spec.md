# Track Specification: Expand WireMock Integration Tests

## Overview
This track aims to expand the existing `tests/wiremock_proxmox.rs` into a comprehensive integration test suite. It will mock the full range of Proxmox VE API endpoints used by the MCP tools and internal client methods, ensuring reliable, offline verification of the server's logic and error handling.

## Functional Requirements
- **Comprehensive API Coverage:** Implement mocks for:
    - **Access:** Login (Ticket/CSRF) and User management.
    - **Nodes:** Listing, status, and stats.
    - **VM/LXC:** Lifecycle (start/stop/shutdown/reboot), configuration retrieval, resource updates, and console access.
    - **Storage & ISOs:** Listing storage, storage content (ISOs/Templates), and volume details.
    - **Networking:** Interface listing and configuration.
    - **Snapshots & Backups:** Listing, creating, rolling back, and deleting.
    - **Task Tracking:** Listing tasks and polling task status (UPID).
- **Error Scenario Testing:** Mock common API failure modes:
    - 401 Unauthorized / 403 Forbidden.
    - 404 Not Found (e.g., missing VMID).
    - 500 Internal Server Error.
    - Request timeouts and connection resets.
- **MCP Tool Verification:** Verify that `McpServer::call_tool` correctly interacts with the mocked client and formats output/errors as valid MCP JSON-RPC responses.

## Non-Functional Requirements
- **Performance:** Tests should run quickly (< 5 seconds total).
- **Maintainability:** Use a hybrid mock data strategy (shared helpers for common boilerplate, local overrides for specific test cases).
- **Stability:** Tests must be deterministic and run without a network connection.

## Acceptance Criteria
- [ ] `tests/wiremock_proxmox.rs` contains test cases covering at least one tool from every major category (Cluster, VM, Storage, Network, Access).
- [ ] Tests verify both successful tool execution and graceful error handling for failed API calls.
- [ ] The suite passes consistently with `cargo test --test wiremock_proxmox`.
- [ ] Mocked JSON responses strictly follow the Proxmox `{"data": ...}` wrapper convention.

## Out of Scope
- Mocking the SSE (Server-Sent Events) stream for live task updates (to be handled in a separate track if needed).
- Performance benchmarking of the MCP server.
