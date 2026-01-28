# Implementation Plan: MCP Tool Integration Testing

This plan implements high-level integration tests that verify the `McpServer` and `ProxmoxClient` logic together against a mocked API.

## Phase 1: Infrastructure & Shared Helpers cf0c1a8378824aa72ace2cf46b969a84942397f3
To avoid duplication, we will move existing WireMock helpers to a shared test module and set up the `McpServer` test harness.

- [~] Task: Create Shared Test Module
    - [ ] Create `tests/common/mod.rs`.
    - [ ] Move helpers (`mock_auth_success`, `mock_node_list_success`, etc.) and constants from `tests/wiremock_proxmox.rs` to `tests/common/mod.rs`.
    - [ ] Update `tests/wiremock_proxmox.rs` to use the shared module.
- [x] Task: Implement MCP Test Factory
    - [ ] Create `tests/mcp_integration.rs`.
    - [ ] Implement a helper to create an `McpServer` instance pointing to a `MockServer`.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Infrastructure & Shared Helpers' (Protocol in workflow.md)

## Phase 2: Information Retrieval Tests fa5b55ea8fde8f78a93ea71c793fefea086808aa
Verify that read-only tools correctly parse API responses and return valid MCP text content.

- [x] Task: Test List Nodes and VMs
- [~] Task: Test Cluster Status
    - [ ] Write test for `get_cluster_status` tool.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Information Retrieval Tests' (Protocol in workflow.md)

## Phase 3: Lifecycle & Configuration Tests 91371117e70f926fa88dee3e9463597a4440190a
Verify that tools requiring task polling (`wait_for_task`) and parameter mapping work correctly.

- [x] Task: Test VM Lifecycle (Start/Stop)
    - [ ] Write test for `start_vm`.
    - [ ] Mock the sequence: POST start -> UPID -> GET task status (stopped).
    - [ ] Verify `McpServer` waits for task completion before responding.
- [~] Task: Test Hardware Configuration
    - [ ] Write test for `add_disk`.
    - [ ] Verify parameters are correctly mapped to the Proxmox API request.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Lifecycle & Configuration Tests' (Protocol in workflow.md)

## Phase 4: Error Mapping & Robustness
Ensure that Proxmox API errors are translated into correct MCP JSON-RPC error codes.

- [~] Task: Test Error Mapping
    - [ ] Write test for authentication failure (401 -> -32001).
    - [ ] Write test for resource missing (404 -> -32004).
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Error Mapping & Robustness' (Protocol in workflow.md)
