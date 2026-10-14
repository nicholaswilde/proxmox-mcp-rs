# AGENTS.md

Instructions, conventions, and architectural context for AI agents working on `proxmox-mcp-rs`.

---

## 1. Project Overview & Architecture

**`proxmox-mcp-rs`** is a high-performance Rust implementation of a [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server for Proxmox VE. It bridges MCP-compatible LLM clients (such as Claude Desktop, Claude Code, or Antigravity) with Proxmox VE virtualization infrastructure.

### Core Goals
- **High Performance:** Fast, lightweight, single-binary alternative to Python-based implementations.
- **Resource Efficiency:** Minimal memory footprint suitable for local execution or containerized environments.
- **Token Efficiency:** Systematically concise tool and argument descriptions to minimize context window usage for LLM clients.
- **Reliability & Safety:** Strict type safety, memory safety, and robust error handling leveraging Rust's concurrency model.

### Key Capabilities
- **VM & Container Lifecycle:** Management of QEMU VMs (including templates, Cloud-Init, QEMU Guest Agent) and LXC containers (including bind mounts).
- **Cluster & Storage:** Nodes, resource pools, storage management, Ceph health/pools/OSDs, and remote storage scanning.
- **Hardware & Security:** PCI and USB device passthrough, firewall rules, security groups, aliases, and SSL certificates.
- **System Management:** Services, APT repositories and updates, subscription checks, and live RRD stats.
- **Dual Transport:**
  - **Stdio:** JSON-RPC 2.0 over standard I/O (default MCP standard).
  - **HTTP:** Optional Server-Sent Events (SSE) and POST transport with authentication via Axum.

---

## 2. Technology Stack

- **Language & Edition:** Rust (Edition 2021)
- **Async Runtime:** [Tokio](https://tokio.rs/) (`full` features)
- **HTTP Client:** [Reqwest](https://docs.rs/reqwest/) (with `rustls-tls`, `json`, `cookies`)
- **HTTP / SSE Server:** [Axum](https://docs.rs/axum/)
- **Protocol:** JSON-RPC 2.0
- **Serialization:** [Serde](https://serde.rs/) & [Serde JSON](https://docs.rs/serde_json/)
- **Configuration:** [Config-rs](https://docs.rs/config/) (TOML, environment variables, CLI overrides)
- **CLI Parsing:** [Clap](https://docs.rs/clap/) (derive and env features)
- **Observability:** [Tracing](https://tokio.rs/blog/2019-09-tracing), `tracing-subscriber`, `tracing-appender`, `tracing-log`
- **Testing & Mocking:**
  - [WireMock](https://docs.rs/wiremock/) for simulating Proxmox VE REST API endpoints.
  - End-to-end integration tests in `tests/mcp_integration.rs`.
  - [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) for code coverage.
  - [Tempfile](https://docs.rs/tempfile/) for file/logging tests.

---

## 3. Development Principles & Code Style

### Token Efficiency
- All tool definitions and argument descriptions must be short and functional imperatives (e.g., "List VMs", "Resize VM disk") to keep tool schemas compact in client context windows.

### Documentation Integrity
- **README.md:** Keep the tool list and features in `README.md` updated whenever tools are added or modified.
- **Infrastructure Configs:** Update `compose.yaml` and `config.toml.example` whenever new tools, functions, or settings are added or modified.
- **Preserve Comments:** Maintain existing comments and docstrings unrelated to your changes.

### Idiomatic Rust & Error Handling
- Use `anyhow` for rich error handling and return clean, meaningful JSON-RPC error responses.
- Keep execution paths efficient, especially for operations across large clusters.
- Run `cargo fmt` and `cargo clippy` to maintain codebase standards.

---

## 4. Testing & Quality Gates

### Test-Driven Development (TDD)
1. **Red Phase:** Write failing unit tests with `wiremock` simulating the Proxmox API response before writing implementation code.
2. **Green Phase:** Write the minimum code needed to pass the tests.
3. **Refactor:** Clean up while ensuring all tests continue to pass.

### Integration Testing Requirements
- Every new Proxmox function and MCP tool **must** have corresponding unit tests in `src/tests.rs` (or relevant module).
- **CRITICAL:** `tests/mcp_integration.rs` **MUST** be updated with a functional test case whenever a new MCP server tool is added.

### Quality Gates
Before submitting changes or completing a task, verify:
- [ ] `task test:ci` passes completely (fmt, clippy, unit & integration tests).
- [ ] Code coverage remains >80% (target >90%).
- [ ] `README.md`, `compose.yaml`, and `config.toml.example` are kept in sync.
- [ ] No compilation warnings or clippy lints.

---

## 5. Development Commands (`Taskfile.yml`)

Use `task` as the primary development entry point:

```bash
task check        # Check for compilation errors
task test         # Run unit tests
task fmt          # Format code using rustfmt
task lint         # Run lints using clippy
task build:debug  # Build debug binary
task test:ci      # Run full CI suite (formatting, linting, and tests)
```

---

## 6. Live Proxmox Testing Protocol

When verifying new tools against a live Proxmox VE instance:
1. Identify new tools via `git diff <last_tag> HEAD -- src/mcp.rs`.
2. Ensure valid connection settings exist in `config.toml` or environment variables.
3. Build release binary: `cargo build --release`.
4. Run a temporary Python script sending JSON-RPC messages via stdio:
   ```python
   import subprocess, json, sys
   def rpc(method, params=None, id=1):
       return {"jsonrpc": "2.0", "method": method, "params": params, "id": id}

   cmd = ["./target/release/proxmox-mcp-rs", "--config", "config.toml"]
   p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=sys.stderr, text=True, bufsize=0)

   # Initialize
   p.stdin.write(json.dumps(rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}})) + "\n")
   print(p.stdout.readline())

   # Call Tool
   p.stdin.write(json.dumps(rpc("tools/call", {"name": "<tool_name>", "arguments": {}})) + "\n")
   print(p.stdout.readline())
   p.terminate()
   ```
5. Document live testing results in `.agents/verification.md`.
6. Remove scratch test scripts.

---

## 7. Git & Release Workflow

### Commit Convention
Use Conventional Commits: `<type>(<scope>): <description>`
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting / whitespace
- `refactor`: Code restructure without behavioral changes
- `test`: Adding or updating tests
- `chore`: Maintenance tasks (dependencies, tooling, prompts)

### Release Process
Release automation is defined in `.agents/commands/release.toml`:
1. Read current version from `Cargo.toml`.
2. Increment patch version (e.g. `0.3.33` -> `0.3.34`).
3. Update `Cargo.toml`.
4. Run `cargo check` to update `Cargo.lock`.
5. Update version string in `README.md` (`[!WARNING]` section).
6. Git operations:
   - `git pull --rebase`
   - Stage `Cargo.toml`, `Cargo.lock`, `README.md`
   - Commit: `chore: Bump version to <version>`
   - Tag: `git tag -a v<version> -m "<version>"`
   - Push atomically: `git push --atomic origin main v<version>`

### Release Summary Convention
Release summaries are generated via `.agents/commands/release-summary.toml`:
- Categorize commits between tags into `### :rocket: **New Features**`, `### :bug: **Bug Fixes**`, `### :sparkles: **Improvements**`, `### :memo: **Documentation**`.
- Omit empty sections.
- Keep scopes (e.g. `(ceph): ...`).
- Include full changelog comparison link: `https://github.com/nicholaswilde/proxmox-mcp-rs/compare/<prev>...<latest>`.

---

## 8. Agent Workspace (`.agents/`)

The `.agents/` directory contains project-specific automation and tracking:
- **`.agents/commands/`**: Reusable automation commands (`release.toml`, `release-summary.toml`, `build-fix.toml`, `test-fix.toml`).
- **`.agents/settings.json`**: MCP server configurations for agent sessions.
- **`.agents/verification.md`**: Live cluster verification log of tested tools and endpoints.
- **`.agents/tracks/`**: Detailed roadmaps, specifications, and task plans for pending/active feature tracks:
  - `advanced_storage_20260131`: Disk resize, move disk, ZFS health.
  - `advanced_networking_20260131`: SDN, advanced firewall, security groups.
  - `node_maintenance_20260131`: Node maintenance, reboot, HA rules.
  - `pbs_integration_20260131`: Proxmox Backup Server backup and restore integration.
  - `guest_insights_20260131`: QEMU Guest Agent deep metrics and insights.
- **`.agents/archive/`**: Completed and archived track specifications and plans.
