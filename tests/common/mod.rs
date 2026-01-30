#![allow(dead_code)]
use proxmox_mcp_rs::mcp::McpServer;
use proxmox_mcp_rs::proxmox::ProxmoxClient;
use serde_json::json;
use std::collections::HashMap;
use url::Url;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

/// Mocks a failed authentication flow (POST /api2/json/access/ticket).
pub async fn mock_auth_failure(server: &MockServer) {
    let response = json!({
        "data": null,
        "errors": {
            "login": "authentication failed"
        }
    });

    Mock::given(method("POST"))
        .and(path("/api2/json/access/ticket"))
        .respond_with(ResponseTemplate::new(401).set_body_json(response))
        .mount(server)
        .await;
}

/// Mocks an unauthorized request (401 Unauthorized).
pub async fn mock_unauthorized_request(server: &MockServer, method_str: &str, path_str: &str) {
    let response = json!({
        "data": null,
        "errors": {
            "error": "401 Unauthorized"
        }
    });

    Mock::given(method(method_str))
        .and(path(path_str))
        .respond_with(ResponseTemplate::new(401).set_body_json(response))
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
        .and(header(
            "Cookie",
            format!("PVEAuthCookie={}", TEST_TICKET).as_str(),
        ))
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
        .and(header(
            "Cookie",
            format!("PVEAuthCookie={}", TEST_TICKET).as_str(),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(status_response))
        .mount(server)
        .await;
}

/// Mocks a successful VM action (start, stop, etc.) returning a UPID.
pub async fn mock_vm_action_success(
    server: &MockServer,
    node: &str,
    vmid: u64,
    action: &str,
    upid: &str,
) {
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
pub async fn mock_storage_content_success(
    server: &MockServer,
    node: &str,
    storage: &str,
    content_type: &str,
) {
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
pub async fn mock_backup_list_success(
    server: &MockServer,
    node: &str,
    storage: &str,
    _vmid: Option<u64>,
) {
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

/// Mocks a missing VM (500/400/404 on actions).
pub async fn mock_vm_not_found(server: &MockServer, node: &str, vmid: u64) {
    // PVE returns 500 or 400 or 404 depending on endpoint.
    // Usually if VM doesn't exist, `qm status` might return "VM 100 not running" or similar,
    // but REST API `GET /nodes/{node}/qemu/{vmid}/status/current` returns 500 "Configuration file ... not found" or 400.
    // Let's mock 500 for "VM not found" as that's common in PVE API for missing configs.

    let path_str = format!("/api2/json/nodes/{}/qemu/{}/status/current", node, vmid);
    Mock::given(method("GET"))
        .and(path(path_str))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .respond_with(
            ResponseTemplate::new(500).set_body_string(
                "Configuration file 'nodes/pve/qemu-server/9999.conf' does not exist",
            ),
        )
        .mount(server)
        .await;
}

/// Mocks a successful VM list retrieval (GET /api2/json/cluster/resources).
pub async fn mock_list_vms_success(server: &MockServer) {
    let response = json!({
        "data": [
            {
                "vmid": 100,
                "name": "vm-100",
                "status": "running",
                "node": "pve-node-01",
                "type": "qemu"
            },
            {
                "vmid": 101,
                "name": "vm-101",
                "status": "stopped",
                "node": "pve-node-02",
                "type": "qemu"
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/api2/json/cluster/resources"))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .and(header(
            "Cookie",
            format!("PVEAuthCookie={}", TEST_TICKET).as_str(),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

/// Mocks a successful VM configuration update (PUT /api2/json/nodes/{node}/{type}/{vmid}/config).
pub async fn mock_vm_config_update_success(
    server: &MockServer,
    node: &str,
    vmid: u64,
    vm_type: &str,
) {
    let response = json!({
        "data": null
    });

    let path_str = format!("/api2/json/nodes/{}/{}/{}/config", node, vm_type, vmid);

    Mock::given(method("PUT"))
        .and(path(path_str))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .and(header(
            "Cookie",
            format!("PVEAuthCookie={}", TEST_TICKET).as_str(),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

/// Mocks a 404 Not Found error (GET /api2/json/nodes).
pub async fn mock_resource_not_found(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/api2/json/nodes"))
        .respond_with(ResponseTemplate::new(404))
        .mount(server)
        .await;
}

/// Mocks an API timeout.
pub async fn mock_api_timeout(server: &MockServer) {
    use std::time::Duration;
    // Mock a timeout on node listing
    Mock::given(method("GET"))
        .and(path("/api2/json/nodes"))
        .and(header("CSRFPreventionToken", TEST_CSRF_TOKEN))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(server)
        .await;
}
