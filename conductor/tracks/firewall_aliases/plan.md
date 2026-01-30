# Firewall Alias Management Plan

## Phase 1: Client Implementation
- [x] Create/Update `src/proxmox/firewall.rs` (or `system.rs`) to include Alias methods.
  - `get_aliases(level, node)`
  - `create_alias(level, node, name, cidr, comment)`
  - `update_alias(level, node, name, cidr, comment)`
  - `delete_alias(level, node, name)`

## Phase 2: MCP Integration
- [x] Define tools in `src/mcp.rs`:
  - `list_firewall_aliases`
  - `create_firewall_alias`
  - `update_firewall_alias`
  - `delete_firewall_alias`
- [x] Implement handlers for these tools.

## Phase 3: Verification
- [x] Verify compilation.
- [x] (Manual) Test creating, listing, updating, and deleting aliases at both cluster and node levels.
