# Implementation Plan - Add Version CLI Argument

## Phase 1: Implementation & Verification
- [x] Task: Update the CLI configuration to use the standard version format.
    - [ ] Sub-task: Write a unit test in `src/tests.rs` to verify that the CLI version string matches the format `x.y.z` (no `v` prefix, no git metadata) as per the specification.
    - [ ] Sub-task: Modify `src/cli.rs` to use `CARGO_PKG_VERSION` directly for the `version` attribute, ensuring it ignores the custom `PROJECT_VERSION` if it contains git metadata.
- [ ] Task: Conductor - User Manual Verification 'Implementation' (Protocol in workflow.md)
