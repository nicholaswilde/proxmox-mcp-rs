use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};
use serde_json::{json, Value};
use reqwest::Client;
use std::time::Duration;

// =============================================================================
// SHARED DATA STRUCTURES
// =============================================================================

/// Represents the standard Proxmox API response wrapper.
/// Proxmox APIs usually wrap successful results in a "data" field.
#[derive(serde::Deserialize, Debug)]
pub struct ProxmoxResponse<T> {
    pub data: T,
}

#[derive(serde::Deserialize, Debug)]
pub struct TicketResponse {
    #[serde(rename = "CSRFPreventionToken")]
    pub csrf_token: String,
    pub ticket: String,
    pub username: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct NodeInfo {
    pub node: String,
    pub status: String,
    #[serde(rename = "maxcpu")]
    pub max_cpu: u64,
    #[serde(rename = "maxmem")]
    pub max_mem: u64,
}

// =============================================================================
// CONSTANTS
// =============================================================================

pub const TEST_CSRF_TOKEN: &str = "5B7A...:8901";
pub const TEST_TICKET: &str = "PVE:root@pam:5B7A...";

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Mocks a successful authentication flow (POST /api2/json/access/ticket).
pub async fn mock_auth_success(server: &MockServer) {
    let auth_response = json!({
        "data": {
            "CSRFPreventionToken": TEST_CSRF_TOKEN,
            "ticket": TEST_TICKET,
            "username": "root@pam",
            "cap": {
                "nodes": { "sysadmin": 1 }
            }
        }
    });

    Mock::given(method("POST"))
        .and(path("/api2/json/access/ticket"))
        .respond_with(ResponseTemplate::new(200).set_body_json(auth_response))
        .mount(server)
        .await;
}

/// Mocks a successful node list retrieval (GET /api2/json/nodes).
pub async fn mock_node_list_success(server: &MockServer) {
    let nodes_response = json!({
        "data": [
            {
                "node": "pve-node-01",
                "status": "online",
                "maxcpu": 16,
                "maxmem": 34359738368u64,
                "level": "",
                "id": "node/pve-node-01",
                "type": "node"
            },
            {
                "node": "pve-node-02",
                "status": "offline",
                "maxcpu": 8,
                "maxmem": 17179869184u64,
                "level": "",
                "id": "node/pve-node-02",
                "type": "node"
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/api2/json/nodes"))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .and(header("Cookie", format!("PVEAuthCookie={}", TEST_TICKET).as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(nodes_response))
        .mount(server)
        .await;
}

/// Mocks a successful cluster status retrieval (GET /api2/json/cluster/status).
pub async fn mock_cluster_status_success(server: &MockServer) {
    let status_response = json!({
        "data": [
            {
                "type": "cluster",
                "name": "production-cluster",
                "id": "cluster/production-cluster",
                "ip": "192.168.1.10",
                "level": "",
                "local": 0,
                "nodeid": 0
            },
            {
                "type": "node",
                "name": "pve-node-01",
                "id": "node/pve-node-01",
                "ip": "192.168.1.11",
                "level": "",
                "local": 1,
                "nodeid": 1,
                "online": 1
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/api2/json/cluster/status"))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .and(header("Cookie", format!("PVEAuthCookie={}", TEST_TICKET).as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(status_response))
        .mount(server)
        .await;
}

/// Mocks a successful VM action (start, stop, etc.) returning a UPID.
pub async fn mock_vm_action_success(server: &MockServer, node: &str, vmid: u64, action: &str, upid: &str) {
    let response = json!({
        "data": upid
    });
    // Path: /api2/json/nodes/{node}/qemu/{vmid}/status/{action}
    let path_str = format!("/api2/json/nodes/{}/qemu/{}/status/{}", node, vmid, action);
    
    Mock::given(method("POST"))
        .and(path(path_str))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

/// Mocks task status polling.
pub async fn mock_task_status_success(server: &MockServer, node: &str, upid: &str) {
    let response = json!({
        "data": {
            "status": "stopped",
            "exitstatus": "OK",
            "id": upid,
            "node": node,
            "type": "qmstart",
            "user": "root@pam",
            "starttime": 1234567890
        }
    });

    let path_str = format!("/api2/json/nodes/{}/tasks/{}/status", node, upid);

    Mock::given(method("GET"))
        .and(path(path_str))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

/// Mocks storage listing.
pub async fn mock_storage_list_success(server: &MockServer, node: &str) {
    let response = json!({
        "data": [
            {
                "storage": "local",
                "type": "dir",
                "active": 1,
                "content": "iso,vztmpl,backup",
                "total": 107374182400u64,
                "used": 10737418240u64,
                "avail": 96636764160u64,
                "shared": 0
            },
            {
                "storage": "local-lvm",
                "type": "lvmthin",
                "active": 1,
                "content": "rootdir,images",
                "total": 536870912000u64,
                "used": 53687091200u64,
                "avail": 483183820800u64,
                "shared": 0
            }
        ]
    });
    
    let path_str = format!("/api2/json/nodes/{}/storage", node);
    Mock::given(method("GET"))
        .and(path(path_str))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

/// Mocks storage content listing.
pub async fn mock_storage_content_success(server: &MockServer, node: &str, storage: &str, content_type: &str) {
    let response = json!({
        "data": [
            {
                "volid": "local:iso/ubuntu-22.04.1-live-server-amd64.iso",
                "format": "iso",
                "size": 1975681024u64,
                "content": "iso"
            }
        ]
    });
    
    let path_str = format!("/api2/json/nodes/{}/storage/{}/content", node, storage);
    Mock::given(method("GET"))
        .and(path(path_str))
        .and(query_param("content", content_type)) // Proxmox usually filters via query param `?content=iso`
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

/// Mocks network interface listing.
pub async fn mock_network_list_success(server: &MockServer, node: &str) {
    let response = json!({
        "data": [
            {
                "iface": "lo",
                "type": "loopback",
                "active": 1,
                "autostart": 1,
                "method": "loopback"
            },
            {
                "iface": "vmbr0",
                "type": "bridge",
                "active": 1,
                "autostart": 1,
                "method": "static",
                "address": "192.168.1.10",
                "cidr": "192.168.1.10/24",
                "gateway": "192.168.1.1",
                "bridge_ports": "eno1",
                "bridge_stp": "off",
                "bridge_fd": "0"
            }
        ]
    });
    
    let path_str = format!("/api2/json/nodes/{}/network", node);
    Mock::given(method("GET"))
        .and(path(path_str))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

/// Mocks snapshot listing.
pub async fn mock_snapshot_list_success(server: &MockServer, node: &str, vmid: u64) {
    let response = json!({
        "data": [
            {
                "name": "current",
                "description": "current state",
                "parent": "snap1",
                "snaptime": 1234567891
            },
            {
                "name": "snap1",
                "description": "baseline",
                "snaptime": 1234567890
            }
        ]
    });
    
    let path_str = format!("/api2/json/nodes/{}/qemu/{}/snapshot", node, vmid);
    Mock::given(method("GET"))
        .and(path(path_str))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

/// Mocks backup listing via storage content.
pub async fn mock_backup_list_success(server: &MockServer, node: &str, storage: &str, _vmid: Option<u64>) {
     let response = json!({
        "data": [
            {
                "volid": "local:backup/vzdump-qemu-100-2023_01_01-00_00_00.vma.zst",
                "format": "vma.zst",
                "size": 1024000u64,
                "content": "backup",
                "vmid": 100
            }
        ]
    });
    
    let path_str = format!("/api2/json/nodes/{}/storage/{}/content", node, storage);
    
    // Note: We ignore vmid filtering here because wiremock query param matching for 
    // strictly optional params logic can be verbose, and `query_param` matches if present.
    // If client code filters client-side, returning all backups is safe for mock.
    
    Mock::given(method("GET"))
        .and(path(path_str))
        .and(query_param("content", "backup"))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

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
    let resp = client.post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;
    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    // Test 2: Cluster Status
    let cluster_url = format!("{}/api2/json/cluster/status", base_url);
    let resp = client.get(&cluster_url)
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
    let resp = client.post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;
    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    // Start VM
    let start_url = format!("{}/api2/json/nodes/{}/qemu/{}/status/start", base_url, node, vmid);
    let resp = client.post(&start_url)
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "Start VM should succeed");
    let upid_response: ProxmoxResponse<String> = resp.json().await?;
    assert_eq!(upid_response.data, upid);

    // Wait for Task
    let task_url = format!("{}/api2/json/nodes/{}/tasks/{}/status", base_url, node, upid);
    let resp = client.get(&task_url)
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
    let resp = client.post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;
    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    // List Storage
    let storage_url = format!("{}/api2/json/nodes/{}/storage", base_url, node);
    let resp = client.get(&storage_url)
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;
    
    assert_eq!(resp.status(), 200, "Storage listing should succeed");
    let storage_body: Value = resp.json().await?;
    assert!(!storage_body["data"].as_array().unwrap().is_empty());

    // List ISOs
    let content_url = format!("{}/api2/json/nodes/{}/storage/{}/content", base_url, node, storage);
    let resp = client.get(&content_url)
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
    let resp = client.post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;
    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    // List Networks
    let network_url = format!("{}/api2/json/nodes/{}/network", base_url, node);
    let resp = client.get(&network_url)
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
    let resp = client.post(&login_url)
        .form(&[("username", "root@pam"), ("password", "secret")])
        .send()
        .await?;
    let body: ProxmoxResponse<TicketResponse> = resp.json().await?;
    let ticket = body.data.ticket;
    let csrf_token = body.data.csrf_token;

    // List Snapshots
    let snap_url = format!("{}/api2/json/nodes/{}/qemu/{}/snapshot", base_url, node, vmid);
    let resp = client.get(&snap_url)
        .header("CSRFPreventionToken", &csrf_token)
        .header("Cookie", format!("PVEAuthCookie={}", ticket))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let snap_body: Value = resp.json().await?;
    assert!(!snap_body["data"].as_array().unwrap().is_empty());

    // List Backups (via storage content)
    let backup_url = format!("{}/api2/json/nodes/{}/storage/{}/content", base_url, node, storage);
    let resp = client.get(&backup_url)
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