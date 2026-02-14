# Plan: Coverage >90% and Coveralls.io Integration

## Phase 1: Coverage Tooling and Baseline
- [ ] Task: Install and configure `cargo-llvm-cov` locally.
- [ ] Task: Add coverage tasks (`cover`, `cover:html`, `cover:lcov`) to `Taskfile.yml`.
- [ ] Task: Generate a baseline coverage report to identify current gaps.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Coverage Tooling and Baseline' (Protocol in workflow.md)

## Phase 2: Core Logic and Proxmox Client Coverage
- [ ] Task: Write tests to increase coverage for `src/mcp.rs` to >90%.
- [ ] Task: Write tests to increase coverage for `src/proxmox/*.rs` modules to >90%.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Core Logic and Proxmox Client Coverage' (Protocol in workflow.md)

## Phase 3: Server and Utility Coverage
- [ ] Task: Write tests to increase coverage for `src/http_server.rs` to >90%.
- [ ] Task: Write tests to increase coverage for `src/cli.rs` and `src/settings.rs` to >90%.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Server and Utility Coverage' (Protocol in workflow.md)

## Phase 4: Coveralls Integration and Finalization
- [ ] Task: Perform final coverage verification to ensure total line coverage is >90%.
- [ ] Task: Update `README.md` with the Coveralls.io coverage badge.
- [ ] Task: Manually upload the final LCOV report to Coveralls.io and verify the dashboard.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Coveralls Integration and Finalization' (Protocol in workflow.md)
