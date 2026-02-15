# Implementation Plan: MCP Token Optimization (mcp_optimization_20260215)

## Phase 1: Benchmarking & Analysis
- [ ] Task: Create a baseline measurement script to count tokens in `list_tools` and sample tool responses (e.g., `list_vms`, `get_node_status`).
- [ ] Task: Identify specific redundant or low-value fields in `proxmox::vm::VmSummary`, `proxmox::system::NodeStatus`, and other key structs.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Benchmarking & Analysis' (Protocol in workflow.md)

## Phase 2: Tool Description Optimization
- [ ] Task: Refine `src/mcp.rs` tool-level descriptions for conciseness using AI-assisted drafting.
- [ ] Task: Refine `src/mcp.rs` argument-level descriptions for conciseness.
- [ ] Task: Verify reduction in `list_tools` token count against baseline.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Tool Description Optimization' (Protocol in workflow.md)

## Phase 3: Response Payload Thinning
- [ ] Task: Implement field filtering for VM and Container listing responses (Moderate reduction).
- [ ] Task: Implement field filtering for Node and Storage status responses.
- [ ] Task: Update `tests/mcp_integration.rs` to reflect thinned schemas (if necessary) and verify all tests pass.
- [ ] Task: Verify reduction in average response tokens against baseline.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Response Payload Thinning' (Protocol in workflow.md)

## Phase 4: Resource & Schema Refinement
- [ ] Task: Optimize MCP resource names and descriptions in `src/mcp.rs`.
- [ ] Task: Review and compress JSON schema definitions for complex tool arguments.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Resource & Schema Refinement' (Protocol in workflow.md)

## Phase 5: Final Validation
- [ ] Task: Perform final token count measurements and compare against Acceptance Criteria.
- [ ] Task: Manual verification of agent capability with the optimized server.
- [ ] Task: Conductor - User Manual Verification 'Phase 5: Final Validation' (Protocol in workflow.md)
