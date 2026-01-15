# Proxmox MCP Server (Rust)

[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)
[![ci](https://img.shields.io/github/actions/workflow/status/nicholaswilde/proxmox-mcp-rs/ci.yml?label=ci&style=for-the-badge&branch=main)](https://github.com/nicholaswilde/proxmox-mcp-rs/actions/workflows/ci.yml)

A Rust implementation of a Proxmox MCP (Model Context Protocol) server. This server connects to a Proxmox VE instance and exposes tools to manage nodes, VMs, and containers via the Model Context Protocol.

It is designed to be a faster, single-binary alternative to Python-based implementations.

## :rocket: Quick Start

You can run the server directly using `docker run`. You must provide your Proxmox credentials via environment variables.

```bash
docker run --rm -i \
  -e PROXMOX_HOST="192.168.1.10" \
  -e PROXMOX_USER="root@pam" \
  -e PROXMOX_PASSWORD="yourpassword" \
  -e PROXMOX_NO_VERIFY_SSL="true" \
  nicholaswilde/proxmox-mcp-rs
```

**Note:** The `-i` flag is crucial for Stdio-based MCP communication.

## :vhs: Docker Compose

Create a `compose.yaml` file:

```yaml
services:
  proxmox-mcp:
    image: nicholaswilde/proxmox-mcp-rs:latest
    stdin_open: true # Equivalent to -i
    environment:
      - PROXMOX_HOST=192.168.1.10
      - PROXMOX_USER=root@pam
      - PROXMOX_PASSWORD=yourpassword
      - PROXMOX_NO_VERIFY_SSL=true
      # Optional: HTTP Server Mode
      # - PROXMOX_SERVER_TYPE=http
      # - PROXMOX_HTTP_PORT=3000
    # ports:
    #   - "3000:3000" # Uncomment if using HTTP mode
```

Run with:
```bash
docker compose up -d
```

## :gear: Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PROXMOX_HOST` | Proxmox Host IP or Domain | Required |
| `PROXMOX_PORT` | Proxmox Port | `8006` |
| `PROXMOX_USER` | User (e.g., `root@pam`) | Required |
| `PROXMOX_PASSWORD` | User Password | Required (or Token) |
| `PROXMOX_TOKEN_NAME` | API Token Name | Optional |
| `PROXMOX_TOKEN_VALUE` | API Token Secret | Optional |
| `PROXMOX_NO_VERIFY_SSL`| Set `true` to disable SSL verification | `false` |
| `PROXMOX_SERVER_TYPE` | `stdio` or `http` | `stdio` |
| `PROXMOX_HTTP_PORT` | Port for HTTP server | `3000` |
| `PROXMOX_LOG_LEVEL` | Log level (`error`, `warn`, `info`, `debug`, `trace`) | `info` |

### Configuration File

You can also mount a configuration file to `/app/config.toml` (or `.yaml`, `.json`).

```bash
docker run --rm -i \
  -v $(pwd)/config.toml:/app/config.toml \
  nicholaswilde/proxmox-mcp-rs
```

## :robot: Claude Desktop Configuration

To use this Docker image with Claude Desktop, add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "proxmox": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e", "PROXMOX_HOST=192.168.1.10",
        "-e", "PROXMOX_USER=root@pam",
        "-e", "PROXMOX_PASSWORD=yourpassword",
        "-e", "PROXMOX_NO_VERIFY_SSL=true",
        "nicholaswilde/proxmox-mcp-rs"
      ]
    }
  }
}
```

## :hammer_and_wrench: Tools Provided

This server provides extensive tools for managing your Proxmox cluster, including:
- **Node Management:** `list_nodes`, `get_node_stats`
- **VM/LXC Lifecycle:** `start`, `stop`, `shutdown`, `reboot`, `reset`, `create`, `delete`
- **Snapshots:** `list`, `create`, `rollback`, `delete`
- **Backups:** `list`, `create`, `restore`
- **Resources:** `update_container_resources`, `add_disk`, `remove_disk`
- **Network:** `list_networks`, `add_network`

For a full list of tools, please refer to the [GitHub Repository](https://github.com/nicholaswilde/proxmox-mcp-rs).
