mod common;
use common::*;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_factory_works() {
    let mock_server = MockServer::start().await;
    let _server = setup_mcp_server(&mock_server).await;
}

#[tokio::test]
async fn test_list_nodes_vm() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    mock_node_list_success(&mock_server).await;

    let args = json!({ "instance": "default" });
    let result = server.call_tool("list_nodes", &args).await.unwrap();

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("pve-node-01"));
    assert!(text.contains("pve-node-02"));
}

#[tokio::test]
async fn test_list_vms() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    mock_list_vms_success(&mock_server).await;

    let args = json!({ "instance": "default" });
    let result = server.call_tool("list_vms", &args).await.unwrap();

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("vm-100"));
    assert!(text.contains("vm-101"));
}

#[tokio::test]
async fn test_list_storage() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    mock_storage_list_success(&mock_server, "pve-node-01").await;

    let args = json!({ "instance": "default", "node": "pve-node-01" });
    let result = server.call_tool("list_storage", &args).await.unwrap();

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("local"));
    assert!(text.contains("local-lvm"));
}

#[tokio::test]
async fn test_vm_power_action() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let node = "pve-node-01";
    let vmid = 100;
    let upid = "UPID:pve-node-01:00000001:00000001:00000001:qmstart:100:root@pam:";

    mock_vm_action_success(&mock_server, node, vmid, "start", upid).await;

    let args = json!({
        "instance": "default",
        "node": node,
        "vmid": vmid,
        "action": "start"
    });
    let result = server.call_tool("vm_power_action", &args).await.unwrap();

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Action 'start' initiated."));
    assert!(text.contains(upid));
}

#[tokio::test]
async fn test_manage_resource_config() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let node = "pve-node-01";
    let vmid = 100;

    mock_vm_config_update_success(&mock_server, node, vmid, "qemu").await;

    let args = json!({
        "instance": "default",
        "node": node,
        "vmid": vmid,
        "action": "add_disk",
        "device": "scsi0",
        "storage": "local-lvm",
        "size_gb": 32
    });
    let result = server
        .call_tool("manage_resource_config", &args)
        .await
        .unwrap();

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Disk added"));
}

#[tokio::test]
async fn test_manage_cluster_config_alias() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    // Mock create alias
    Mock::given(method("POST"))
        .and(path("/api2/json/cluster/firewall/aliases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
        .mount(&mock_server)
        .await;

    let args = json!({
        "instance": "default",
        "type": "firewall_alias",
        "action": "create",
        "level": "cluster",
        "name": "local_net",
        "cidr": "192.168.0.0/24"
    });
    let result = server
        .call_tool("manage_cluster_config", &args)
        .await
        .unwrap();

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Alias local_net create"));
}

#[tokio::test]
async fn test_manage_node_system_apt() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let node = "pve-node-01";

    // Mock run apt update
    Mock::given(method("POST"))
        .and(path(format!("/api2/json/nodes/{}/apt/update", node)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
        .mount(&mock_server)
        .await;

    let args = json!({
        "instance": "default",
        "node": node,
        "action": "apt_update"
    });
    let result = server.call_tool("manage_node_system", &args).await.unwrap();
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("APT update initiated"));
}

#[tokio::test]
async fn test_manage_resource_lifecycle() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let node = "pve-node-01";
    let vmid = 100;

    // Mock create VM
    Mock::given(method("POST"))
        .and(path(format!("/api2/json/nodes/{}/qemu", node)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
        .mount(&mock_server)
        .await;

    let args = json!({
        "instance": "default",
        "node": node,
        "vmid": vmid,
        "action": "create",
        "type": "qemu",
        "name": "test-vm"
    });
    let result = server.call_tool("manage_resource", &args).await.unwrap();
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Create qemu initiated"));
}
