# Specification: Advanced Storage Operations

## Goal
Add advanced storage management tools to the MCP server, specifically for disk resizing, moving, and ZFS health monitoring.

## Features
1. **Resize VM Disk**: Increase the size of an existing virtual disk.
2. **Move VM Disk**: Move a disk from one storage to another (online/offline).
3. **ZFS Status**: List ZFS pools and get detailed status for a specific pool.

## API Endpoints
- PUT `/nodes/{node}/qemu/{vmid}/resize`: Resize VM disk.
- POST `/nodes/{node}/qemu/{vmid}/move_disk`: Move VM disk.
- GET `/nodes/{node}/storage/{storage}/zfs`: (Hypothetical or CLI-based via agent if API is limited). Note: PVE has `/nodes/{node}/disks/zfs`.

## Tools to Add
- `resize_vm_disk`: Increase disk size.
- `move_vm_disk`: Move disk between storages.
- `list_zfs_pools`: List ZFS pools on a node.
- `get_zfs_status`: Get status of a specific ZFS pool.
