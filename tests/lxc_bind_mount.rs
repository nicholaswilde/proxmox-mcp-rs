use proxmox_mcp_rs::proxmox::ProxmoxClient;
use serde_json::json;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

fn create_test_client(uri: &str) -> ProxmoxClient {
    let url = Url::parse(uri).unwrap();
    let host_str = format!("{}://{}", url.scheme(), url.host_str().unwrap());
    ProxmoxClient::new(&host_str, url.port().unwrap(), true).unwrap()
}

#[tokio::test]
async fn test_add_lxc_bind_mount_client() {
    let mock_server = MockServer::start().await;

    // Mock config update
    Mock::given(method("PUT"))
        .and(path("/api2/json/nodes/pve1/lxc/100/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let res = client
        .add_lxc_bind_mount(
            "pve1",
            100,
            "mp0",
            "/host/path",
            "/container/path",
            Some(true),
        )
        .await;

    assert!(res.is_ok());
}

#[tokio::test]
async fn test_tool_definition() {
    let mock_server = MockServer::start().await;
    let server = common::setup_mcp_server(&mock_server).await;

    let req = proxmox_mcp_rs::mcp::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: None,
        id: Some(json!(1)),
    };
    let res = server.handle_request(req).await.unwrap();
    let tools = res["tools"].as_array().unwrap();

    assert!(
        tools.iter().any(|t| t["name"] == "manage_resource_config"),
        "Tool manage_resource_config not found in tools/list"
    );
}

#[tokio::test]
async fn test_call_add_lxc_bind_mount() {
    let mock_server = MockServer::start().await;
    let server = common::setup_mcp_server(&mock_server).await;

    // Mock config update
    Mock::given(method("PUT"))
        .and(path("/api2/json/nodes/pve1/lxc/100/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
        .mount(&mock_server)
        .await;

    let args = json!({
        "node": "pve1",
        "vmid": 100,
        "action": "add_lxc_bind_mount",
        "mp_id": "mp0",
        "source": "/host/path",
        "target": "/container/path",
        "read_only": true
    });
    let result = server.call_tool("manage_resource_config", &args).await;

    assert!(result.is_ok());
    let content = result.unwrap();
    assert!(content["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("mp0 added to CT 100")); // Assuming success message format
}
