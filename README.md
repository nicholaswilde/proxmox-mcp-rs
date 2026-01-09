# :crab: Proxmox MCP RS :robot:

[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)

> [!WARNING]
> This project is currently in active development (v0.x.x) and is **not production-ready**. Features may change, and breaking changes may occur without notice.

A Rust implementation of a Proxmox MCP (Model Context Protocol) server. This server connects to a Proxmox VE instance and exposes tools to manage nodes, VMs, and containers via the Model Context Protocol.

It is designed to be a faster, single-binary alternative to the Python-based [proxmox-mcp-plus](https://github.com/nicholaswilde/proxmox-mcp-plus).

## :sparkles: Features

- **Protocol:** JSON-RPC 2.0 over Stdio (MCP standard).
- **Authentication:** Proxmox User/Password (Ticket-based) or API Token.
- **Logging:** Configurable log levels, console output (stderr), and optional file logging with rotation (daily, hourly).
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
  - `list_snapshots`: List snapshots for a VM or Container.
  - `snapshot_vm`: Create a snapshot of a VM or Container.
  - `rollback_vm`: Rollback a VM or Container to a snapshot.
  - `delete_snapshot`: Delete a snapshot of a VM or Container.
  - `clone_vm`: Clone a VM or Container.
  - `migrate_vm`: Migrate a VM or Container to another node.

## :hammer_and_wrench: Build

To build the project, you need a Rust toolchain installed.

```bash
cargo build --release
```

The binary will be available at `target/release/proxmox-mcp-rs`.

## :rocket: Usage

You can run the server directly from the command line, or using Docker.

### :whale: Docker

#### Build

```bash
docker build -t proxmox-mcp-rs .
```

#### Run

```bash
docker run --rm -it \
  -e PROXMOX_HOST="192.168.1.10" \
  -e PROXMOX_USER="root@pam" \
  -e PROXMOX_PASSWORD="yourpassword" \
  -e PROXMOX_NO_VERIFY_SSL="true" \
  proxmox-mcp-rs
```

### :vhs: Docker Compose

Copy `config.toml.example` to `config.toml` and update it with your credentials, then run:

```bash
docker compose up -d
```

### :keyboard: Command Line Arguments

```bash
./target/release/proxmox-mcp-rs --help
```

Arguments:
- `--config`, `-c`: Path to a configuration file (TOML, JSON, or YAML).
- `--host`, `-H`: Proxmox Host (e.g., `192.168.1.10`).
- `--port`, `-p`: Proxmox Port (default: `8006`).
- `--user`, `-u`: Proxmox User (e.g., `root@pam`).
- `--password`, `-P`: Proxmox Password (optional if using token).
- `--token-name`, `-n`: API Token Name (e.g., `mytoken`).
- `--token-value`, `-v`: API Token Secret.
- `--no-verify-ssl`, `-k`: Disable SSL verification (useful for self-signed certs).
- `--log-level`, `-L`: Log level (error, warn, info, debug, trace) (default: `info`).
- `--log-file-enable`: Enable logging to a file (default: `false`).
- `--log-dir`: Directory for log files (default: `.`).
- `--log-filename`: Log filename prefix (default: `proxmox-mcp-rs.log`).
- `--log-rotate`: Log rotation strategy (daily, hourly, never) (default: `daily`).
- `--server-type`, `-t`: Server type (`stdio` or `http`) (default: `stdio`).
- `--http-host`: HTTP Listen Host (default: `0.0.0.0`).
- `--http-port`, `-l`: HTTP Listen Port (default: `3000`).
---
- `PROXMOX_SERVER_TYPE` (`stdio` or `http`)
- `PROXMOX_HTTP_HOST` (default: `0.0.0.0`)
- `PROXMOX_HTTP_PORT` (default: `3000`)

### :gear: Configuration File

The server can load configuration from a file named `config.toml`, `config.yaml`, or `config.json` in the current directory, or via the `--config` flag. See `config.toml.example` for details.

### :earth_africa: Environment Variables

You can also configure the server using environment variables:
- `PROXMOX_CONFIG`: Path to a configuration file.
- `PROXMOX_HOST`
- `PROXMOX_PORT`
- `PROXMOX_USER`
- `PROXMOX_PASSWORD`
- `PROXMOX_TOKEN_NAME`
- `PROXMOX_TOKEN_VALUE`
- `PROXMOX_NO_VERIFY_SSL` (set to `true` to disable verification)
- `PROXMOX_LOG_LEVEL`
- `PROXMOX_LOG_FILE_ENABLE` (set to `true` to enable)
- `PROXMOX_LOG_DIR`
- `PROXMOX_LOG_FILENAME`
- `PROXMOX_LOG_ROTATE`
- `PROXMOX_SERVER_TYPE` (`stdio` or `http`)
- `PROXMOX_HTTP_HOST` (default: `0.0.0.0`)
- `PROXMOX_HTTP_PORT` (default: `3000`)

### :robot: Configuration Example (Claude Desktop)

Add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "proxmox": {
      "command": "/path/to/proxmox-mcp-rs/target/release/proxmox-mcp-rs",
      "args": [
        "--host", "192.168.1.10",
        "--port", "8006",
        "--user", "root@pam",
        "--password", "yourpassword",
        "--no-verify-ssl"
      ]
    }
  }
}
```

### :whale: Configuration Example (Docker for Claude Code/Desktop)

If you prefer to run the server via Docker, use the following configuration:

```json
{
  "mcpServers": {
    "proxmox-docker": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e", "PROXMOX_HOST=192.168.1.10",
        "-e", "PROXMOX_USER=root@pam",
        "-e", "PROXMOX_PASSWORD=yourpassword",
        "-e", "PROXMOX_NO_VERIFY_SSL=true",
        "proxmox-mcp-rs"
      ]
    }
  }
}
```

## :handshake: Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](.github/CONTRIBUTING.md) for guidelines.

## :balance_scale: License

​[​Apache License 2.0](https://raw.githubusercontent.com/nicholaswilde/proxmox-mcp-rs/refs/heads/main/LICENSE)

## :writing_hand: Author

​This project was started in 2026 by [Nicholas Wilde][2].

[2]: <https://github.com/nicholaswilde/>
