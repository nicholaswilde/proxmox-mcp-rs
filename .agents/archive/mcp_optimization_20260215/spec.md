# Specification: MCP Token Optimization (mcp_optimization_20260215)

## Overview
This track focuses on reducing the token footprint of the MCP server during agent interactions. By optimizing tool definitions, argument descriptions, and response payloads, we aim to improve the agent's context window efficiency and reduce latency/cost.

## Functional Requirements
- **Description Refinement**: Systematically shorten all tool and argument descriptions in `src/mcp.rs`.
- **Payload Thinning**: Implement a "Moderate" reduction strategy for tool response payloads. Remove redundant fields, internal-only metadata, and fields that can be trivially inferred by the agent.
- **Tool Consolidation**: Group related granular tools into unified, multi-purpose tools (e.g., merging `start_vm`, `stop_vm`, etc., into a single `vm_power_action`) to reduce the total number of tools and their associated schema tokens.
- **Resource Metadata Optimization**: Update MCP resource names and descriptions to be more concise.
- **Schema Optimization**: Review JSON schema definitions for tools to ensure they are as compact as possible without losing structural integrity.
- **Documentation Sync**: Update `README.md` and any other user-facing documentation to reflect tool consolidation and schema changes.
- **Test Alignment**: Update all integration and unit tests to align with consolidated tools and thinned response payloads.

## Non-Functional Requirements
- **Compatibility**: Ensure that the optimized payloads still provide sufficient information for the agent to perform its tasks correctly.
- **Maintainability**: The optimization should not make the code significantly harder to read or extend.
- **Performance**: The thinning process should have negligible impact on server response time.

## Acceptance Criteria
- [ ] Total token count for the `list_tools` response is reduced by at least 20%.
- [ ] Total number of tools is reduced by at least 25% through consolidation.
- [ ] Average token count for common tool responses (e.g., `list_vms`, `get_vm_config`) is reduced by at least 15%.
- [ ] All existing integration tests (`tests/mcp_integration.rs`) pass with the optimized payloads.
- [ ] Manual verification confirms that the agent can still successfully execute complex tasks (e.g., "Move VM 100 to node pve2 and start it").

## Out of Scope
- Implementing different "verbosity levels" selectable by the client.
- Changing the underlying JSON-RPC protocol.
- Optimizing logging or internal server metrics.
