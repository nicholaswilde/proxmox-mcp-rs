# Specification: Coverage >90% and Coveralls.io Integration

## Overview
This track aims to increase the code coverage of the `proxmox-mcp-rs` project to over 90% and integrate manual reporting to [Coveralls.io](https://coveralls.io/). This will ensure high reliability, catch regressions early, and align with the project's core goals of high performance and reliability.

## Functional Requirements
- **Comprehensive Coverage Expansion:**
  - Expand unit and integration tests for `src/mcp.rs` to cover all MCP tool handlers and protocol logic.
  - Increase coverage for all modules in `src/proxmox/` (VMs, storage, networking, etc.).
  - Improve test coverage for `src/http_server.rs`, `src/cli.rs`, and `src/settings.rs`.
- **Tooling Integration:**
  - Utilize `cargo-llvm-cov` for generating precise coverage reports.
  - Update `Taskfile.yml` with tasks for generating local coverage reports (HTML and LCOV).
- **Coveralls.io Integration:**
  - Configure manual upload of coverage reports to Coveralls.io using the LCOV format.
  - Update `README.md` to include a Coveralls coverage badge.

## Non-Functional Requirements
- **Coverage Target:** Total line coverage must exceed 90%.
- **Automation:** Coverage tasks in the `Taskfile.yml` should be non-interactive and suitable for local use.
- **Performance:** Coverage instrumentation should not significantly degrade test execution time for local development.

## Acceptance Criteria
- [ ] Total project code coverage is >90% as reported by `cargo-llvm-cov`.
- [ ] New `Taskfile.yml` tasks (`task cover`, `task cover:html`) work as expected.
- [ ] `README.md` contains a valid Coveralls badge linking to the project's dashboard.
- [ ] A successful manual upload to Coveralls.io has been verified.

## Out of Scope
- Automated CI integration for Coveralls (manual upload only for this track).
- 100% coverage (target is >90%).
