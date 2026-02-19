# Product Guidelines - proxmox-mcp-rs

## Documentation Style
- **Technical Precision:** All documentation, error messages, and logs should use precise technical terminology related to Proxmox VE and the Model Context Protocol.
- **Conciseness for Efficiency:** All tool and argument descriptions should be systematically shortened to minimal functional imperatives (e.g., "List VMs") to minimize token usage and maximize agent context window.
- **Consistent Terminology:** Use consistent naming conventions that map directly to Proxmox API concepts (e.g., `vmid`, `node`, `storage`).
- **README Updates:** All Proxmox functions and tools must be documented in the `README.md`.

## Development Principles
- **Idiomatic Rust:** Code should follow standard Rust conventions and idioms. Use `cargo fmt` and `cargo clippy` to maintain quality.
- **Safety & Robustness:** Prioritize memory safety and robust error handling. Use `anyhow` or similar crates for meaningful error reporting back to the MCP client.
- **Performance:** Keep the execution path efficient, especially for resource-heavy operations like listing all VMs in a large cluster.
- **Test-Driven Reliability:** Every new Proxmox tool or core feature should be accompanied by unit tests using `wiremock` to simulate Proxmox API responses.
- **Testing Coverage:** Every Proxmox function and MCP tool must have corresponding unit tests in `src/tests.rs` (or relevant module). Additionally, `tests/mcp_integration.rs` MUST be updated with a functional test case whenever a new MCP server tool is added.
- **CI Verification:** Run `task test:ci` after every feature addition to ensure formatting, linting, and tests all pass.
- **Versioning:** When asked to create a new git tag, always update the version in `Cargo.toml` to match the new tag version. When creating the tag, use the `-m` argument to add a descriptive comment (e.g., `git tag -a v0.3.22 -m "v0.3.22"`).
