import sys
import re

def fix_tests():
    path = 'src/tests.rs'
    with open(path, 'r') as f:
        content = f.read()

    # Split by #[tokio::test] to get individual tests
    parts = content.split('#[tokio::test]')
    new_parts = [parts[0]]
    
    for part in parts[1:]:
        # Identify test name
        test_match = re.search(r'async fn (\w+)', part)
        if not test_match:
            new_parts.append(part)
            continue
        
        test_name = test_match.group(1)
        
        # Injections based on test name
        if 'test_start_vm' in test_name: 
            part = part.replace('call_tool("vm_power_action", &args)', 'call_tool("vm_power_action", &json!({"action": "start", "type": "qemu", "node": "pve1", "vmid": 100}))')
        elif 'test_stop_vm' in test_name:
            part = part.replace('call_tool("vm_power_action", &args)', 'call_tool("vm_power_action", &json!({"action": "stop", "type": "qemu", "node": "pve1", "vmid": 100}))')
        elif 'test_shutdown_vm' in test_name:
            part = part.replace('call_tool("vm_power_action", &args)', 'call_tool("vm_power_action", &json!({"action": "shutdown", "type": "qemu", "node": "pve1", "vmid": 100}))')
        elif 'test_reboot_vm' in test_name:
            part = part.replace('call_tool("vm_power_action", &args)', 'call_tool("vm_power_action", &json!({"action": "reboot", "type": "qemu", "node": "pve1", "vmid": 100}))')
        elif 'test_reset_vm' in test_name:
            part = part.replace('call_tool("vm_power_action", &args)', 'call_tool("vm_power_action", &json!({"action": "reset", "type": "qemu", "node": "pve1", "vmid": 100, "vm_id": "100"}))')
        elif 'test_reset_container' in test_name:
            part = part.replace('call_tool("vm_power_action", &args)', 'call_tool("vm_power_action", &json!({"action": "reboot", "type": "lxc", "node": "pve1", "vmid": 100, "container_id": "100"}))')
        
        elif 'test_create_vm' in test_name:
            part = part.replace('call_tool("manage_resource", &args)', 'call_tool("manage_resource", &json!({"action": "create", "type": "qemu", "node": "pve1", "vmid": 100, "name": "test"}))')
        elif 'test_delete_vm' in test_name:
            part = part.replace('call_tool("manage_resource", &args)', 'call_tool("manage_resource", &json!({"action": "delete", "type": "qemu", "node": "pve1", "vmid": 100}))')
        elif 'test_create_container' in test_name:
            part = part.replace('call_tool("manage_resource", &args)', 'call_tool("manage_resource", &json!({"action": "create", "type": "lxc", "node": "pve1", "vmid": 100, "ostemplate": "local:vztmpl/ubuntu.tar.gz"}))')
        elif 'test_delete_container' in test_name:
            part = part.replace('call_tool("manage_resource", &args)', 'call_tool("manage_resource", &json!({"action": "delete", "type": "lxc", "node": "pve1", "vmid": 100}))')
        elif 'test_clone_vm' in test_name:
            part = part.replace('call_tool("manage_resource", &args)', 'call_tool("manage_resource", &json!({"action": "clone", "type": "qemu", "node": "pve1", "vmid": 100, "newid": 101}))')
        elif 'test_migrate_vm' in test_name:
            part = part.replace('call_tool("manage_resource", &args)', 'call_tool("manage_resource", &json!({"action": "migrate", "type": "qemu", "node": "pve1", "vmid": 100, "target_node": "pve2"}))')

        elif 'test_update_vm_resources' in test_name:
            part = part.replace('call_tool("manage_resource_config", &args)', 'call_tool("manage_resource_config", &json!({"action": "update_resources", "type": "qemu", "node": "pve1", "vmid": 100, "cores": 2}))')
        elif 'test_update_container_resources' in test_name:
            part = part.replace('call_tool("manage_resource_config", &args)', 'call_tool("manage_resource_config", &json!({"action": "update_resources", "type": "lxc", "node": "pve1", "vmid": 100, "memory": 1024}))')
        elif 'test_hardware_config' in test_name:
            part = part.replace('call_tool("manage_resource_config", &args)', 'call_tool("manage_resource_config", &json!({"action": "add_disk", "node": "pve1", "vmid": 100, "device": "scsi0", "storage": "local-lvm", "size_gb": 32}))')
        elif 'test_lxc_mountpoints' in test_name:
            part = part.replace('call_tool("manage_resource_config", &args)', 'call_tool("manage_resource_config", &json!({"action": "add_lxc_mountpoint", "node": "pve1", "vmid": 100, "mp_id": "mp0", "volume": "local-lvm:100/vm-100-disk-0.raw", "path": "/mnt/data"}))')
        elif 'test_qemu_agent_tools' in test_name:
            part = part.replace('call_tool("manage_resource_config", &args)', 'call_tool("manage_resource_config", &json!({"action": "exec", "node": "pve1", "vmid": 100, "command": "ls"}))')
        elif 'test_cloudinit_and_tags' in test_name:
            part = part.replace('call_tool("manage_resource_config", &args)', 'call_tool("manage_resource_config", &json!({"action": "set_cloudinit", "node": "pve1", "vmid": 100, "ciuser": "admin"}))')

        elif 'test_snapshot_lifecycle' in test_name:
            part = part.replace('call_tool("manage_snapshot_backup", &args)', 'call_tool("manage_snapshot_backup", &json!({"action": "snapshot_create", "node": "pve1", "vmid": 100, "snapname": "test"}))')
        elif 'test_backup_tools' in test_name:
            part = part.replace('call_tool("manage_snapshot_backup", &args)', 'call_tool("manage_snapshot_backup", &json!({"action": "backup_create", "node": "pve1", "vmid": 100}))')
        elif 'test_backup_schedule_tools' in test_name:
            part = part.replace('call_tool("manage_snapshot_backup", &args)', 'call_tool("manage_snapshot_backup", &json!({"action": "create", "storage": "local", "schedule": "daily"}))')

        elif 'test_apt_and_services' in test_name:
            part = part.replace('call_tool("manage_node_system", &args)', 'call_tool("manage_node_system", &json!({"action": "manage_service", "node": "pve1", "service": "pvestatd", "service_action": "restart"}))')
            # The APT run part needs action too
            part = part.replace('let res = server.call_tool("manage_node_system", &args).await.unwrap();', 'let res = server.call_tool("manage_node_system", &json!({"action": "apt_update", "node": "pve1"})).await.unwrap();')
        elif 'test_subscription_tools' in test_name:
            part = part.replace('call_tool("manage_node_system", &args)', 'call_tool("manage_node_system", &json!({"action": "set_subscription_key", "node": "pve1", "key": "123"}))')

        elif 'test_storage_tools' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "add", "type": "storage", "storage": "local-nfs", "type": "nfs", "server": "1.2.3.4", "export": "/srv/nfs"}))')
        elif 'test_firewall_alias_tools' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "create", "type": "firewall_alias", "level": "cluster", "name": "test", "cidr": "1.1.1.1"}))')
        elif 'test_firewall_security_group_tools' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "add", "type": "security_group", "name": "test", "type": "in", "action": "ACCEPT"}))')
            part = part.replace('call_tool("manage_cluster_config", &json!({ "name": "new_group" }))', 'call_tool("manage_cluster_config", &json!({"action": "create", "type": "security_group", "name": "new_group"}))')
        elif 'test_pool_management' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "create", "type": "pool", "poolid": "test"}))')
        elif 'test_replication_tools' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "create", "type": "replication", "id": "100-0", "target": "pve2"}))')
        elif 'test_ha_management' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "add", "type": "ha", "sid": "vm:100"}))')
        elif 'test_sdn_tools' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "create", "type": "sdn", "resource_type": "zone", "zone": "test", "zone_type": "simple"}))')
        elif 'test_ceph_tools' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "create", "type": "ceph", "node": "pve1", "name": "test"}))')
        elif 'test_metric_server_tools' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "create", "type": "metric", "id": "test", "server_type": "influxdb", "server": "1.1.1.1", "port": 8086}))')
        elif 'test_pci_usb_mapping_tools' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "create", "type": "mapping", "resource_type": "pci", "id": "test", "map": "0000:00:00.0"}))')
        elif 'test_user_management' in test_name:
            part = part.replace('call_tool("manage_cluster_config", &args)', 'call_tool("manage_cluster_config", &json!({"action": "create", "type": "user", "userid": "test@pve", "password": "123"}))')

        new_parts.append(part)

    with open(path, 'w') as f:
        f.write("#[tokio::test]".join(new_parts))

if __name__ == "__main__":
    fix_tests()
