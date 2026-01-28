use proxmox_mcp_rs::mcp::McpServer;
use proxmox_mcp_rs::proxmox::ProxmoxClient;
use serde_json::json;
use std::collections::HashMap;
use url::Url;
use wiremock::MockServer;

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
