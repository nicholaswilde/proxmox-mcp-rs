# APT Repository Management Plan

## Phase 1: Client Implementation
- [x] Add APT repository methods to `ProxmoxClient`.
  - `get_repositories(node)`
  - `add_repository(node, handle)`
  - `change_repository_state(node, index, enabled)`

## Phase 2: MCP Integration
- [x] Define tools in `src/mcp.rs`.
- [x] Implement handlers.

## Phase 3: Verification
- [x] Verify compilation.
- [x] Test listing and state changing via mock server.
