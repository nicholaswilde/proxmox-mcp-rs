# Product Guidelines - proxmox-mcp-rs

## Documentation Style
- **Technical Precision:** All documentation, error messages, and logs should use precise technical terminology related to Proxmox VE and the Model Context Protocol.
- **Clarity over Brevity:** Ensure that tool descriptions and parameter requirements are explicitly stated to minimize ambiguity for LLM clients.
- **Consistent Terminology:** Use consistent naming conventions that map directly to Proxmox API concepts (e.g., `vmid`, `node`, `storage`).

## Development Principles
- **Idiomatic Rust:** Code should follow standard Rust conventions and idioms. Use `cargo fmt` and `cargo clippy` to maintain quality.
- **Safety & Robustness:** Prioritize memory safety and robust error handling. Use `anyhow` or similar crates for meaningful error reporting back to the MCP client.
- **Performance:** Keep the execution path efficient, especially for resource-heavy operations like listing all VMs in a large cluster.
- **Test-Driven Reliability:** Every new Proxmox tool or core feature should be accompanied by unit tests using `wiremock` to simulate Proxmox API responses.
