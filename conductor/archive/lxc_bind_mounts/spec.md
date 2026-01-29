# Specification: LXC Bind Mount Tools

## Goal
Enable users to configure bind mounts (mount points) for LXC containers via MCP tools. Bind mounts allow sharing directories from the Proxmox host to the container.

## Features
1.  **Add Bind Mount**: Add a new bind mount (mp0, mp1...) to a container.
2.  **Remove Bind Mount**: Remove a bind mount from a container.

## API Endpoints
-   PUT `/nodes/{node}/lxc/{vmid}/config`: Update Container config (add/remove mount points).
    -   Parameter: `mp[n]=volume,mp=/path/in/ct,...`
    -   For bind mounts: `mp[n]=/host/path,mp=/container/path`

## Tools to Add
-   `add_lxc_mountpoint`: Add a bind mount or volume mount point to an LXC container.
-   `remove_lxc_mountpoint`: Remove a mount point from an LXC container.
