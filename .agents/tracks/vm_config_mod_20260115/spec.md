# Specification: Implement VM Configuration Modification Tools

## 1. Overview
This track focuses on expanding the capability of the Proxmox MCP server to modify Virtual Machine (VM) and LXC Container configurations. The goal is to allow users to change resource limits (CPU, Memory), add/remove storage devices, and manage network interfaces via new MCP tools.

## 2. User Stories
- As a DevOps engineer, I want to increase the memory or CPU cores of a VM so that I can scale resources dynamically.
- As a system administrator, I want to add a new disk to a VM to expand its storage capacity.
- As a network engineer, I want to add or remove network interfaces from a container to change its network connectivity.

## 3. Functional Requirements
### 3.1 VM/LXC Resource Management
- **Tool:** `update_vm_resources`
    - **Parameters:** `vmid`, `memory` (optional), `cores` (optional), `sockets` (optional).
    - **Behavior:** Updates the VM's hardware configuration.
- **Tool:** `update_container_resources` (Enhance existing or verify)
    - **Parameters:** `vmid`, `memory` (optional), `swap` (optional), `cores` (optional).

### 3.2 Storage Management
- **Tool:** `add_vm_disk`
    - **Parameters:** `vmid`, `storage`, `size`, `type` (scsi, sata, virtio).
    - **Behavior:** Creates a new volume on the specified storage and attaches it to the VM.
- **Tool:** `remove_vm_disk`
    - **Parameters:** `vmid`, `disk_id` (e.g., scsi1).
    - **Behavior:** Detaches and optionally deletes the disk volume.

### 3.3 Network Management
- **Tool:** `add_vm_network`
    - **Parameters:** `vmid`, `bridge` (default: vmbr0), `model` (default: virtio).
    - **Behavior:** Adds a new network interface to the VM.
- **Tool:** `remove_vm_network`
    - **Parameters:** `vmid`, `interface_id` (e.g., net1).
    - **Behavior:** Removes the specified network interface.

## 4. Technical Considerations
- **Proxmox API:** Use the `/nodes/{node}/qemu/{vmid}/config` and `/nodes/{node}/lxc/{vmid}/config` endpoints.
- **Asynchronous Operations:** Some configuration changes (like disk creation) might be locked or take time; ensure proper error handling and async waiting if necessary.
- **Safety:** Validate inputs (e.g., ensure memory is a positive integer, storage exists) to prevent misconfiguration.

## 5. Testing Strategy
- **Unit Tests:** Use `wiremock` to simulate Proxmox API responses for config updates.
- **Integration Tests:** Verify that the MCP tools correctly parse arguments and form the correct HTTP requests.
