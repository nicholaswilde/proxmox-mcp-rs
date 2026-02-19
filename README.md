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
- **Token Optimized:** Consolidated tools and shortened descriptions reduce context window usage by ~85%.
- **Tools:**
  - All tools support an optional `instance` argument to target a specific Proxmox environment.

  ### Consolidated Management
  - `vm_power_action`: Unified power state transitions (start, stop, shutdown, reboot, reset, suspend, resume) for VMs and Containers.
  - `manage_resource`: Lifecycle operations (create, delete, clone, migrate, template).
  - `manage_resource_config`: Hardware updates (CPU, RAM), disk management, network configuration, Cloud-Init, and Guest Agent command execution.
  - `manage_snapshot_backup`: Unified management of snapshots, manual backups, and backup schedules.
  - `manage_node_system`: Service management, APT repository configuration, SSL certificates, and Proxmox subscriptions.
  - `manage_cluster_config`: Cluster-wide configuration for Storage, SDN, Firewall, Pools, Roles, Users, ACLs, HA, and Ceph.
  - `manage_tags`: Add, remove, or set resource tags.

  ### Listings
  - `list_nodes`: Cluster node status.
  - `list_vms` / `list_containers`: Resource discovery.
  - `list_storage` / `list_cluster_storage`: Storage status and definitions.
  - `list_networks`: Node network interfaces.
  - `list_firewall_rules` / `list_firewall_aliases` / `list_security_groups` / `list_security_group_rules`: Firewall inspection.
  - `list_tasks`: Node task history.
  - `list_backups` / `list_snapshots`: Backup and snapshot inventory.
  - `list_templates` / `list_isos`: Image and template discovery.
  - `list_pools` / `list_users` / `list_roles` / `list_acls`: Access control inspection.
  - `list_apt_updates` / `list_services` / `list_certificates` / `list_repositories`: System maintenance.
  - `list_pci_devices` / `list_usb_devices`: Hardware discovery.
  - `list_pci_mappings` / `list_usb_mappings`: Resource mapping inventory.
  - `list_metric_servers`: External monitoring config.
  - `list_sdn_zones` / `list_sdn_vnets`: Software Defined Network status.
  - `list_ceph_pools` / `list_ceph_osds` / `list_ceph_monitors`: Ceph storage status.
  - `list_backup_schedules`: Cluster backup jobs.

  ### Stats & Logs
  - `get_cluster_status` / `get_cluster_log`: Cluster-wide monitoring.
  - `get_node_stats` / `get_vm_stats`: Performance metrics (RRD).
  - `get_vm_config`: Full hardware/software configuration.
  - `get_task_status` / `read_task_log`: Detailed task monitoring.
  - `get_ceph_status`: Ceph health and status.
  - `get_storage_volume`: Details for a specific volume.
  - `get_pool_details`: Members and configuration of a pool.
  - `get_subscription_info`: Proxmox subscription status.
  - `get_apt_versions`: Package version details.

  ### Utilities
  - `bulk_vm_action`: Concurrent power actions on multiple IDs.
  - `scan_storage_remote`: Remote share discovery (NFS/CIFS).
  - `download_url`: Integrated ISO/Template downloader.
  - `get_console_url`: Generate console links (NoVNC/xterm.js).
  - `wait_for_task`: Synchronous task waiting.
  - `apply_sdn_changes`: Commit pending network changes.
  - `vm_agent_ping` / `vm_exec_status`: Guest agent health and command tracking.
  - `vm_read_file` / `vm_write_file`: File operations via guest agent.
  - `load_all_tools`: Catalog expansion for Lazy Mode.

- **Resources:**
  - `proxmox://vms`: Live JSON stream of all VMs and Containers.

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
- `--config`, `-c`: Path to configuration file.
- `--host`, `-H`: Proxmox Host.
- `--port`, `-p`: Proxmox Port (default: `8006`).
- `--user`, `-u`: Proxmox User.
- `--password`, `-P`: Proxmox Password (optional if using token).
- `--token-name`, `-n`: API Token Name.
- `--token-value`, `-v`: API Token Secret.
- `--no-verify-ssl`, `-k`: Disable SSL verification.
- `--log-level`, `-L`: Log level (error, warn, info, debug, trace).
- `--log-file-enable`: Enable file logging.
- `--log-dir`: Directory for logs.
- `--log-filename`: Log prefix.
- `--log-rotate`: hourly, daily, or never.
- `--server-type`, `-t`: `stdio` or `http`.
- `--http-host`: HTTP listen address.
- `--http-port`, `-l`: HTTP listen port.
- `--http-auth-token`: Optional Bearer token for HTTP security.
- `--lazy-mode`: Minimal initial tool list to save context tokens.

### :gear: Multi-Instance Configuration

Configure multiple Proxmox instances in `config.toml`:

```toml
# Default instance
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
```

Every tool supports an optional `instance` argument matching the `name` or `host`.

### :earth_africa: Environment Variables

- `PROXMOX_CONFIG`
- `PROXMOX_HOST`, `PROXMOX_PORT`, `PROXMOX_USER`, `PROXMOX_PASSWORD`
- `PROXMOX_TOKEN_NAME`, `PROXMOX_TOKEN_VALUE`
- `PROXMOX_NO_VERIFY_SSL`
- `PROXMOX_LOG_LEVEL`, `PROXMOX_LOG_FILE_ENABLE`, `PROXMOX_LOG_DIR`
- `PROXMOX_SERVER_TYPE`, `PROXMOX_HTTP_HOST`, `PROXMOX_HTTP_PORT`, `PROXMOX_HTTP_AUTH_TOKEN`
- `PROXMOX_LAZY_MODE`

**Multi-Instance:** `PROXMOX_INSTANCES__<INDEX>__<FIELD>` (e.g., `PROXMOX_INSTANCES__0__HOST`).

### :robot: Configuration Example (Claude Desktop)

```json
{
  "mcpServers": {
    "proxmox": {
      "command": "/path/to/proxmox-mcp-rs/target/release/proxmox-mcp-rs",
      "args": ["--config", "/path/to/config.toml"]
    }
  }
}
```

## :test_tube: Testing

### MCP Inspector
Interactive testing via web interface:
```bash
task inspector
```

### Integration Tests
Verify protocol and connectivity:
```bash
python3 integration_test.py
```

## :handshake: Contributing

Contributions are welcome! See [CONTRIBUTING.md](.github/CONTRIBUTING.md).

## :balance_scale: License

​[​Apache License 2.0](LICENSE)

## :writing_hand: Author

​Nicholas Wilde.
