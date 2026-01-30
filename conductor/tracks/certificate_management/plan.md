# Certificate Management Plan

## Phase 1: Client Implementation
- [ ] Add certificate methods to `ProxmoxClient`.
  - `get_certificates(node)`
  - `upload_certificate(node, certs, key)`
  - `renew_acme_certificate(node)`

## Phase 2: MCP Integration
- [ ] Define tools in `src/mcp.rs`.
- [ ] Implement handlers.

## Phase 3: Verification
- [ ] Verify compilation.
- [ ] Mock certificate listing and upload responses.
