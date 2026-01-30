# LXC Bind Mounts Plan

## Phase 1: Client Implementation
- [ ] Add `add_lxc_bind_mount` to `src/proxmox/vm.rs`.
  - Logic: Validate paths, format string, call `update_config`.

## Phase 2: MCP Integration
- [ ] Define tool in `src/mcp.rs`.
- [ ] Implement handler.

## Phase 3: Verification
- [ ] Verify compilation.
- [ ] Test adding a bind mount to a mock container config.
