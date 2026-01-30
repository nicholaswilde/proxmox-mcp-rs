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
async fn test_get_aliases_cluster() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api2/json/cluster/firewall/aliases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                { "name": "web", "cidr": "10.0.0.1/32", "comment": "Web Server" }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());
    let aliases = client.get_aliases("cluster", None).await.unwrap();
    
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases[0]["name"], "web");
}

#[tokio::test]
async fn test_get_aliases_node() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api2/json/nodes/pve1/firewall/aliases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                { "name": "local", "cidr": "127.0.0.1/32", "comment": "Localhost" }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());
    let aliases = client.get_aliases("node", Some("pve1")).await.unwrap();
    
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases[0]["name"], "local");
}

#[tokio::test]
async fn test_create_alias_cluster() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api2/json/cluster/firewall/aliases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());
    client.create_alias("cluster", None, "new", "10.0.0.2/32", Some("New Alias")).await.unwrap();
}

#[tokio::test]
async fn test_update_alias_cluster() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/api2/json/cluster/firewall/aliases/web"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());
    client.update_alias("cluster", None, "web", "10.0.0.3/32", Some("Updated")).await.unwrap();
}


#[tokio::test]
async fn test_delete_alias_node() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api2/json/nodes/pve1/firewall/aliases/old"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());
    client.delete_alias("node", Some("pve1"), "old").await.unwrap();
}
