use proxmox_mcp_rs::mcp::McpServer;
use proxmox_mcp_rs::proxmox::ProxmoxClient;
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
    // Or we just manually set them if the client allows it.
    // ProxmoxClient has internal state for ticket/csrf.
    
    // In our case, we can just call client.login with dummy creds since we will mock the response.
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
