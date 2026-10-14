# Bulk VM Power Management Specification

## Goal
Allow users to perform power operations (Start, Stop, Shutdown, Suspend) on multiple VMs simultaneously. This improves efficiency for managing groups of resources (e.g., shutting down a lab environment).

## New Tools

### `bulk_vm_action`
- **Description:** Perform a power action on a list of VMs.
- **Arguments:**
  - `node` (string, required): The node name.
  - `vmids` (array of integers, required): List of VM IDs.
  - `action` (string, required): One of "start", "stop", "shutdown", "suspend", "resume", "reboot".
- **Behavior:**
  - Can use the Proxmox Bulk API endpoints if available (e.g., `/nodes/{node}/startall` generally takes a `vms` param but is for *all* or filtered).
  - **Strategy:** To be safe and explicit, the client will iterate through the provided `vmids` and issue individual API calls asynchronously (or efficiently in parallel). *Wait, Proxmox has a `/nodes/{node}/startall` but it's often "start all on boot".*
  - **Refined Strategy:** Proxmox API typically handles bulk actions via specific endpoints like `/nodes/{node}/stopall` etc., but they often apply to *all* VMs.
  - **Safe Implementation:** We will implement this as a "Client-side Bulk" operation where `proxmox-mcp-rs` iterates the list and calls the single-VM endpoints. This ensures precise control over exactly which VMs are affected.

## Technical Details
- **Implementation:**
  - `ProxmoxClient` won't necessarily need a new "bulk" method if the logic is in the MCP handler layer, BUT adding a helper `bulk_vm_action` in the client that uses `futures::future::join_all` is cleaner.
- **Concurrency:** Execute requests in parallel (up to a reasonable limit) to speed up the operation.

