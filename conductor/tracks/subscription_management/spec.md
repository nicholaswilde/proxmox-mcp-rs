# Specification: Subscription Management Tools

## Goal
Implement MCP tools to manage Proxmox VE subscription keys. This allows users to view current subscription status and upload new subscription keys.

## Features
1.  **List Subscriptions**: View the status of the subscription for a specific node (e.g., community, enterprise, active/inactive).
2.  **Set Subscription**: Upload a new subscription key to a node.
3.  **Update Subscription**: Trigger a check/update of the subscription status.

## API Endpoints
-   GET `/nodes/{node}/subscription`: Get subscription info.
-   POST `/nodes/{node}/subscription`: Set subscription key (parameter: `key`).
-   PUT `/nodes/{node}/subscription`: Update/Check subscription status.

## Tools to Add
-   `get_subscription_info`: Get subscription status for a node.
-   `set_subscription_key`: Set a new subscription key.
-   `check_subscription`: Force update/check of the subscription.
