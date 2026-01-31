# Specification: Advanced Firewall and Network Tools

## Goal
Extend firewall management with IPSet support and provide access to node network configuration.

## Features
1. **Firewall IPSets**: Manage IPSets (collections of IP addresses/CIDRs) used in firewall rules.
2. **Node Network Config**: Retrieve the low-level network configuration of a node.

## Tools to Add
- `list_firewall_ipsets`: List IPSets at cluster or node level.
- `create_firewall_ipset`: Create a new IPSet.
- `delete_firewall_ipset`: Delete an IPSet.
- `list_firewall_ipset_entries`: List entries in an IPSet.
- `add_firewall_ipset_entry`: Add an entry to an IPSet.
- `remove_firewall_ipset_entry`: Remove an entry from an IPSet.
- `get_node_network_config`: Get raw network configuration for a node.
