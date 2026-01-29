# Firewall Alias Management Specification

## Goal
Enable users to manage Proxmox Firewall Aliases (Cluster and Node level). Aliases allow defining friendly names for IP addresses or networks (e.g., "web_servers" = "10.0.0.5,10.0.0.6"), making firewall rules more readable and easier to maintain.

## New Tools

### `list_firewall_aliases`
- **Description:** List all firewall aliases.
- **Arguments:**
  - `level` (string, required): "cluster" or "node".
  - `node` (string, optional): Node name (required if level is "node").

### `create_firewall_alias`
- **Description:** Create a new firewall alias.
- **Arguments:**
  - `level` (string, required): "cluster" or "node".
  - `node` (string, optional): Node name (required if level is "node").
  - `name` (string, required): Alias name (e.g., "web_servers").
  - `cidr` (string, required): Network/IP (e.g., "192.168.1.0/24").
  - `comment` (string, optional): Description.

### `update_firewall_alias`
- **Description:** Update an existing firewall alias.
- **Arguments:**
  - `level` (string, required): "cluster" or "node".
  - `node` (string, optional): Node name.
  - `name` (string, required): Alias name to update.
  - `cidr` (string, required): New Network/IP.
  - `comment` (string, optional): New Description.

### `delete_firewall_alias`
- **Description:** Delete a firewall alias.
- **Arguments:**
  - `level` (string, required): "cluster" or "node".
  - `node` (string, optional): Node name.
  - `name` (string, required): Alias name to delete.

## Technical Details
- **API Endpoints:**
  - Cluster: `/cluster/firewall/aliases`
  - Node: `/nodes/{node}/firewall/aliases`
- **Client Implementation:** Add methods to `ProxmoxClient` (likely in a new `src/proxmox/firewall.rs` or existing `system.rs`).
- **MCP Handler:** Register new tools and handlers in `src/mcp.rs`.
