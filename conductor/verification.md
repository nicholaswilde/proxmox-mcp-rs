# Verification Log

This file tracks the tools and features that have been manually verified against a live Proxmox environment using `config.toml`.

## 2026-01-17 - Comprehensive Tool Verification (v0.3.14+)

**Environment:**
- Proxmox VE 8.x
- Cluster: pxe01 (2 nodes: pve01, pve03)
- Auth: API Token

**Verified Tools:**

### Cluster Management
- [x] `get_cluster_join_info`: Successfully retrieved join information (IP, fingerprint, totem config).
- [ ] `create_cluster`: Skipped (Destructive).
- [ ] `join_cluster`: Skipped (Destructive).

### Cluster & Node
- [x] `list_nodes`: Successfully listed nodes (`pve01`, `pve03`) with stats.
- [x] `get_cluster_status`: Successfully retrieved cluster status.
- [x] `get_cluster_log`: Successfully retrieved recent cluster logs.
- [x] `get_node_stats`: Successfully retrieved RRD stats for `pve01`.

### VM & Container Lifecycle
- [x] `list_vms`: Successfully listed all VMs and containers.
- [x] `list_containers`: Successfully listed only containers.
- [x] `get_vm_config`: Successfully retrieved config for VM 100 (`omv`).
- [x] `get_console_url`: Successfully generated a NoVNC URL.
- [x] `get_vm_stats`: Successfully retrieved RRD stats for VM 100.

### VM & Container Configuration
- [x] `update_vm_resources`: Successfully updated VM memory (8192 -> 8320 -> 8192).
- [x] `add_tag`: Successfully added "testtag" to VM 100.
- [x] `remove_tag`: Successfully removed "testtag" from VM 100.
- [ ] `set_tags`: Skipped (covered by add/remove).
- [ ] `update_container_resources`: Skipped (similar logic to VM resources).

### Subscription Management
- [x] `get_subscription_info`: Successfully retrieved subscription status ("notfound").
- [x] `check_subscription`: Successfully initiated subscription check.
- [x] `set_subscription_key`: Verified input validation (400 Bad Request with "value does not match regex" for invalid key).

### Users & Access Control
- [x] `list_users`: Successfully listed users.
- [x] `list_roles`: Successfully listed roles.
- [x] `list_acls`: Successfully listed ACLs.

### System & Services
- [x] `list_services`: Successfully listed system services on `pve01`.
- [x] `list_apt_updates`: Successfully listed available updates.
- [x] `get_apt_versions`: Successfully listed installed package versions.

### QEMU Guest Agent
- [x] `vm_agent_ping`: Successfully pinged agent on VM 100.

### Resource Pools
- [x] `create_pool`: Successfully created "mcp-test-pool".
- [x] `list_pools`: Successfully listed the new pool.
- [x] `get_pool_details`: Successfully retrieved pool details.
- [x] `update_pool`: Successfully updated the pool comment.
- [x] `delete_pool`: Successfully deleted the pool.

### High Availability (HA) & Replication
- [x] `list_ha_resources`: Successfully returned empty list (no HA resources configured).
- [x] `list_replication_jobs`: Successfully returned empty list (no replication jobs configured).
- [x] `list_ha_groups`: Tool executed but API returned 500 "ha groups have been migrated to rules" (Proxmox version specific behavior).

### Error Reporting
- [x] Verified 401 Unauthorized (Invalid Token).
- [x] Verified 400 Bad Request (Validation Error).
- [x] Verified 500 Internal Server Error (Target not found).