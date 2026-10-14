# Specification: Cluster Management Tools

## Goal
Implement MCP tools to manage Proxmox VE clusters, specifically allowing creating a new cluster and joining an existing one.

## Features
1.  **Create Cluster**: Initialize a new Proxmox VE cluster on the current node.
2.  **Join Information**: Get the join information (IP, fingerprint, etc.) needed for other nodes to join.
3.  **Join Cluster**: Join the current node to an existing cluster using IP, password, and fingerprint.

## API Endpoints
-   POST `/cluster/config`: Create a new cluster. Params: `clustername`.
-   GET `/cluster/config/join`: Get join information.
-   POST `/cluster/config/join`: Join a cluster. Params: `hostname`, `password`, `fingerprint`.

## Tools to Add
-   `create_cluster`: Create a new cluster.
-   `get_cluster_join_info`: Get the join info for the current cluster.
-   `join_cluster`: Join an existing cluster.
