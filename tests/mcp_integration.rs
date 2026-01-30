use proxmox_mcp_rs::mcp::{JsonRpcRequest, McpServer};
use proxmox_mcp_rs::proxmox::ProxmoxClient;
use serde_json::json;
use std::collections::HashMap;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;
use common::*;

/// Helper to create an McpServer instance pointing to a MockServer.
pub async fn setup_mcp_server(mock_server: &MockServer) -> McpServer {
    let uri = mock_server.uri();
    let url = Url::parse(&uri).unwrap();
    let host = url.host_str().unwrap();
    let port = url.port().unwrap();

    let mut client = ProxmoxClient::new(&format!("http://{}", host), port, true).unwrap();
    // We mock a successful login to get the ticket/csrf for the client internal state
    mock_auth_success(mock_server).await;
    client.login("root@pam", "secret").await.unwrap();

    let mut clients = HashMap::new();
    clients.insert("default".to_string(), client);

    McpServer::new(clients, "default".to_string(), false)
}

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
async fn test_cluster_status() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    mock_cluster_status_success(&mock_server).await;

    let args = json!({ "instance": "default" });
    let result = server.call_tool("get_cluster_status", &args).await.unwrap();

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("production-cluster"));
    assert!(text.contains("pve-node-01"));
}

#[tokio::test]
async fn test_vm_lifecycle() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let node = "pve-node-01";
    let vmid = 100;
    let upid = "UPID:pve-node-01:00000001:00000001:00000001:qmstart:100:root@pam:";

    mock_vm_action_success(&mock_server, node, vmid, "start", upid).await;
    mock_task_status_success(&mock_server, node, upid).await;

    let args = json!({
        "instance": "default",
        "node": node,
        "vmid": vmid
    });
    let result = server.call_tool("start_vm", &args).await.unwrap();

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Action 'start' initiated."));
    assert!(text.contains(upid));
}

#[tokio::test]
async fn test_hardware_config() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let node = "pve-node-01";
    let vmid = 100;

    mock_vm_config_update_success(&mock_server, node, vmid, "qemu").await;

    let args = json!({
        "instance": "default",
        "node": node,
        "vmid": vmid,
        "device": "scsi0",
        "storage": "local-lvm",
        "size_gb": 32
    });
    let result = server.call_tool("add_disk", &args).await.unwrap();

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Disk scsi0 added to qemu 100"));
}

#[tokio::test]
async fn test_error_mapping() {
    // 1. Test 401 Mapping
    {
        let mock_server = MockServer::start().await;
        let server = setup_mcp_server(&mock_server).await;
        mock_unauthorized_request(&mock_server, "GET", "/api2/json/nodes").await;

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "list_nodes",
                "arguments": { "instance": "default" }
            })),
            id: Some(json!(1)),
        };

        let result = server.handle_request(req).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("401 Unauthorized"));
    }

    // 2. Test 404 Mapping
    {
        let mock_server = MockServer::start().await;
        let server = setup_mcp_server(&mock_server).await;
        mock_resource_not_found(&mock_server).await;

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "list_nodes",
                "arguments": { "instance": "default" }
            })),
            id: Some(json!(1)),
        };

        let result = server.handle_request(req).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("404 Not Found"));
    }
}

#[tokio::test]
async fn test_firewall_aliases() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    // Mock list aliases
    Mock::given(method("GET"))
        .and(path("/api2/json/cluster/firewall/aliases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                { "name": "local_net", "cidr": "192.168.0.0/24", "comment": "Local Network" }
            ]
        })))
        .mount(&mock_server)
        .await;

    let args = json!({
        "instance": "default",
        "level": "cluster"
    });
    let result = server
        .call_tool("list_firewall_aliases", &args)
        .await
        .unwrap();

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("local_net"));
}

#[tokio::test]
async fn test_bulk_vm_action() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let node = "pve-node-01";
    let vmid1 = 100;
    let vmid2 = 101;
    let upid1 = "UPID:pve-node-01:00000001:qmstart:100:root@pam:";
    let upid2 = "UPID:pve-node-01:00000002:qmstart:101:root@pam:";

    mock_vm_action_success(&mock_server, node, vmid1, "start", upid1).await;
    mock_vm_action_success(&mock_server, node, vmid2, "start", upid2).await;

    let args = json!({
        "instance": "default",
        "node": node,
        "vmids": [vmid1, vmid2],
        "action": "start"
    });

    let result = server.call_tool("bulk_vm_action", &args).await.unwrap();
    let text = result["content"][0]["text"].as_str().unwrap();

    assert!(text.contains("VM 100: Success"));
    assert!(text.contains("VM 101: Success"));
}

#[tokio::test]
async fn test_scan_storage_remote() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let node = "pve-node-01";

    // Mock scan NFS
    Mock::given(method("GET"))
        .and(path(format!("/api2/json/nodes/{}/scan/nfs", node)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                { "path": "/srv/nfs/share1", "options": "rw" }
            ]
        })))
        .mount(&mock_server)
        .await;

    let args = json!({
        "instance": "default",
        "node": node,
        "type": "nfs",
        "server": "1.2.3.4"
    });

    let result = server
        .call_tool("scan_storage_remote", &args)
        .await
        .unwrap();
    let text = result["content"][0]["text"].as_str().unwrap();

    assert!(text.contains("/srv/nfs/share1"));
}

#[tokio::test]
async fn test_apt_repositories() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let node = "pve-node-01";

    // Mock list repositories
    Mock::given(method("GET"))
        .and(path(format!("/api2/json/nodes/{}/apt/repositories", node)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "files": [
                    { "path": "/etc/apt/sources.list", "repositories": [] }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock add repository
    Mock::given(method("POST"))
        .and(path(format!("/api2/json/nodes/{}/apt/repositories", node)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
        .mount(&mock_server)
        .await;

    // 1. Test list_repositories
    let args = json!({
        "instance": "default",
        "node": node
    });
    let result = server.call_tool("list_repositories", &args).await.unwrap();
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("/etc/apt/sources.list"));

    // 2. Test add_repository
    let args = json!({
        "instance": "default",
        "node": node,
        "handle": "pve-no-subscription"
    });
    let result = server.call_tool("add_repository", &args).await.unwrap();
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Repository pve-no-subscription added"));

    // 3. Test update_repository_state
    let args = json!({
        "instance": "default",
        "node": node,
        "path": "/etc/apt/sources.list",
        "index": 0,
        "enabled": true
    });
    let result = server
        .call_tool("update_repository_state", &args)
        .await
        .unwrap();
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Repository state updated"));
}
