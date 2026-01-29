# VM Template Management Specification

## Goal
Enable users to convert existing Virtual Machines (VMs) into templates. This is a one-way operation that is essential for creating "Golden Images" that can be cloned to provision new VMs rapidly.

## New Tools

### `template_vm`
- **Description:** Convert a VM into a template.
- **Arguments:**
  - `node` (string, required): The name of the node where the VM resides.
  - `vmid` (integer, required): The ID of the VM to convert.
- **Behavior:**
  - Calls the Proxmox API endpoint: `POST /api2/json/nodes/{node}/qemu/{vmid}/template`
  - Returns a standard success message or task UPID if async.

## Technical Details
- **Proxmox API:** `POST /nodes/{node}/qemu/{vmid}/template`
- **Permissions:** Requires `VM.Allocate` on the VM.
- **Client Implementation:** Add `template_vm` method to `ProxmoxClient` in `src/proxmox/vm.rs`.
- **MCP Handler:** Add case for `template_vm` in `src/mcp.rs`.
