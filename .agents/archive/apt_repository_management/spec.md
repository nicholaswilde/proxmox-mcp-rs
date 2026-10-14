# APT Repository Management Specification

## Goal
Allow management of APT repositories on Proxmox nodes. This is primarily used to configure update channels (e.g., adding `no-subscription` repositories).

## New Tools

### `list_repositories`
- **Description:** List configured APT repositories on a node.
- **Arguments:**
  - `node` (string, required): Node name.

### `add_repository`
- **Description:** Add a standard Proxmox repository.
- **Arguments:**
  - `node` (string, required): Node name.
  - `handle` (string, required): Repository handle (e.g., "pve-no-subscription", "pve-enterprise").

### `update_repository_state`
- **Description:** Enable or disable a repository.
- **Arguments:**
  - `node` (string, required): Node name.
  - `index` (integer, required): Repository index.
  - `enabled` (boolean, required): Whether the repo should be enabled.

## Technical Details
- **API Endpoints:**
  - GET `/nodes/{node}/apt/repositories`
  - POST `/nodes/{node}/apt/repositories`
- **Client:** Update `src/proxmox/system.rs` (or create `apt.rs`).
