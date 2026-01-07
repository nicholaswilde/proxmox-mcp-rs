# proxmox-mcp-rs

A Rust implementation of a Proxmox MCP (Model Context Protocol) server. This server connects to a Proxmox VE instance and exposes tools to manage nodes, VMs, and containers via the Model Context Protocol.

It is designed to be a faster, single-binary alternative to the Python-based [proxmox-mcp-plus](https://github.com/nicholaswilde/proxmox-mcp-plus).

## Features

- **Protocol:** JSON-RPC 2.0 over Stdio (MCP standard).
- **Authentication:** Proxmox User/Password (Ticket-based).
- **Tools:**
  - `list_nodes`: List all nodes in the cluster.
  - `list_vms`: List all VMs and LXC containers.
  - `start_vm`: Start a VM/Container.
  - `stop_vm`: Stop (Power Off) a VM/Container.
  - `shutdown_vm`: Gracefully shutdown a VM/Container.
  - `reboot_vm`: Reboot a VM/Container.

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
- `--host`: Proxmox Host (e.g., `192.168.1.10:8006`).
- `--user`: Proxmox User (e.g., `root@pam`).
- `--password`: Proxmox Password.
- `--no-verify-ssl`: Disable SSL verification (useful for self-signed certs).

### Environment Variables

You can also configure the server using environment variables:
- `PROXMOX_HOST`
- `PROXMOX_USER`
- `PROXMOX_PASSWORD`

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