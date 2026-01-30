# LXC Bind Mounts Specification

## Goal
Simplify the addition of bind mounts to LXC containers. Bind mounts allow exposing directories from the host to the container.

## New Tools

### `add_lxc_bind_mount`
- **Description:** Add a bind mount to an LXC container.
- **Arguments:**
  - `node` (string, required): Node name.
  - `vmid` (integer, required): Container ID.
  - `mp_id` (string, required): Mount point ID (e.g. mp0, mp1).
  - `source` (string, required): Host directory path.
  - `target` (string, required): Container directory path.
  - `read_only` (boolean, optional): Default false.

## Technical Details
- **Behavior:** This tool is a wrapper around the `config` endpoint that formats the string correctly (e.g. `/host/path,mp=/container/path`).
- **Client:** Update `src/proxmox/vm.rs`.
