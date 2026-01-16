# Specification: Enhanced Error Reporting

## Goal
Improve error reporting to provide more context and clarity to MCP clients when operations fail.

## Current State
Errors are currently returned as generic strings or simple `anyhow::Error` messages, which often lack specific details (e.g., UPID, HTTP status codes from Proxmox).

## Requirements
1.  **Contextual Errors**: Include relevant IDs (VMID, Node, Storage) in error messages.
2.  **Proxmox API Errors**: Propagate HTTP status codes and API error messages (e.g., "401 Unauthorized", "500 Internal Server Error").
3.  **Task Errors**: specific handling for Task UPID failures (e.g. timeout or non-OK exit status).
4.  **Structure**: Use a custom Error type or improved `anyhow` context to structure these errors before converting to JSON-RPC errors.
