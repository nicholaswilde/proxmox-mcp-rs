use proxmox_mcp_rs::proxmox::ProxmoxClient;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use url::Url;

fn create_test_client(uri: &str) -> ProxmoxClient {
    let url = Url::parse(uri).unwrap();
    let host_str = format!("{}://{}", url.scheme(), url.host_str().unwrap());
    ProxmoxClient::new(&host_str, url.port().unwrap(), true).unwrap()
}

#[tokio::test]
async fn test_get_repositories() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api2/json/nodes/pve1/apt/repositories"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "files": [
                    { "path": "/etc/apt/sources.list", "repositories": [] }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());
    let repos = client.get_repositories("pve1").await.unwrap();
    
    assert!(repos.get("files").is_some());
}

#[tokio::test]
async fn test_add_repository() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api2/json/nodes/pve1/apt/repositories"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());
    client.add_repository("pve1", "pve-no-subscription").await.unwrap();
}

#[tokio::test]
async fn test_change_repository_state() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api2/json/nodes/pve1/apt/repositories"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());
    client.change_repository_state("pve1", "/etc/apt/sources.list", 0, true).await.unwrap();
}
