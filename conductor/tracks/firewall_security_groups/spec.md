# Firewall Security Groups Specification

## Goal
Manage cluster-wide Firewall Security Groups. Security groups allow defining sets of rules that can be applied to multiple VMs or containers.

## New Tools

### `list_security_groups`
- **Description:** List all firewall security groups.

### `create_security_group`
- **Description:** Create a new security group.
- **Arguments:**
  - `name` (string, required): Group name.
  - `comment` (string, optional): Description.

### `manage_security_group_rules`
- **Description:** Add or remove rules within a security group.
- **Arguments:**
  - `name` (string, required): Group name.
  - `action` (string, required): "add" or "delete".
  - `rule` (object, required): Rule parameters.

## Technical Details
- **API Endpoint:** `/cluster/firewall/groups`
- **Client:** Update `src/proxmox/firewall.rs`.
