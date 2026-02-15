import sys

def generate():
    out = [
        "mod common;",
        "use common::*;",
        "use serde_json::json;",
        "use wiremock::matchers::{method, path, query_param};",
        "use wiremock::{Mock, MockServer, ResponseTemplate};",
        "use proxmox_mcp_rs::proxmox::ProxmoxClient;",
        "use std::time::Duration;",
        "",
        "#[tokio::test]",
        "async fn test_coverage_final_v9() {",
        "    let mock_server = MockServer::start().await;",
        "    let server = setup_mcp_server(&mock_server).await;",
        "",
    ]

    # --- Access ---
    out.append('    // --- Access ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/access/users")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_users", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/access/users")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "user", "action": "create", "userid": "test@pve", "password": "secret", "email": "test@example.com", "firstname": "Test", "lastname": "User", "expire": 0, "enable": true, "comment": "test", "groups": ["admin"]})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/access/users/test@pve")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "user", "action": "delete", "userid": "test@pve"})).await.unwrap();')
    out.append('')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/access/roles")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_roles", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/access/roles")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "role", "action": "create", "roleid": "test-role", "privs": "VM.Config.Options"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/access/roles/test-role")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "role", "action": "update", "roleid": "test-role", "privs": "VM.Config.Options", "append": true})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/access/roles/test-role")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "role", "action": "delete", "roleid": "test-role"})).await.unwrap();')
    out.append('')

    out.append('    Mock::given(method("GET")).and(path("/api2/json/access/acl")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_acls", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/access/acl")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "acl", "action": "update", "path": "/vms/100", "roles": "Admin"})).await.unwrap();')
    out.append('')

    # --- Ceph ---
    out.append('    // --- Ceph ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/ceph/status")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {}}))).mount(&mock_server).await;')
    out.append('    server.call_tool("get_ceph_status", &json!({"node": "pve1"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/ceph/pools")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_ceph_pools", &json!({"node": "pve1"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/ceph/pools")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "ceph", "action": "create", "node": "pve1", "name": "test-pool"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/nodes/pve1/ceph/pools/test-pool")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "ceph", "action": "delete", "node": "pve1", "name": "test-pool", "remove_storages": true})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/ceph/osd")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_ceph_osds", &json!({"node": "pve1"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/ceph/mon")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_ceph_monitors", &json!({"node": "pve1"})).await.unwrap();')
    out.append('')

    # --- Mapping ---
    out.append('    // --- Mapping ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/mapping/pci")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_pci_mappings", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/mapping/pci")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "mapping", "resource_type": "pci", "action": "create", "id": "gpu", "map": "node=pve1,path=0000:01:00.0"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/cluster/mapping/pci/gpu")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "mapping", "resource_type": "pci", "action": "update", "id": "gpu", "map": "node=pve1,path=0000:01:00.0"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/cluster/mapping/pci/gpu")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "mapping", "resource_type": "pci", "action": "delete", "id": "gpu"})).await.unwrap();')
    out.append('')

    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/mapping/usb")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_usb_mappings", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/mapping/usb")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "mapping", "resource_type": "usb", "action": "create", "id": "mouse", "map": "node=pve1,path=1-1"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/cluster/mapping/usb/mouse")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "mapping", "resource_type": "usb", "action": "update", "id": "mouse", "map": "node=pve1,path=1-1"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/cluster/mapping/usb/mouse")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "mapping", "resource_type": "usb", "action": "delete", "id": "mouse"})).await.unwrap();')
    out.append('')

    # --- Metric Server ---
    out.append('    // --- Metric Server ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/metrics/server")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_metric_servers", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/metrics/server")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "metric", "server_type": "influx", "action": "create", "id": "influx", "server": "1.1.1.1"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/cluster/metrics/server/influx")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "metric", "action": "update", "id": "influx", "server": "1.1.1.2"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/cluster/metrics/server/influx")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "metric", "action": "delete", "id": "influx"})).await.unwrap();')
    out.append('')

    # --- Replication ---
    out.append('    // --- Replication ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/replication")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_replication_jobs", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/replication")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "replication", "action": "create", "id": "100-0", "target": "pve2", "schedule": "*/15", "rate": 10.0, "comment": "test", "enable": true})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/cluster/replication/100-0")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "replication", "action": "update", "id": "100-0", "comment": "updated"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/cluster/replication/100-0")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "replication", "action": "delete", "id": "100-0"})).await.unwrap();')
    out.append('')

    # --- SDN ---
    out.append('    // --- SDN ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/sdn/zones")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_sdn_zones", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/sdn/zones")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "sdn", "zone_type": "simple", "action": "create", "zone": "test-zone"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/cluster/sdn/zones/test-zone")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "sdn", "action": "delete", "zone": "test-zone"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/sdn/vnets")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_sdn_vnets", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/sdn/vnets")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "vnet", "action": "create", "vnet": "vnet1", "zone": "test-zone"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/cluster/sdn/vnets/vnet1")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "vnet", "action": "delete", "vnet": "vnet1"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/cluster/sdn")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("apply_sdn_changes", &json!({})).await.unwrap();')
    out.append('')

    # --- Snapshot ---
    out.append('    // --- Snapshot ---')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu/100/snapshot")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_snapshot_backup", &json!({"node": "pve1", "action": "snapshot_create", "vmid": 100, "snapname": "snap1", "description": "test", "vmstate": true})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu/100/snapshot/snap1/rollback")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_snapshot_backup", &json!({"node": "pve1", "action": "snapshot_rollback", "vmid": 100, "snapname": "snap1"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/nodes/pve1/qemu/100/snapshot/snap1")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_snapshot_backup", &json!({"node": "pve1", "action": "snapshot_delete", "vmid": 100, "snapname": "snap1"})).await.unwrap();')
    out.append('')

    # --- Backup Schedule ---
    out.append('    // --- Backup Schedule ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/backup")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_backup_schedules", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/backup")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_snapshot_backup", &json!({"node": "pve1", "action": "create", "vmid": 100, "storage": "local", "schedule": "daily"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/cluster/backup/1")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_snapshot_backup", &json!({"node": "pve1", "action": "update", "id": "1", "schedule": "weekly"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/cluster/backup/1")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_snapshot_backup", &json!({"node": "pve1", "action": "delete", "id": "1"})).await.unwrap();')
    out.append('')

    # --- Storage ---
    out.append('    // --- Storage ---')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/storage")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "storage", "action": "add", "storage": "nfs-test", "storage_type": "nfs", "server": "1.1.1.1", "export": "/share"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/storage/local")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "storage", "action": "update", "storage": "local", "content": "iso"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/storage/nfs-test")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "storage", "action": "delete", "storage": "nfs-test"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/nodes/pve1/storage/local/content/iso/debian.iso")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "storage_content", "action": "delete", "node": "pve1", "storage": "local", "volume": "iso/debian.iso"})).await.unwrap();')
    out.append('')

    # --- System ---
    out.append('    // --- System ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/subscription")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {}}))).mount(&mock_server).await;')
    out.append('    server.call_tool("get_subscription_info", &json!({"node": "pve1"})).await.unwrap();')
    out.append('')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/subscription")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_node_system", &json!({"node": "pve1", "action": "set_subscription_key", "key": "NEW-KEY"})).await.unwrap();')
    out.append('')

    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/certificates/acme/certificate")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_node_system", &json!({"node": "pve1", "action": "cert_renew"})).await.unwrap();')
    out.append('')

    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/certificates/custom")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_node_system", &json!({"node": "pve1", "action": "upload_certificate", "certificates": "CERT", "key": "KEY", "force": true, "restart": true})).await.unwrap();')
    out.append('')

    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/apt/repositories")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_node_system", &json!({"node": "pve1", "action": "add_repository", "handle": "pve-no-subscription"})).await.unwrap();')
    out.append('')

    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/apt/repositories")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_node_system", &json!({"node": "pve1", "action": "update_repository_state", "path": "/etc/apt/sources.list", "index": 0, "enabled": true})).await.unwrap();')
    out.append('')

    # --- Tags ---
    out.append('    // --- Tags ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/resources")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [{"vmid": 100, "node": "pve1", "type": "qemu"}]} ))).mount(&mock_server).await;')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/qemu/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"tags": "tag1"}}))).mount(&mock_server).await;')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/qemu/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_tags", &json!({"node": "pve1", "vmid": 100, "action": "add", "tags": "tag2"})).await.unwrap();')
    out.append('    server.call_tool("manage_tags", &json!({"node": "pve1", "vmid": 100, "action": "remove", "tags": "tag1"})).await.unwrap();')
    out.append('    server.call_tool("manage_tags", &json!({"node": "pve1", "vmid": 100, "action": "set", "tags": "tag3"})).await.unwrap();')
    out.append('')

    # --- HA ---
    out.append('    // --- HA ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/ha/resources")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_ha_resources", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/ha/groups")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_ha_groups", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/ha/resources")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "ha", "action": "add", "sid": "vm:100", "group": "ha-group"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/cluster/ha/resources/vm:100")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "ha", "action": "update", "sid": "vm:100", "state": "started"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/cluster/ha/resources/vm:100")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "ha", "action": "delete", "sid": "vm:100"})).await.unwrap();')
    out.append('')

    # --- Pools ---
    out.append('    // --- Pools ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/pools")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_pools", &json!({})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/pools")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "pool", "action": "create", "poolid": "test-pool", "comment": "test"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/pools/test-pool")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "pool", "action": "update", "poolid": "test-pool", "comment": "updated"})).await.unwrap();')
    out.append('')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/pools/test-pool")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "pool", "action": "delete", "poolid": "test-pool"})).await.unwrap();')
    out.append('')

    # --- VM Power Action ---
    out.append('    // --- VM Power Action ---')
    for action in ["stop", "shutdown", "reboot", "reset", "suspend", "resume"]:
        out.append(f'    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu/100/status/{action}")).respond_with(ResponseTemplate::new(200).set_body_json(json!({{"data": "UPID:..."}}))).mount(&mock_server).await;')
        out.append(f'    server.call_tool("vm_power_action", &json!({{"node": "pve1", "vmid": 100, "action": "{action}", "type": "qemu"}})).await.unwrap();')
    
    # --- Manage Resource ---
    out.append('    // --- Manage Resource ---')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/nodes/pve1/qemu/100")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource", &json!({"node": "pve1", "vmid": 100, "action": "delete", "type": "qemu"})).await.unwrap();')
    
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu/100/clone")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource", &json!({"node": "pve1", "vmid": 100, "action": "clone", "type": "qemu", "newid": 101, "name": "cloned", "target": "pve2", "full": true})).await.unwrap();')
    
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu/100/migrate")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource", &json!({"node": "pve1", "vmid": 100, "action": "migrate", "type": "qemu", "target_node": "pve2", "online": true})).await.unwrap();')
    
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu/100/template")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource", &json!({"node": "pve1", "vmid": 100, "action": "template", "type": "qemu"})).await.unwrap();')

    # --- Manage Resource Config ---
    out.append('    // --- Manage Resource Config ---')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/qemu/100/resize")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/qemu/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "update_resources", "type": "qemu", "cores": 4, "memory": 4096, "sockets": 2, "disk_gb": 10, "disk": "scsi0"})).await.unwrap();')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/qemu/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "remove_disk", "type": "qemu", "device": "scsi0"})).await.unwrap();')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/qemu/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "add_network", "type": "qemu", "device": "net0", "bridge": "vmbr0", "model": "virtio", "mac": "AA:BB:CC:DD:EE:FF"})).await.unwrap();')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/qemu/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "remove_network", "type": "qemu", "device": "net0"})).await.unwrap();')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/qemu/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "set_cloudinit", "type": "qemu", "sshkeys": "ssh-rsa ..." , "ipconfig0": "ip=dhcp"})).await.unwrap();')
    
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu/100/agent/exec")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"pid": 1234}}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "exec", "type": "qemu", "command": "ls /"})).await.unwrap();')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/lxc/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "add_lxc_mountpoint", "type": "lxc", "mp_id": "mp0", "volume": "local-lvm:10", "path": "/data", "read_only": true, "backup": true})).await.unwrap();')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/lxc/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "add_lxc_bind_mount", "type": "lxc", "mp_id": "mp1", "source": "/host/path", "target": "/ct/path", "read_only": false})).await.unwrap();')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/qemu/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "add_pci_device", "type": "qemu", "device_id": "hostpci0", "host": "0000:01:00.0", "pcie": true, "mdev": "mdev-type"})).await.unwrap();')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/qemu/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "add_usb_device", "type": "qemu", "device_id": "usb0", "host": "1-1", "usb3": true})).await.unwrap();')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/qemu/100/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_resource_config", &json!({"node": "pve1", "vmid": 100, "action": "remove_vm_device", "type": "qemu", "device_id": "hostpci0"})).await.unwrap();')

    # --- Manage Snapshot/Backup ---
    out.append('    // --- Manage Snapshot/Backup ---')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/vzdump")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_snapshot_backup", &json!({"node": "pve1", "action": "backup_create", "vmid": 100, "storage": "local", "mode": "snapshot", "compress": "zstd", "remove": true})).await.unwrap();')
    
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_snapshot_backup", &json!({"node": "pve1", "action": "backup_restore", "vmid": 100, "type": "qemu", "archive": "local:backup/vzdump-...", "storage": "local-lvm", "force": true})).await.unwrap();')

    # --- List Tools ---
    out.append('    // --- List Tools ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/storage")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_cluster_storage", &json!({})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/firewall/rules")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_firewall_rules", &json!({"node": "pve1"})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/firewall/groups")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_security_groups", &json!({})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/firewall/groups/web")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_security_group_rules", &json!({"name": "web"})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/tasks")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_tasks", &json!({"node": "pve1", "limit": 10})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/storage/local/content")).and(query_param("content", "backup")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_backups", &json!({"node": "pve1", "storage": "local", "vmid": 100})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/qemu/100/snapshot")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_snapshots", &json!({"node": "pve1", "vmid": 100, "type": "qemu"})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/storage/local/content")).and(query_param("content", "vztmpl")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_templates", &json!({"node": "pve1", "storage": "local", "content": "vztmpl"})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/storage/local/content")).and(query_param("content", "iso")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_isos", &json!({"node": "pve1", "storage": "local"})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/apt/repositories")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_repositories", &json!({"node": "pve1"})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/apt/update")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_apt_updates", &json!({"node": "pve1"})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/services")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_services", &json!({"node": "pve1"})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/hardware/pci")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_pci_devices", &json!({"node": "pve1"})).await.unwrap();')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/hardware/usb")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_usb_devices", &json!({"node": "pve1"})).await.unwrap();')

    # --- Firewall level=node ---
    out.append('    // --- Firewall level=node ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/firewall/aliases")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    server.call_tool("list_firewall_aliases", &json!({"level": "node", "node": "pve1"})).await.unwrap();')
    
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/firewall/aliases")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "firewall_alias", "action": "create", "level": "node", "node": "pve1", "name": "test", "cidr": "1.1.1.1/32", "comment": "test"} )).await.unwrap();')
    
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/firewall/aliases/test")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "firewall_alias", "action": "update", "level": "node", "node": "pve1", "name": "test", "cidr": "1.1.1.2/32", "comment": "updated"} )).await.unwrap();')
    
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/nodes/pve1/firewall/aliases/test")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "firewall_alias", "action": "delete", "level": "node", "node": "pve1", "name": "test"} )).await.unwrap();')

    # --- Firewall security group ---
    out.append('    // --- Firewall security group ---')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/firewall/groups")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "security_group", "action": "create", "name": "test-group", "comment": "test"} )).await.unwrap();')
    
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/cluster/firewall/groups/test-group")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"type": "security_group", "action": "delete", "name": "test-group"} )).await.unwrap();')
    
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/firewall/groups/test-group")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    server.call_tool("manage_cluster_config", &json!({"resource_type": "security_group", "action": "add", "name": "test-group", "type": "in", "rule_action": "ACCEPT"} )).await.unwrap();')

    # --- Cluster Management ---
    out.append('    // --- Cluster Management ---')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/config")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/config/join")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {}}))).mount(&mock_server).await;')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/cluster/config/join")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    
    out.append('    let uri = mock_server.uri();')
    out.append('    let url = url::Url::parse(&uri).unwrap();')
    out.append('    let host_str = format!("{}://{}", url.scheme(), url.host_str().unwrap());')
    out.append('    let client = ProxmoxClient::new(&host_str, url.port().unwrap(), true).unwrap();')
    
    out.append('    let _: String = client.create_cluster("test-cluster").await.unwrap();')
    out.append('    let _: serde_json::Value = client.get_join_info().await.unwrap();')
    out.append('    let _: String = client.join_cluster("host", "pass", "finger").await.unwrap();')

    # --- Firewall Rules ---
    out.append('    // --- Firewall Rules ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/resources")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [{"vmid": 100, "node": "pve1", "type": "qemu"}]} ))).mount(&mock_server).await;')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/qemu/100/firewall/rules")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    let _: Vec<serde_json::Value> = client.get_firewall_rules(Some("pve1"), Some(100)).await.unwrap();')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu/100/firewall/rules")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    let _: () = client.add_firewall_rule(Some("pve1"), Some(100), &json!({})).await.unwrap();')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/nodes/pve1/qemu/100/firewall/rules/0")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    let _: () = client.delete_firewall_rule(Some("pve1"), Some(100), 0).await.unwrap();')

    # --- Pools & Metric Servers ---
    out.append('    // --- Pools & Metric Servers ---')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/pools/test-pool")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {}}))).mount(&mock_server).await;')
    out.append('    let _: serde_json::Value = client.get_pool_details("test-pool").await.unwrap();')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/cluster/metrics/server/influx")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {}}))).mount(&mock_server).await;')
    out.append('    let _: serde_json::Value = client.get_metric_server("influx").await.unwrap();')

    # --- Subscription ---
    out.append('    // --- Subscription ---')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/subscription")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    let _: () = client.update_subscription("pve1").await.unwrap();')

    # --- ProxmoxClient direct methods ---
    out.append('    // --- ProxmoxClient direct methods ---')
    
    # agent.rs
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu/100/agent/ping")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    let _: () = client.agent_ping("pve1", 100).await.unwrap();')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/qemu/100/agent/exec-status")).and(query_param("pid", "1234")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {}}))).mount(&mock_server).await;')
    out.append('    let _: serde_json::Value = client.agent_exec_status("pve1", 100, 1234).await.unwrap();')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/qemu/100/agent/file-read")).and(query_param("file", "/etc/passwd")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"content": "..."}}))).mount(&mock_server).await;')
    out.append('    let _: serde_json::Value = client.agent_file_read("pve1", 100, "/etc/passwd").await.unwrap();')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/qemu/100/agent/file-write")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    let _: () = client.agent_file_write("pve1", 100, "/tmp/test", "content", Some(true)).await.unwrap();')

    # network_config.rs
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/network")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    let _: Vec<serde_json::Value> = client.get_network_interfaces("pve1").await.unwrap();')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/network")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    let _: () = client.create_network_bridge("pve1", "vmbr1", &json!({})).await.unwrap();')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/network")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    let _: () = client.create_network_bond("pve1", "bond0", &json!({})).await.unwrap();')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/network/vmbr1")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    let _: () = client.update_network_interface("pve1", "vmbr1", &json!({})).await.unwrap();')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/nodes/pve1/network/vmbr1")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    let _: () = client.delete_network_interface("pve1", "vmbr1").await.unwrap();')
    out.append('    Mock::given(method("PUT")).and(path("/api2/json/nodes/pve1/network")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    let _: String = client.apply_network_config("pve1").await.unwrap();')
    out.append('    Mock::given(method("DELETE")).and(path("/api2/json/nodes/pve1/network")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": null}))).mount(&mock_server).await;')
    out.append('    let _: () = client.revert_network_config("pve1").await.unwrap();')

    # storage.rs
    out.append('    Mock::given(method("POST")).and(path("/api2/json/nodes/pve1/storage/local/download-url")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:..."}))).mount(&mock_server).await;')
    out.append('    let _: String = client.download_url("pve1", "local", "http://...", "file", "iso", Some("hash"), Some("sha256")).await.unwrap();')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/storage/local/content/iso/debian.iso")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {}}))).mount(&mock_server).await;')
    out.append('    let _: serde_json::Value = client.get_storage_content_volume("pve1", "local", "iso/debian.iso").await.unwrap();')
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/scan/nfs")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []}))).mount(&mock_server).await;')
    out.append('    let _: Vec<serde_json::Value> = client.scan_storage("pve1", "nfs", "1.1.1.1", Some("user"), Some("pass")).await.unwrap();')

    # Error cases
    out.append('    // --- Error Cases ---')
    out.append('    Mock::given(method("POST")).and(path("/api2/json/access/ticket")).respond_with(ResponseTemplate::new(401)).mount(&mock_server).await;')
    out.append('    let mut client_err = client.clone();')
    out.append('    let _ = client_err.login("u", "p").await;')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/storage")).respond_with(ResponseTemplate::new(500)).mount(&mock_server).await;')
    out.append('    let _ = client.get_storage_list("pve1").await;')
    
    out.append('    Mock::given(method("GET")).and(path("/api2/json/nodes/pve1/tasks/UPID/status")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"status": "running"}}))).mount(&mock_server).await;')
    out.append('    let _ = client.wait_for_task("pve1", "UPID", 1).await;')

    out.append("}")

    with open("tests/coverage_expansion.rs", "w") as f:
        f.write("\n".join(out))

if __name__ == "__main__":
    generate()
