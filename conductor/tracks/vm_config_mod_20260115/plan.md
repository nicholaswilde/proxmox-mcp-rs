# Implementation Plan - Implement VM Configuration Modification Tools

## Phase 1: Resource Management (CPU/Memory)
- [x] Task: Create `update_vm_resources` tool structure and definition in `src/mcp.rs`.
    - Added `update_vm_resources` to `get_tool_definitions`.
    - Updated `call_tool` to handle `update_vm_resources`.
    - Updated `handle_update_resources` to support `sockets`.
- [~] Task: Implement `update_vm_config` function in `src/proxmox/vm.rs` to handle memory and core updates via Proxmox API.
    - [ ] Sub-task: Write unit tests with `wiremock` for successful and failed updates.
    - [ ] Sub-task: Implement the API call logic.
- [ ] Task: Integrate `update_vm_resources` tool with the backend logic.
- [ ] Task: Conductor - User Manual Verification 'Resource Management' (Protocol in workflow.md)

## Phase 2: Storage Management (Disks)
- [ ] Task: Create `add_vm_disk` and `remove_vm_disk` tool definitions in `src/mcp.rs`.
- [ ] Task: Implement `add_disk` and `remove_disk` functions in `src/proxmox/vm.rs`.
    - [ ] Sub-task: Write unit tests simulating disk addition and removal responses.
    - [ ] Sub-task: Implement the API logic, handling storage validation if possible.
- [ ] Task: Integrate storage tools with the MCP registry.
- [ ] Task: Conductor - User Manual Verification 'Storage Management' (Protocol in workflow.md)

## Phase 3: Network Management (Interfaces)
- [ ] Task: Create `add_vm_network` and `remove_vm_network` tool definitions in `src/mcp.rs`.
- [ ] Task: Implement `add_network` and `remove_network` functions in `src/proxmox/vm.rs`.
    - [ ] Sub-task: Write unit tests for network interface management.
    - [ ] Sub-task: Implement the API logic.
- [ ] Task: Integrate network tools with the MCP registry.
- [ ] Task: Conductor - User Manual Verification 'Network Management' (Protocol in workflow.md)
