import sys
import re

def inject_args():
    path = 'src/tests.rs'
    with open(path, 'r') as f:
        lines = f.readlines()

    # Define specific injections for known test functions
    # This is safer than global regex
    
    new_lines = []
    for line in lines:
        # Power
        if 'call_tool("vm_power_action"' in line:
            if 'test_start_vm' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "start", "type": "qemu", ')
            elif 'test_stop_vm' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "stop", "type": "qemu", ')
            elif 'test_shutdown_vm' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "shutdown", "type": "qemu", ')
            elif 'test_reboot_vm' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "reboot", "type": "qemu", ')
            elif 'test_reset_vm' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "reset", "type": "qemu", ')
            elif 'test_reset_container' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "reboot", "type": "lxc", ')
        
        # Lifecycle
        if 'call_tool("manage_resource"' in line:
            if 'test_create_vm' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "qemu", ')
            elif 'test_delete_vm' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "delete", "type": "qemu", ')
            elif 'test_create_container' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "lxc", ')
            elif 'test_delete_container' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "delete", "type": "lxc", ')
            elif 'test_clone_vm' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "clone", "type": "qemu", ')
            elif 'test_migrate_vm' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "migrate", "type": "qemu", ')
            elif 'test_template_vm' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "template", ')

        # Config
        if 'call_tool("manage_resource_config"' in line:
            if 'test_update_vm_resources' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "update_resources", "type": "qemu", ')
            elif 'test_update_container_resources' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "update_resources", "type": "lxc", ')
            elif 'test_hardware_config' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "add_disk", ')
            elif 'test_lxc_mountpoints' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "add_lxc_mountpoint", ')
            elif 'test_qemu_agent_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "exec", ')
            elif 'test_cloudinit_and_tags' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "set_cloudinit", ')

        # Cluster
        if 'call_tool("manage_cluster_config"' in line:
            if 'test_storage_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "add", "type": "storage", ')
            elif 'test_cluster_storage_management' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "add", "type": "storage", ')
            elif 'test_firewall_alias_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "firewall_alias", ')
            elif 'test_firewall_security_group_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "security_group", ')
            elif 'test_pool_management' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "pool", ')
            elif 'test_replication_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "replication", ')
            elif 'test_ha_management' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "add", "type": "ha", ')
            elif 'test_sdn_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "sdn", ')
            elif 'test_ceph_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "ceph", ')
            elif 'test_metric_server_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "metric", ')
            elif 'test_pci_usb_mapping_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "mapping", ')
            elif 'test_user_management' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", "type": "user", ')

        # Snapshot/Backup
        if 'call_tool("manage_snapshot_backup"' in line:
            if 'test_snapshot_lifecycle' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "snapshot_create", ')
            elif 'test_backup_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "backup_create", ')
            elif 'test_backup_schedule_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "create", ')

        # Node System
        if 'call_tool("manage_node_system"' in line:
            if 'test_apt_and_services' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "manage_service", ')
            elif 'test_subscription_tools' in "".join(new_lines[-20:]): line = line.replace('json!({', 'json!({"action": "set_subscription_key", ')

        new_lines.append(line)

    with open(path, 'w') as f:
        f.writelines(new_lines)

if __name__ == "__main__":
    inject_args()
