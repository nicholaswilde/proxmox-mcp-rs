# Implementation Plan: Expand WireMock Integration Tests

This plan expands the integration testing suite to cover all core MCP tools and Proxmox client methods using `wiremock`.

## Phase 1: Infrastructure & Common Helpers 4088af0e7a32503561031e180d8171dfcbdb3afc
Prepare the testing infrastructure to support a wide variety of mocked endpoints with minimal boilerplate.

- [x] Task: Create Shared Mock Helpers
    - [ ] Implement `mock_auth_success` helper.
    - [ ] Implement `mock_node_list_success` helper.
    - [ ] Create a standard `ProxmoxResponse` wrapper utility for generating JSON payloads.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Infrastructure & Common Helpers' (Protocol in workflow.md)

## Phase 2: Core Tool Integration Tests
Implement tests for the most frequently used cluster and compute management tools.

- [x] Task: Cluster and Node Management Tests
    - [ ] Write failing test for `list_nodes` and `get_cluster_status`.
    - [ ] Implement mocks and verify success.
- [x] Task: VM and Container Lifecycle Tests
    - [ ] Write failing tests for `start_vm`, `stop_vm`, and `shutdown_vm`.
    - [ ] Implement mocks for VM status and task UPID returns.
    - [ ] Verify `wait_for_task` logic works against mocked status polling.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Core Tool Integration Tests' (Protocol in workflow.md)

## Phase 3: Resource & Configuration Tests
Expand coverage to storage, networking, and guest configurations.

- [ ] Task: Storage and ISO Management Tests
    - [ ] Write failing tests for `list_storage` and `list_isos`.
    - [ ] Implement mocks for storage content listing.
- [ ] Task: Networking and Firewall Tests
    - [ ] Write failing tests for `list_networks` and `list_firewall_rules`.
    - [ ] Implement mocks for node network interfaces.
- [ ] Task: Snapshot and Backup Tests
    - [ ] Write failing tests for `list_snapshots` and `list_backups`.
    - [ ] Implement mocks for snapshot trees and backup volumes.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Resource & Configuration Tests' (Protocol in workflow.md)

## Phase 4: Error Handling & Robustness
Ensure the MCP server handles Proxmox API failures gracefully and returns appropriate MCP errors.

- [ ] Task: Authentication and Authorization Error Tests
    - [ ] Write failing tests for 401 Unauthorized and 403 Forbidden responses.
    - [ ] Verify MCP error codes match specification (e.g., -32001).
- [ ] Task: Resource Missing and API Timeout Tests
    - [ ] Write failing tests for 404 Not Found (invalid VMID).
    - [ ] Write failing tests for connection timeouts/500 errors.
    - [ ] Verify that the server returns descriptive error messages to the LLM.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Error Handling & Robustness' (Protocol in workflow.md)
