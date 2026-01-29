# Storage Scanning Plan

## Phase 1: Client Implementation
- [ ] Add `scan_storage` method to `ProxmoxClient` in `src/proxmox/storage.rs`.
  - Signature: `async fn scan_storage(&self, node: &str, storage_type: &str, server: &str, user: Option<&str>, password: Option<&str>) -> Result<Value>`
  - Path: `nodes/{node}/scan/{type}`

## Phase 2: MCP Integration
- [ ] Define `scan_storage_remote` tool in `src/mcp.rs` (`tool_defs_storage`).
- [ ] Implement handler `handle_scan_storage_remote`.

## Phase 3: Verification
- [ ] Verify compilation.
- [ ] (Manual) Test scanning a public NFS or iSCSI target (if available) or mock response.
