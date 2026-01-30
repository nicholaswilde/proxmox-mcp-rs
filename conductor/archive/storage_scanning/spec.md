# Storage Scanning Specification

## Goal
Enable users to scan remote storage servers (NFS, CIFS, iSCSI, LVM) to discover available shares or targets before configuring them as Proxmox storage.

## New Tools

### `scan_storage_remote`
- **Description:** Scan a remote server for storage targets.
- **Arguments:**
  - `node` (string, required): Proxmox node to perform the scan from.
  - `type` (string, required): Storage type ("nfs", "cifs", "iscsi", "lvm", "zfs", "pbs").
  - `server` (string, required): Hostname or IP of the remote server.
  - `username` (string, optional): For CIFS/PBS.
  - `password` (string, optional): For CIFS/PBS.
- **Behavior:**
  - Calls: `GET /nodes/{node}/scan/{type}`
  - Returns: List of available shares/targets (structure depends on type).

## Technical Details
- **API Endpoint:** `/nodes/{node}/scan/{type}`
- **Parameters:**
  - `server`: always required.
  - `username`, `password`: type dependent.
- **Client Implementation:** Add `scan_storage` method to `ProxmoxClient` in `src/proxmox/storage.rs`.
- **MCP Handler:** Register `scan_storage_remote` tool.
