# Specification: Proxmox Backup Server (PBS) Integration

## Goal
Add tools to interact with Proxmox Backup Server (PBS) remotes configured in Proxmox VE.

## Features
1. **PBS Datastore Listing**: List datastores and check their usage.
2. **PBS Snapshot Listing**: List backup snapshots for a specific datastore or resource.

## Tools to Add
- `list_pbs_datastores`: List datastores on a PBS remote.
- `list_pbs_snapshots`: List snapshots stored on a PBS remote.
