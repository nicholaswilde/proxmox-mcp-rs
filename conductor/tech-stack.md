# Technology Stack - proxmox-mcp-rs

## Core
- **Language:** [Rust](https://www.rust-lang.org/) (Edition 2021) - Chosen for performance, memory safety, and concurrency guarantees.
- **Runtime:** [Tokio](https://tokio.rs/) - High-performance asynchronous runtime for managing concurrent API calls and server operations.

## Networking & API
- **HTTP Client:** [Reqwest](https://docs.rs/reqwest/) - Robust HTTP client for interacting with the Proxmox VE REST API.
- **Server Framework:** [Axum](https://docs.rs/axum/) - Ergonmic and efficient web framework for the optional HTTP/SSE server mode.
- **Protocol:** JSON-RPC 2.0 - Standard transport for the Model Context Protocol (MCP).

## Data Handling
- **Serialization:** [Serde](https://serde.rs/) / [Serde JSON](https://docs.rs/serde_json/) - For parsing Proxmox API responses and MCP JSON-RPC messages.
- **Configuration:** [Config-rs](https://docs.rs/config/) - Flexible configuration management supporting TOML, environment variables, and CLI overrides.

## Observability & CLI
- **Logging:** [Tracing](https://tokio.rs/blog/2019-09-tracing) / [Tracing-Subscriber](https://docs.rs/tracing-subscriber/) - Comprehensive instrumentation and logging for debugging and audit trails.
- **CLI Parsing:** [Clap](https://docs.rs/clap/) - Powerful command-line argument parser for configuration and operational control.

## Testing
- **API Mocking:** [WireMock](https://docs.rs/wiremock/) - For integration testing without requiring a live Proxmox instance.
- **Utilities:** [Tempfile](https://docs.rs/tempfile/) - For testing file-based logging and configuration loading.
