#[cfg(test)]
mod tests {
    use crate::proxmox::ProxmoxClient;
    use crate::mcp::McpServer;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use serde_json::json;
    use url::Url;

    fn create_test_client(uri: &str) -> ProxmoxClient {
        let url = Url::parse(uri).unwrap();
        let host_str = format!("{}://{}", url.scheme(), url.host_str().unwrap());
        ProxmoxClient::new(&host_str, url.port().unwrap(), true).unwrap()
    }

    #[test]
    fn test_file_logging_setup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_dir = temp_dir.path().to_str().unwrap();
        let log_filename = "test.log";
        
        let file_appender = tracing_appender::rolling::never(log_dir, log_filename);
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        
        let subscriber = tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_ansi(false)
            .finish();
            
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("Test log message");
        });
        
        // Ensure flushing
        drop(_guard);
        std::thread::sleep(std::time::Duration::from_millis(200));
        
        let file_path = temp_dir.path().join(log_filename);
        assert!(file_path.exists(), "Log file was not created");
        
        let content = std::fs::read_to_string(file_path).unwrap();
        assert!(content.contains("Test log message"), "Log file missing expected content");
    }

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

        let client = create_test_client(&mock_server.uri());
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

        let client = create_test_client(&mock_server.uri());
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
            
        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        
        let args = json!({ "vm_id": "999" });
        let res = server.call_tool("reset_vm", &args).await;
        
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_list_nodes() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{ "node": "pve1", "status": "online" }]
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        let res = server.call_tool("list_nodes", &json!({})).await.unwrap();
        let content = res["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("pve1"));
    }

    #[tokio::test]
    async fn test_list_vms_and_containers() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "vmid": 100, "node": "pve1", "type": "qemu", "status": "running" },
                    { "vmid": 200, "node": "pve1", "type": "lxc", "status": "stopped" }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);

        // Test list_vms (should return both)
        let res_vms = server.call_tool("list_vms", &json!({})).await.unwrap();
        let text_vms = res_vms["content"][0]["text"].as_str().unwrap();
        assert!(text_vms.contains("100"));
        assert!(text_vms.contains("200"));

        // Test list_containers (should return only lxc)
        let res_ct = server.call_tool("list_containers", &json!({})).await.unwrap();
        let text_ct = res_ct["content"][0]["text"].as_str().unwrap();
        assert!(!text_ct.contains("100")); // qemu shouldn't be here
        assert!(text_ct.contains("200"));
    }

    #[tokio::test]
    async fn test_start_vm() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/status/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        
        let args = json!({ "node": "pve1", "vmid": 100 });
        // Default type is qemu
        let res = server.call_tool("start_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("initiated"));
    }

    #[tokio::test]
    async fn test_stop_vm() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/status/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        
        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("stop_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("initiated"));
    }

    #[tokio::test]
    async fn test_shutdown_vm() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/status/shutdown"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        
        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("shutdown_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("initiated"));
    }

    #[tokio::test]
    async fn test_reboot_vm() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/status/reboot"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        
        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("reboot_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("initiated"));
    }

    #[tokio::test]
    async fn test_create_vm() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        
        let args = json!({ "node": "pve1", "vmid": 101, "name": "test-vm", "memory": 2048 });
        let res = server.call_tool("create_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("initiated"));
    }

    #[tokio::test]
    async fn test_create_container() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/lxc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        
        let args = json!({ 
            "node": "pve1", 
            "vmid": 102, 
            "ostemplate": "local:vztmpl/ubuntu.tar.gz",
            "hostname": "test-ct" 
        });
        let res = server.call_tool("create_container", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("initiated"));
    }

    #[tokio::test]
    async fn test_snapshot_lifecycle() {
        let mock_server = MockServer::start().await;

        // List Snapshots
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/qemu/100/snapshot"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{ "name": "snap1", "description": "test snap" }]
            })))
            .mount(&mock_server)
            .await;

        // Create Snapshot
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/snapshot"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        // Rollback Snapshot
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/snapshot/snap1/rollback"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        // Delete Snapshot
        Mock::given(method("DELETE"))
            .and(path("/api2/json/nodes/pve1/qemu/100/snapshot/snap1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);

        // Test List
        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("list_snapshots", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("snap1"));

        // Test Create
        let args = json!({ "node": "pve1", "vmid": 100, "snapname": "snap1" });
        let res = server.call_tool("snapshot_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("created"));

        // Test Rollback
        let args = json!({ "node": "pve1", "vmid": 100, "snapname": "snap1" });
        let res = server.call_tool("rollback_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("Rollback"));

        // Test Delete
        let args = json!({ "node": "pve1", "vmid": 100, "snapname": "snap1" });
        let res = server.call_tool("delete_snapshot", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("Delete"));
    }

    #[tokio::test]
    async fn test_delete_container() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api2/json/nodes/pve1/lxc/200"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        
        let args = json!({ "node": "pve1", "vmid": 200 });
        let res = server.call_tool("delete_container", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("initiated"));
    }

    #[tokio::test]
    async fn test_list_templates() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/storage/local/content"))
            // .and(query_param("content", "vztmpl")) // WireMock matching query params needs explicit matchers
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "volid": "local:vztmpl/ubuntu-20.04-standard_20.04-1_amd64.tar.gz", "content": "vztmpl" }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        
        let args = json!({ "node": "pve1" });
        let res = server.call_tool("list_templates", &args).await.unwrap();
        let content = res["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("ubuntu-20.04"));
    }

    #[tokio::test]
    async fn test_update_container_resources() {
        let mock_server = MockServer::start().await;
        
        // Mock Config Update
        Mock::given(method("PUT"))
            .and(path("/api2/json/nodes/pve1/lxc/200/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock Disk Resize
        Mock::given(method("PUT"))
            .and(path("/api2/json/nodes/pve1/lxc/200/resize"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client);
        
        // Update both
        let args = json!({ 
            "node": "pve1", 
            "vmid": 200, 
            "cores": 2, 
            "memory": 1024,
            "disk_gb": 5 
        });
        
        let res = server.call_tool("update_container_resources", &args).await.unwrap();
        let content = res["content"][0]["text"].as_str().unwrap();
        
        assert!(content.contains("Resource config updated"));
        assert!(content.contains("Disk rootfs resize initiated"));
    }
}
