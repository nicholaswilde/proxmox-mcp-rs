# Verification Log

This file tracks the tools and features that have been manually verified against a live Proxmox environment using `config.toml`.

## 2026-01-17 - v0.3.14 to v0.3.17 Features

**Environment:**
- Proxmox VE 8.x (assumed based on API)
- Auth: API Token

**Verified Tools:**

### Cluster Management
- [x] `get_cluster_join_info`: Successfully retrieved join information (IP, fingerprint, totem config).
- [ ] `create_cluster`: Skipped (Destructive).
- [ ] `join_cluster`: Skipped (Destructive).

### Subscription Management
- [x] `get_subscription_info`: Successfully retrieved subscription status ("notfound").
- [x] `check_subscription`: Successfully initiated subscription check.
- [x] `set_subscription_key`: Verified input validation (400 Bad Request with "value does not match regex" for invalid key).

### VM Configuration
- [x] `update_vm_resources`: Successfully updated VM memory (8192 -> 8320 -> 8192). Verified 500 Internal Error handling for non-existent VM.

### Error Reporting
- [x] Verified 401 Unauthorized (Invalid Token).
- [x] Verified 400 Bad Request (Validation Error).
- [x] Verified 500 Internal Server Error (Target not found).
