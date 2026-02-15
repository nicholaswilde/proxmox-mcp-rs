# :crab: Proxmox MCP RS :robot:

[![Coveralls](https://img.shields.io/coveralls/github/nicholaswilde/proxmox-mcp-rs/main?style=for-the-badge&logo=coveralls)](https://coveralls.io/github/nicholaswilde/proxmox-mcp-rs?branch=main)
[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)
[![ci](https://img.shields.io/github/actions/workflow/status/nicholaswilde/proxmox-mcp-rs/ci.yml?label=ci&style=for-the-badge&branch=main&logo=github-actions)](https://github.com/nicholaswilde/proxmox-mcp-rs/actions/workflows/ci.yml)

> [!WARNING]
> This project is currently in active development (v0.x.x) and is **not production-ready**. Features may change, and breaking changes may occur without notice.

A Rust implementation of a Proxmox [MCP (Model Context Protocol) server](https://modelcontextprotocol.io/docs/getting-started/intro). This server connects to a Proxmox VE instance and exposes tools to manage nodes, VMs, and containers via the Model Context Protocol.

It is designed to be a faster, single-binary alternative to the Python-based [ProxmoxMCP-Plus](https://github.com/RekklesNA/ProxmoxMCP-Plus).

## :sparkles: Features

- **Protocol:** JSON-RPC 2.0 over Stdio (MCP standard).
- **Multi-Instance:** Support for multiple Proxmox instances in a single configuration.
- **Authentication:** Proxmox User/Password (Ticket-based) or API Token.
- **Logging:** Configurable log levels, console output (stderr), and optional file logging with rotation (daily, hourly).
- **Tools:**
  - All tools support an optional `instance` argument to target a specific Proxmox environment.

  **Consolidated Management**
  - `vm_power_action`: Perform power actions (start, stop, shutdown, reboot, reset, suspend, resume) on a VM or Container.
  - `manage_resource`: Resource lifecycle management (create, delete, clone, migrate, template).
  - `manage_resource_config`: Update hardware, disks, network, Cloud-Init, or execute commands (via Agent).
  - `manage_snapshot_backup`: Create/manage snapshots, backups, and backup schedules.
  - `manage_node_system`: Manage node services, APT repositories, certificates, and subscriptions.
  - `manage_cluster_config`: Configure cluster-wide storage, SDN, firewall, pools, roles, users, ACLs, HA, and Ceph.
  - `manage_tags`: Add, remove, or set tags on resources.

  **Listings**
  - `list_nodes`: List all nodes in the cluster.
  - `list_vms`: List all VMs and LXC containers.
  - `list_containers`: List only LXC containers.
  - `list_storage`: List node storage.
  - `list_cluster_storage`: List all storage definitions.
  - `list_networks`: List node network interfaces.
  - `list_firewall_rules`: List firewall rules for node/VM.
  - `list_firewall_aliases`: List firewall aliases.
  - `list_security_groups`: List firewall security groups.
  - `list_tasks`: List recent tasks on a node.
  - `list_backups`: List backups on a storage.
  - `list_snapshots`: List snapshots for a VM/Container.
  - `list_templates`: List container templates.
  - `list_isos`: List ISO images.
  - `list_pools`: List resource pools.
  - `list_replication_jobs`: List replication jobs.
  - `list_ha_resources`: List HA resources.
  - `list_users`: List cluster users.
  - `list_roles`: List roles.
  - `list_acls`: List ACLs.
  - `list_apt_updates`: List available APT updates.
  - `list_services`: List node services.
  - `list_certificates`: List node certificates.
  - `list_pci_devices` / `list_usb_devices`: List hardware available for passthrough.
  - `list_pci_mappings` / `list_usb_mappings`: List cluster resource mappings.
  - `list_metric_servers`: List configured metric servers.
  - `list_sdn_zones` / `list_sdn_vnets`: List SDN configuration.
  - `list_ceph_pools` / `list_ceph_osds`: List Ceph status and resources.
  - `list_backup_schedules`: List backup schedules.

  **Stats & Logs**
  - `get_cluster_status`: Get cluster status information.
  - `get_cluster_log`: Read cluster log.
  - `get_node_stats`: Get RRD statistics for a node.
  - `get_vm_stats`: Get RRD statistics for a VM/Container.
  - `get_vm_config`: Get the full configuration of a VM/Container.
  - `get_task_status`: Get the status of a specific task (UPID).
  - `read_task_log`: Read the log of a specific task (UPID).
  - `get_ceph_status`: Get Ceph cluster status.

  **Utilities**
  - `bulk_vm_action`: Perform power actions on multiple VMs simultaneously.
  - `scan_storage_remote`: Scan a remote server (NFS, CIFS) for target shares.
  - `download_url`: Download ISO/Template from a URL to storage.
  - `get_console_url`: Get URL for Proxmox web console.
  - `wait_for_task`: Wait for a task to finish.
  - `apply_sdn_changes`: Apply pending SDN changes.
  - `load_all_tools`: Load full tool catalog (when in Lazy Mode).
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
- `--lazy-mode`: Enable Lazy Loading mode. Starts with a minimal set of tools to save context tokens. Use the `load_all_tools` tool to load the full catalog.

### :gear: Configuration File

The server can load configuration from a file named `config.toml`, `config.yaml`, or `config.json` in the current directory, or via the `--config` flag. See `config.toml.example` for details.

### :gear: Multi-Instance Configuration

You can configure multiple Proxmox instances in your `config.toml` using the `[[instances]]` array. This allows you to manage several clusters or standalone nodes from a single MCP server.

```toml
# Default instance (legacy format)
host = "192.168.1.10"
user = "root@pam"
password = "..."

# Additional instances
[[instances]]
name = "lab"
host = "192.168.1.20"
user = "root@pve"
token_name = "..."
token_value = "..."

[[instances]]
name = "prod"
host = "pve.example.com"
user = "root@pam"
password = "..."
```

#### Targeting Instances
Every tool supports an optional `instance` argument. This argument matches the `name` (if provided) or the `host` of the instance.

If no `instance` is specified, the server uses the default instance (the top-level configuration or the first entry in the `[[instances]]` list).

**Example tool call (JSON-RPC):**
```json
{
  "method": "tools/call",
  "params": {
    "name": "list_vms",
    "arguments": {
      "instance": "lab"
    }
  }
}
```

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

**Multi-Instance via Environment Variables:**
You can configure multiple instances using the pattern `PROXMOX_INSTANCES__<INDEX>__<FIELD>`. Use double underscores `__` as separators.

Example:
```bash
PROXMOX_INSTANCES__0__HOST=192.168.1.10
PROXMOX_INSTANCES__0__USER=root@pam
PROXMOX_INSTANCES__0__PASSWORD=secret

PROXMOX_INSTANCES__1__NAME=seedbox
PROXMOX_INSTANCES__1__HOST=seedbox.example.com
PROXMOX_INSTANCES__1__USER=root@pve
PROXMOX_INSTANCES__1__TOKEN_NAME=mcp
PROXMOX_INSTANCES__1__TOKEN_VALUE=...
```

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

### :robot: Configuration Example (Docker for Claude Code/Desktop)

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

## :test_tube: Testing

### MCP Inspector
The easiest way to test the server interactively is using the MCP Inspector. It provides a web interface to call tools and inspect resources.

```bash
task inspector
```

If running on a remote host, you can specify the host and allowed origins:
```bash
task inspector HOST=0.0.0.0 ALLOWED_ORIGINS="*"
```

### Integration Tests
You can run the automated integration tests to verify the MCP protocol and Proxmox API connectivity:

```bash
python3 integration_test.py
```

## :handshake: Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](.github/CONTRIBUTING.md) for guidelines.

## :balance_scale: License

​[​Apache License 2.0](https://raw.githubusercontent.com/nicholaswilde/proxmox-mcp-rs/refs/heads/main/LICENSE)

## :writing_hand: Author

​This project was started in 2026 by [Nicholas Wilde][2].

[2]: <https://github.com/nicholaswilde/>
