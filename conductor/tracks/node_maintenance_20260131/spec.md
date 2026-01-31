# Specification: Node Maintenance and HA Tools

## Goal
Add high-level node control tools for maintenance and power management.

## Features
1. **Node Power Control**: Reboot and shutdown Proxmox nodes.
2. **HA Maintenance**: Manage HA maintenance mode for nodes to ensure smooth resource migration.

## Tools to Add
- `reboot_node`: Reboot a Proxmox node.
- `shutdown_node`: Shutdown a Proxmox node.
- `set_ha_maintenance`: Set HA maintenance state for a node.
