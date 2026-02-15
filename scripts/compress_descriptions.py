import sys
import re

def compress_descriptions():
    path = 'src/mcp.rs'
    with open(path, 'r') as f:
        content = f.read()

    replacements = {
        '"VM/CT power action"': '"Power action"',
        '"Create/Delete VM/CT"': '"Lifecycle"',
        '"Update VM/CT config"': '"Config"',
        '"Snapshot/Backup tools"': '"Snapshots/Backups"',
        '"Node system tools"': '"Node system"',
        '"Cluster config tools"': '"Cluster config"',
        '"Manage resource tags"': '"Manage tags"',
        '"List cluster nodes"': '"List nodes"',
        '"Get cluster status"': '"Cluster status"',
        '"Read cluster log"': '"Cluster log"',
        '"Get node stats"': '"Node stats"',
        '"List all VMs"': '"List VMs"',
        '"List all LXC"': '"List LXC"',
        '"List node storage"': '"List storage"',
        '"List cluster storage"': '"List storage (cluster)"',
        '"List CT templates"': '"List templates"',
        '"List ISOs"': '"List ISOs"',
        '"List node networks"': '"List networks"',
        '"List firewall aliases"': '"List aliases"',
        '"List security groups"': '"List security groups"',
        '"List group rules"': '"List security rules"',
        '"List node tasks"': '"List tasks"',
        '"List node services"': '"List services"',
        '"List APT repos"': '"List repositories"',
        '"List APT updates"': '"List updates"',
        '"List node certs"': '"List certificates"',
        '"List cluster users"': '"List users"',
        '"List roles"': '"List roles"',
        '"List ACLs"': '"List ACLs"',
        '"List pools"': '"List pools"',
        '"List replication jobs"': '"List replication"',
        '"List HA resources"': '"List HA"',
        '"List HA groups"': '"List HA groups"',
        '"List SDN Zones"': '"List zones"',
        '"List SDN Vnets"': '"List vnets"',
        '"List Ceph pools"': '"List pools (Ceph)"',
        '"List Ceph OSDs"': '"List OSDs"',
        '"List Ceph monitors"': '"List monitors"',
        '"List backup schedules"': '"List backup jobs"',
        '"List PCI mappings"': '"List PCI mappings"',
        '"List USB mappings"': '"List USB mappings"',
        '"List metric servers"': '"List metric servers"',
        '"List node PCI devices"': '"List PCI devices"',
        '"List node USB devices"': '"List USB devices"',
    }

    for old, new in replacements.items():
        content = content.replace(old, new)

    with open(path, 'w') as f:
        f.write(content)

if __name__ == "__main__":
    compress_descriptions()
