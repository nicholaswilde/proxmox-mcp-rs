# Specification - Add Version CLI Argument

## Overview
Add a command-line argument to the `proxmox-mcp-rs` binary to display its current version information. This follows standard CLI conventions and allows users and automated tools to easily identify the installed version.

## Functional Requirements
- **Version Flag:** Support `--version` and the short alias `-V`.
- **Output Format:** Display the package name and version number (e.g., `proxmox-mcp-rs 0.3.29`).
- **Execution Flow:** When the version flag is provided, the application must print the version information to standard output and exit immediately with a success code (0).
- **Priority:** The version flag should take precedence over other arguments (e.g., it shouldn't require a host or password to be set if `--version` is present).

## Technical Requirements
- Use the existing `clap` configuration in `src/cli.rs`.
- Leverage `CARGO_PKG_VERSION` from the environment at build time.

## Acceptance Criteria
- Running `proxmox-mcp-rs --version` prints the version and exits 0.
- Running `proxmox-mcp-rs -V` prints the version and exits 0.
- The output format matches `proxmox-mcp-rs x.y.z`.
