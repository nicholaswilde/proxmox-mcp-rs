# APT Repository Management Plan

## Phase 1: Client Implementation
- [ ] Add APT repository methods to `ProxmoxClient`.
  - `get_repositories(node)`
  - `add_repository(node, handle)`
  - `change_repository_state(node, index, enabled)`

## Phase 2: MCP Integration
- [ ] Define tools in `src/mcp.rs`.
- [ ] Implement handlers.

## Phase 3: Verification
- [ ] Verify compilation.
- [ ] Test listing and state changing via mock server.
