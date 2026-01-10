# :crab: Proxmox MCP RS :robot:

[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)
[![ci](https://img.shields.io/github/actions/workflow/status/nicholaswilde/proxmox-mcp-rs/ci.yml?label=ci&style=for-the-badge&branch=main)](https://github.com/nicholaswilde/proxmox-mcp-rs/actions/workflows/ci.yml)

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
  - `list_backups`: List backups on a storage.
  - `create_backup`: Create a backup (vzdump).
  - `restore_backup`: Restore a VM or Container from a backup.
  - `get_task_status`: Get the status of a specific task (UPID).
  - `list_tasks`: List recent tasks on a node.
  - `read_task_log`: Read the log of a specific task (UPID).
  - `wait_for_task`: Wait for a task to finish (with timeout).
  - `get_vm_config`: Get the configuration of a VM or Container.
  - `list_networks`: List network interfaces and bridges on a node.
  - `list_storage`: List all storage on a node.
  - `list_isos`: List ISO images on a specific storage.
  - `get_cluster_status`: Get cluster status information.
  - `get_cluster_log`: Read cluster log.
  - `list_firewall_rules`: List firewall rules.
  - `add_firewall_rule`: Add a firewall rule.
  - `delete_firewall_rule`: Delete a firewall rule.
  - `add_disk`: Add a virtual disk to a VM or Container.
  - `remove_disk`: Remove (detach/delete) a virtual disk.
  - `add_network`: Add a network interface to a VM or Container.
  - `remove_network`: Remove a network interface.
  - `get_node_stats`: Get RRD statistics for a node.
  - `get_vm_stats`: Get RRD statistics for a VM or Container.
- **Resources:**
  - `proxmox://vms`: Live JSON list of all VMs and Containers.

## :hammer_and_wrench: Build

To build the project, you need a Rust toolchain installed.

```bash
cargo build --release
```

The binary will be available at `target/release/proxmox-mcp-rs`.

## :books: Documentation & Completions

### Generating Assets
You can generate man pages and shell completions (Bash, Zsh, Fish) using the included generator:

```bash
cargo run --example gen_manual
```

The assets will be created in the `assets/` directory:
- `assets/man/`: Man pages.
- `assets/completions/`: Shell completion scripts.

These assets are also bundled with every [GitHub Release](https://github.com/nicholaswilde/proxmox-mcp-rs/releases).

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
- `--http-auth-token`: HTTP Auth Token (Bearer or query param).
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
- `PROXMOX_HTTP_AUTH_TOKEN`

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
