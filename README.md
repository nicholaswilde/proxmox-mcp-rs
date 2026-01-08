# Proxmox MCP RS

> [!WARNING]
> This project is currently in active development (v0.x.x) and is **not production-ready**. Features may change, and breaking changes may occur without notice.

A Rust implementation of a Proxmox MCP (Model Context Protocol) server. This server connects to a Proxmox VE instance and exposes tools to manage nodes, VMs, and containers via the Model Context Protocol.

It is designed to be a faster, single-binary alternative to the Python-based [proxmox-mcp-plus](https://github.com/nicholaswilde/proxmox-mcp-plus).

## Features

- **Protocol:** JSON-RPC 2.0 over Stdio (MCP standard).
- **Authentication:** Proxmox User/Password (Ticket-based).
- **Tools:**
  - `list_nodes`: List all nodes in the cluster.
  - `list_vms`: List all VMs and LXC containers (uses `get_all_vms`).
  - `list_containers`: List all LXC containers.
  - `list_templates`: List container templates on a storage.
  - `start_vm` / `start_container`: Start a VM/Container.
  - `stop_vm` / `stop_container`: Stop (Power Off) a VM/Container.
  - `shutdown_vm` / `shutdown_container`: Gracefully shutdown a VM/Container.
  - `reset_vm` / `reset_container`: Reset (Stop and Start) a VM/Container.
  - `reboot_vm`: Reboot a VM/Container.
  - `create_vm` / `create_container`: Create a new VM or Container.
  - `delete_vm` / `delete_container`: Delete a VM or Container.
  - `update_container_resources`: Update LXC container resources (cores, memory, swap, disk).

## Build

To build the project, you need a Rust toolchain installed.

```bash
cargo build --release
```

The binary will be available at `target/release/proxmox-mcp-rs`.

## Usage

You can run the server directly from the command line, but it is intended to be used by an MCP client (like Claude Desktop, Cline, etc.).

### Command Line Arguments

```bash
./target/release/proxmox-mcp-rs --help
```

Arguments:
- `--config`, `-c`: Path to a configuration file (TOML, JSON, or YAML).
- `--host`: Proxmox Host (e.g., `192.168.1.10:8006`).
- `--user`: Proxmox User (e.g., `root@pam`).
- `--password`: Proxmox Password (optional if using token).
- `--token-name`: API Token Name (e.g., `mytoken`).
- `--token-value`: API Token Secret.
- `--no-verify-ssl`: Disable SSL verification (useful for self-signed certs).

### Configuration File

The server can load configuration from a file named `config.toml`, `config.yaml`, or `config.json` in the current directory, or via the `--config` flag. See `config.toml.example` for details.

### Environment Variables

You can also configure the server using environment variables:
- `PROXMOX_CONFIG`: Path to a configuration file.
- `PROXMOX_HOST`
- `PROXMOX_USER`
- `PROXMOX_PASSWORD`
- `PROXMOX_TOKEN_NAME`
- `PROXMOX_TOKEN_VALUE`
- `PROXMOX_NO_VERIFY_SSL` (set to `true` to disable verification)

### Configuration Example (Claude Desktop)

Add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "proxmox": {
      "command": "/path/to/proxmox-mcp-rs/target/release/proxmox-mcp-rs",
      "args": [
        "--host", "192.168.1.10:8006",
        "--user", "root@pam",
        "--password", "yourpassword",
        "--no-verify-ssl"
      ]
    }
  }
}
```

## License

MIT
