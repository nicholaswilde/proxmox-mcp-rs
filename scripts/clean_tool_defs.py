import sys
import re

def clean_defs():
    path = 'src/mcp.rs'
    with open(path, 'r') as f:
        content = f.read()

    defs = {
        'vm_config': [],
        'firewall_aliases': ['list_firewall_aliases'],
        'firewall_security_groups': ['list_security_groups', 'list_security_group_rules'],
        'system': ['list_tasks', 'list_services'],
        'apt': ['list_repositories', 'list_apt_updates'],
        'certificates': ['list_certificates'],
        'access': ['list_users', 'list_roles', 'list_acls'],
        'ha': ['list_pools', 'list_replication_jobs', 'list_ha_resources', 'list_ha_groups'],
        'sdn': ['list_sdn_zones', 'list_sdn_vnets'],
        'ceph': ['list_ceph_pools', 'list_ceph_osds', 'list_ceph_monitors'],
        'backup_schedule': ['list_backup_schedules'],
        'mapping': ['list_pci_mappings', 'list_usb_mappings'],
        'metric_server': ['list_metric_servers'],
        'misc': ['list_pci_devices', 'list_usb_devices']
    }

    # Helper to find tool definition JSON by name
    def find_tool_json(name, content):
        pattern = r'json!\(\{\s+"name": "' + name + r'".*?\}\)'
        match = re.search(pattern, content, re.DOTALL)
        if match:
            return match.group(0)
        return None

    for group, tools in defs.items():
        fn_name = 'tool_defs_' + group
        tool_jsons = []
        for t in tools:
            t_json = find_tool_json(t, content)
            if t_json:
                tool_jsons.append(t_json)
        
        new_fn_body = 'fn ' + fn_name + '(&self) -> Vec<Value> {\n        vec![\n            ' + ',\n            '.join(tool_jsons) + '\n        ]\n    }'
        
        pattern = r'fn ' + fn_name + r'\(&self\) -> Vec<Value> \{.*?\}\n'
        content = re.sub(pattern, new_fn_body + '\n', content, flags=re.DOTALL)

    with open(path, 'w') as f:
        f.write(content)

if __name__ == "__main__":
    clean_defs()
