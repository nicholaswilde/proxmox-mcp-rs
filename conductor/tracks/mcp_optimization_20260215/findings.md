# Optimization Findings - MCP Token Optimization

## Tool Consolidation Opportunities
- **VM Power**: Merge `start_vm`, `stop_vm`, `shutdown_vm`, `reset_vm`, `reboot_vm`, `start_container`, `stop_container`, `shutdown_container`, `reset_container` -> `vm_power_action(node, vmid, type, action)`.
- **Resource Lifecycle**: Merge `create_vm`/`create_container` and `delete_vm`/`delete_container`.
- **Firewall**: Merge Alias and Security Group CRUD operations.
- **SDN**: Merge Zone and Vnet CRUD operations.
- **Storage**: Merge CRUD operations for storage definitions.

## Payload Reduction
- **Pretty-printing**: Remove all `to_string_pretty` calls in favor of compact JSON.
- **Field Filtering**:
    - `list_nodes`: Keep only `node`, `status`, `cpu`, `mem`, `maxcpu`, `maxmem`, `uptime`.
    - `list_vms`: Keep existing fields (already thin).
    - `get_vm_config`: Possibly filter out extremely long fields or internal hardware IDs unless requested.
    - `list_tasks`: Keep only `upid`, `node`, `user`, `starttime`, `status`, `type`.

## Description Refinement
- Many descriptions use flowery language ("Retrieves a comprehensive list..."). These can be shortened to direct imperatives ("List...").
- Argument descriptions can be shortened (e.g., "Unique Process ID" -> "UPID").
