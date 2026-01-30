# Certificate Management Plan

## Phase 1: Client Implementation
- [x] Add certificate methods to `ProxmoxClient`.
  - `get_certificates(node)`
  - `upload_certificate(node, certs, key)`
  - `renew_acme_certificate(node)`

## Phase 2: MCP Integration
- [x] Define tools in `src/mcp.rs`.
- [x] Implement handlers.

## Phase 3: Verification
- [x] Verify compilation.
- [x] Mock certificate listing and upload responses.
