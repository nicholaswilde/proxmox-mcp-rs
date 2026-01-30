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
async fn test_bulk_vm_action_success() {
    let mock_server = MockServer::start().await;

    // Mock start action for VM 100
    Mock::given(method("POST"))
        .and(path("/api2/json/nodes/pve1/qemu/100/status/start"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:pve1:..." })))
        .mount(&mock_server)
        .await;

    // Mock start action for VM 101
    Mock::given(method("POST"))
        .and(path("/api2/json/nodes/pve1/qemu/101/status/start"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:pve1:..." })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());
    let vmids = vec![100, 101];
    
    // This method does not exist yet, so this test will fail to compile.
    // However, for the "Red" phase in Rust, compilation failure due to missing method 
    // is technically a failure. But usually we want runtime failure if possible.
    // Since we can't call a non-existent method, I will comment it out and assert(false) 
    // to simulate "test written but logic missing" or just let it fail compilation 
    // if the harness allows. 
    // The instructions say "Write Failing Tests (Red Phase)... Run the tests and confirm that they fail".
    // In Rust, if code doesn't compile, `cargo test` fails. That counts.
    
    let result = client.bulk_vm_action("pve1", vmids, "start", None).await;
    
    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.len(), 2);
    assert!(report.contains_key(&100));
    assert!(report.contains_key(&101));
    assert!(report[&100].is_ok());
    assert!(report[&101].is_ok());
}

#[tokio::test]
async fn test_bulk_vm_action_partial_failure() {
    let mock_server = MockServer::start().await;

    // Mock start action for VM 100 (Success)
    Mock::given(method("POST"))
        .and(path("/api2/json/nodes/pve1/qemu/100/status/stop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:pve1:..." })))
        .mount(&mock_server)
        .await;

    // Mock start action for VM 101 (Failure)
    Mock::given(method("POST"))
        .and(path("/api2/json/nodes/pve1/qemu/101/status/stop"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Error"))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());
    let vmids = vec![100, 101];

    let result = client.bulk_vm_action("pve1", vmids, "stop", None).await;

    assert!(result.is_ok()); // The bulk action itself succeeds, but returns individual statuses
    let report = result.unwrap();
    assert!(report[&100].is_ok());
    assert!(report[&101].is_err());
}
