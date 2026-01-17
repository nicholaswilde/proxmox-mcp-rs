# Implementation Plan: Cluster Management Tools

## Phase 1: Core Implementation
- [x] Task: Update `src/proxmox/cluster.rs` (or create new `cluster_ops.rs` if `cluster.rs` is too big) to implement:
    - `create_cluster(clustername)`
    - `get_join_info()`
    - `join_cluster(hostname, password, fingerprint)`
- [x] Task: Add new tools to `src/mcp.rs`:
    - `create_cluster`
    - `get_cluster_join_info`
    - `join_cluster`

## Phase 2: Testing & Verification
- [x] Task: Add unit tests in `src/tests.rs` mocking the cluster endpoints.
    - Mock POST `/cluster/config`
    - Mock GET `/cluster/config/join`
    - Mock POST `/cluster/config/join`
- [x] Task: Manual verification (requires caution as this modifies cluster state).