#[cfg(test)]
mod tests {
    use crate::proxmox::ProxmoxClient;
    use crate::mcp::McpServer;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use serde_json::json;

    #[tokio::test]
    async fn test_reset_vm() {
        let mock_server = MockServer::start().await;

        // Mock /cluster/resources for location lookup
        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "vmid": 100, "node": "pve1", "type": "qemu", "status": "running" }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock reset action
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/status/reset"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": "UPID:pve1:..."
            })))
            .mount(&mock_server)
            .await;

        let client = ProxmoxClient::new(&mock_server.uri(), true).unwrap();
        // Skip login for this test or mock it.
        // ProxmoxClient usually checks login? No, it just sets up client.
        // BUT request() method might fail if no ticket, unless we mock the response ignoring auth headers.
        // Our mock server ignores headers by default unless matched.

        let server = McpServer::new(client);
        
        let args = json!({ "vm_id": "100" });
        let res = server.call_tool("reset_vm", &args).await.unwrap();
        
        let content = res["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("Reset initiated"));
        assert!(content.contains("UPID:pve1"));
    }

    #[tokio::test]
    async fn test_reset_container() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "vmid": 200, "node": "pve1", "type": "lxc", "status": "running" }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock reboot action for container
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/lxc/200/status/reboot"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": "UPID:pve1:..."
            })))
            .mount(&mock_server)
            .await;

        let client = ProxmoxClient::new(&mock_server.uri(), true).unwrap();
        let server = McpServer::new(client);
        
        let args = json!({ "container_id": "200" });
        let res = server.call_tool("reset_container", &args).await.unwrap();

        let content = res["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("Reset initiated"));
    }
    
    #[tokio::test]
    async fn test_reset_vm_invalid_id() {
        let mock_server = MockServer::start().await;
         Mock::given(method("GET"))
            .and(path("/api2/json/cluster/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": []
            })))
            .mount(&mock_server)
            .await;
            
        let client = ProxmoxClient::new(&mock_server.uri(), true).unwrap();
        let server = McpServer::new(client);
        
        let args = json!({ "vm_id": "999" });
        let res = server.call_tool("reset_vm", &args).await;
        
        assert!(res.is_err());
    }
}
