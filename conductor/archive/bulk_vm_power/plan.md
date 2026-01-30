# Bulk VM Power Management Plan

## Phase 1: Client Implementation
- [x] Add `bulk_vm_action` to `ProxmoxClient` in `src/proxmox/vm.rs`.
  - Input: `Vec<i64>` of VMIDs.
  - Logic: Use `futures::stream` or `join_all` to execute `vm_action` for each ID concurrently.
  - Return: A report of successes and failures (e.g., `map<vmid, result>`).

## Phase 2: MCP Integration
- [x] Define `bulk_vm_action` tool in `src/mcp.rs`.
- [x] Implement handler.
  - Parse `vmids` array.
  - Call client method.
  - Format response to show which started and which failed.

## Phase 3: Verification
- [x] Verify compilation.
- [x] (Manual) Test stopping 2 test VMs simultaneously.
