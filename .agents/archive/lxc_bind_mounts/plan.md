# LXC Bind Mounts Plan

## Phase 1: Client Implementation
- [x] Add `add_lxc_bind_mount` to `src/proxmox/vm.rs`.
  - Logic: Validate paths, format string, call `update_config`.

## Phase 2: MCP Integration
- [x] Define tool in `src/mcp.rs`.
- [x] Implement handler.

## Phase 3: Verification
- [x] Verify compilation.
- [x] Test adding a bind mount to a mock container config.
