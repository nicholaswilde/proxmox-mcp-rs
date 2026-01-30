# Firewall Security Groups Plan

## Phase 1: Client Implementation
- [ ] Add Security Group methods to `src/proxmox/firewall.rs`.
  - `get_security_groups()`
  - `create_security_group(name, comment)`
  - `delete_security_group(name)`
  - `get_security_group_rules(name)`
  - `add_security_group_rule(name, rule)`

## Phase 2: MCP Integration
- [ ] Define tools in `src/mcp.rs`.
- [ ] Implement handlers.

## Phase 3: Verification
- [ ] Verify compilation.
- [ ] Test creating a group and adding a rule.
