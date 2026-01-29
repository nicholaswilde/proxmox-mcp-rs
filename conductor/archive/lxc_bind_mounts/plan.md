# Implementation Plan: LXC Bind Mount Tools

## Phase 1: Implementation
- [x] Task: Update `src/proxmox/vm.rs` to implement `add_lxc_mountpoint` and `remove_lxc_mountpoint`.
    - `add_lxc_mountpoint` accepts `mp_id` (mp0-mp9), `volume` (host path or storage volume), `path` (container path), and optional flags (ro, backup, size, etc.).
- [x] Task: Add `add_lxc_mountpoint` and `remove_lxc_mountpoint` tools to `src/mcp.rs`.

## Phase 2: Testing & Verification
- [x] Task: Add unit tests in `src/tests.rs` mocking the LXC config update endpoints.
- [x] Task: Run `task test:ci` to verify all tests, formatting, and linting pass.
- [x] Task: Manual verification using `config.toml` (create a test CT, add mount, remove mount).
