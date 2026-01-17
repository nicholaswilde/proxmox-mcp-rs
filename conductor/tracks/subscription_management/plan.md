# Implementation Plan: Subscription Management Tools

## Phase 1: Core Implementation
- [x] Task: Create `src/proxmox/subscription.rs`.
- [x] Task: Implement `get_subscription`, `set_subscription`, and `update_subscription` in `src/proxmox/subscription.rs` (and extend `ProxmoxClient`).
- [x] Task: Add new tools to `src/mcp.rs`:
    - `get_subscription_info`
    - `set_subscription_key`
    - `check_subscription`
- [x] Task: Update `src/proxmox/mod.rs` to include the new module.

## Phase 2: Testing & Verification
- [x] Task: Add unit tests in `src/tests.rs` mocking the subscription endpoints.
- [x] Task: Verify manually (if possible) or ensure mocks cover error cases (invalid key, etc.).