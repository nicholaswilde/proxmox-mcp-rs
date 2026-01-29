# VM Template Management Plan

## Phase 1: Client Implementation
- [x] Add `template_vm` method to `ProxmoxClient` in `src/proxmox/vm.rs`.
  - Signature: `async fn template_vm(&self, node: &str, vmid: i64) -> Result<String>`
  - API Path: `nodes/{node}/qemu/{vmid}/template`

## Phase 2: MCP Integration
- [x] Add `template_vm` to the tool definitions in `src/mcp.rs` (`tool_defs_vm_lifecycle`).
- [x] Add handler case for `template_vm` in `handle_vm_action` or a new handler method in `src/mcp.rs`.

## Phase 3: Verification
- [x] Verify compilation.
- [x] (Manual) Test converting a dummy VM to a template.