# Initial Concept
A Rust implementation of a Proxmox MCP (Model Context Protocol) server to manage Proxmox VE resources (nodes, VMs, containers, etc.).

# Product Definition - proxmox-mcp-rs

## Target Users
The primary users of **proxmox-mcp-rs** are DevOps engineers and system administrators who utilize MCP-compatible LLM clients (such as Claude Desktop or Claude Code) to manage their Proxmox VE infrastructure. It serves as a bridge between high-level AI-driven management and low-level virtualization infrastructure.

## Core Goals
- **High Performance:** Provide a significantly faster, single-binary alternative to existing Python-based Proxmox MCP implementations.
- **Resource Efficiency:** Maintain a minimal resource footprint suitable for local execution or containerized environments.
- **Reliability:** Leverage Rust's type safety and concurrency model to ensure robust interaction with the Proxmox API.

## Technical Features
- **Protocol:** JSON-RPC 2.0 over Stdio (MCP standard).
- **Authentication:** Proxmox User/Password (Ticket-based) or API Token.
- **Logging:** Configurable log levels, console output (stderr), and optional file logging with rotation (daily, hourly).
- **Dual Transport:** Stdio (JSON-RPC 2.0) and HTTP (SSE/POST) with optional authentication.

## Key Features
- **Comprehensive Proxmox Management:** Full lifecycle control over VMs, LXC containers, snapshots, and backups.
- **Cluster & Resource Management:** Tools for managing nodes, pools, storage, and networking across a Proxmox cluster.
- **Monitoring:** Async task tracking (UPID) and live resource reading (MCP Resources).
- **Console Access:** Get URLs for NoVNC, xterm.js, or Spice consoles.
- **System Management:** Manage system services, APT updates, and firewall rules on Proxmox nodes.
- **Cloud-Init:** Configure Cloud-Init settings for VMs.
- **Guest Agent:** Execute commands, read/write files inside VMs via QEMU Guest Agent.
