use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use wiremock::MockServer;

mod common;
use common::*;

// =============================================================================
// TESTS
// =============================================================================

#[tokio::test]
async fn test_infrastructure_helpers() -> anyhow::Result<()> {
    // 1. Start a local Mock Server
    let mock_server = MockServer::start().await;

    // 2. Setup Mocks using Helpers
    mock_auth_success(&mock_server).await;
    mock_node_list_success(&mock_server).await;

    // 3. Execution: Dummy Client Interaction
    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;

    let base_url = mock_server.uri();

    // Step A: Login
    let login_url = format!("{}/api2/json/access/ticket", base_url);
    let resp = client
        .post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "Login should succeed");

    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    assert_eq!(ticket, TEST_TICKET);
    assert_eq!(csrf_token, TEST_CSRF_TOKEN);

    // Step B: Query Nodes
    let nodes_url = format!("{}/api2/json/nodes", base_url);
    let resp = client
        .get(&nodes_url)
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;

    assert_eq!(
        resp.status(),
        200,
        "Node listing should succeed with valid auth"
    );

    let node_body: ProxmoxResponse<Vec<NodeInfo>> = resp.json().await?;
    assert_eq!(node_body.data.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_cluster_and_node_management() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    mock_auth_success(&mock_server).await;
    mock_node_list_success(&mock_server).await;
    mock_cluster_status_success(&mock_server).await;

    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let base_url = mock_server.uri();

    // Authenticate first
    let login_url = format!("{}/api2/json/access/ticket", base_url);
    let resp = client
        .post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;
    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    // Test 2: Cluster Status
    let cluster_url = format!("{}/api2/json/cluster/status", base_url);
    let resp = client
        .get(&cluster_url)
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "Cluster status should return 200");
    let status_body: Value = resp.json().await?;
    assert!(!status_body["data"].as_array().unwrap().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_vm_lifecycle() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    mock_auth_success(&mock_server).await;

    let node = "pve-node-01";
    let vmid = 100;
    let upid = "UPID:pve-node-01:00000001:00000001:00000001:qmstart:100:root@pam:";

    // Mocks
    mock_vm_action_success(&mock_server, node, vmid, "start", upid).await;
    mock_task_status_success(&mock_server, node, upid).await;

    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let base_url = mock_server.uri();

    // Authenticate
    let login_url = format!("{}/api2/json/access/ticket", base_url);
    let resp = client
        .post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;
    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    // Start VM
    let start_url = format!(
        "{}/api2/json/nodes/{}/qemu/{}/status/start",
        base_url, node, vmid
    );
    let resp = client
        .post(&start_url)
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "Start VM should succeed");
    let upid_response: ProxmoxResponse<String> = resp.json().await?;
    assert_eq!(upid_response.data, upid);

    // Wait for Task
    let task_url = format!(
        "{}/api2/json/nodes/{}/tasks/{}/status",
        base_url, node, upid
    );
    let resp = client
        .get(&task_url)
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let task_body: Value = resp.json().await?;
    assert_eq!(task_body["data"]["status"], "stopped");
    assert_eq!(task_body["data"]["exitstatus"], "OK");

    Ok(())
}

#[tokio::test]
async fn test_storage_and_iso_management() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    mock_auth_success(&mock_server).await;
    let node = "pve-node-01";
    let storage = "local";

    // Mocks
    mock_storage_list_success(&mock_server, node).await;
    mock_storage_content_success(&mock_server, node, storage, "iso").await;

    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let base_url = mock_server.uri();

    // Authenticate
    let login_url = format!("{}/api2/json/access/ticket", base_url);
    let resp = client
        .post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;
    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    // List Storage
    let storage_url = format!("{}/api2/json/nodes/{}/storage", base_url, node);
    let resp = client
        .get(&storage_url)
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "Storage listing should succeed");
    let storage_body: Value = resp.json().await?;
    assert!(!storage_body["data"].as_array().unwrap().is_empty());

    // List ISOs
    let content_url = format!(
        "{}/api2/json/nodes/{}/storage/{}/content",
        base_url, node, storage
    );
    let resp = client
        .get(&content_url)
        .query(&[("content", "iso")])
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "ISO listing should succeed");
    let iso_body: Value = resp.json().await?;
    assert!(!iso_body["data"].as_array().unwrap().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_networking_and_firewall() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    mock_auth_success(&mock_server).await;
    let node = "pve-node-01";

    // Mocks
    mock_network_list_success(&mock_server, node).await;

    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let base_url = mock_server.uri();

    // Authenticate
    let login_url = format!("{}/api2/json/access/ticket", base_url);
    let resp = client
        .post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;
    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    // List Networks
    let network_url = format!("{}/api2/json/nodes/{}/network", base_url, node);
    let resp = client
        .get(&network_url)
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "Network listing should succeed");
    let network_body: Value = resp.json().await?;
    assert!(!network_body["data"].as_array().unwrap().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_snapshot_and_backup() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    mock_auth_success(&mock_server).await;
    let node = "pve-node-01";
    let vmid = 100;
    let storage = "local";

    // Mocks
    mock_snapshot_list_success(&mock_server, node, vmid).await;
    mock_backup_list_success(&mock_server, node, storage, Some(vmid)).await;

    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let base_url = mock_server.uri();

    // Authenticate
    let login_url = format!("{}/api2/json/access/ticket", base_url);
    let resp = client
        .post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;
    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    // List Snapshots
    let snap_url = format!(
        "{}/api2/json/nodes/{}/qemu/{}/snapshot",
        base_url, node, vmid
    );
    let resp = client
        .get(&snap_url)
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let snap_body: Value = resp.json().await?;
    assert!(!snap_body["data"].as_array().unwrap().is_empty());

    // List Backups (via storage content)
    let backup_url = format!(
        "{}/api2/json/nodes/{}/storage/{}/content",
        base_url, node, storage
    );
    let resp = client
        .get(&backup_url)
        .query(&[("content", "backup")])
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let backup_body: Value = resp.json().await?;
    assert!(!backup_body["data"].as_array().unwrap().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_auth_failure() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    mock_auth_failure(&mock_server).await;

    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let base_url = mock_server.uri();

    let login_url = format!("{}/api2/json/access/ticket", base_url);
    let resp = client
        .post(&login_url)
        .form(&[("username", "root@pam"), ("password", "wrong")])
        .send()
        .await?;

    assert_eq!(resp.status(), 401, "Login should fail with 401");
    Ok(())
}

#[tokio::test]
async fn test_resource_missing() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    mock_auth_success(&mock_server).await;
    let node = "pve-node-01";
    let vmid = 9999;

    mock_vm_not_found(&mock_server, node, vmid).await;

    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let base_url = mock_server.uri();

    let login_url = format!("{}/api2/json/access/ticket", base_url);
    let _ = client
        .post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;

    // Try to get VM status
    let status_url = format!(
        "{}/api2/json/nodes/{}/qemu/{}/status/current",
        base_url, node, vmid
    );
    let resp = client
        .get(&status_url)
        .header("CSRFPreventionToken", TEST_CSRF_TOKEN)
        .header("Cookie", format!("PVEAuthCookie={}", TEST_TICKET))
        .send()
        .await?;

    assert_eq!(resp.status(), 500);
    Ok(())
}

#[tokio::test]
async fn test_api_timeout() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    mock_auth_success(&mock_server).await;

    mock_api_timeout(&mock_server).await;

    let client = Client::builder().timeout(Duration::from_secs(1)).build()?; // Short timeout
    let base_url = mock_server.uri();

    let login_url = format!("{}/api2/json/access/ticket", base_url);
    let _ = client
        .post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;

    // List Nodes (Should timeout)
    let nodes_url = format!("{}/api2/json/nodes", base_url);
    let result = client
        .get(&nodes_url)
        .header("CSRFPreventionToken", TEST_CSRF_TOKEN)
        .header("Cookie", format!("PVEAuthCookie={}", TEST_TICKET))
        .send()
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().is_timeout());

    Ok(())
}