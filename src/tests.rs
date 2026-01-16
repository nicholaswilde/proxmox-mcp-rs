#[cfg(test)]
mod tests {
    use crate::mcp::McpServer;
    use crate::proxmox::ProxmoxClient;
    use serde_json::json;
    use url::Url;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
        assert!(
            content.contains("Test log message"),
            "Log file missing expected content"
        );
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

        let server = McpServer::new(client, false);

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
        let server = McpServer::new(client, false);

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
        let server = McpServer::new(client, false);

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
        let server = McpServer::new(client, false);
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
        let server = McpServer::new(client, false);

        // Test list_vms (should return both)
        let res_vms = server.call_tool("list_vms", &json!({})).await.unwrap();
        let text_vms = res_vms["content"][0]["text"].as_str().unwrap();
        assert!(text_vms.contains("100"));
        assert!(text_vms.contains("200"));

        // Test list_containers (should return only lxc)
        let res_ct = server
            .call_tool("list_containers", &json!({}))
            .await
            .unwrap();
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
        let server = McpServer::new(client, false);

        let args = json!({ "node": "pve1", "vmid": 100 });
        // Default type is qemu
        let res = server.call_tool("start_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("initiated"));
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
        let server = McpServer::new(client, false);

        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("stop_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("initiated"));
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
        let server = McpServer::new(client, false);

        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("shutdown_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("initiated"));
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
        let server = McpServer::new(client, false);

        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("reboot_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("initiated"));
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
        let server = McpServer::new(client, false);

        let args = json!({ "node": "pve1", "vmid": 101, "name": "test-vm", "memory": 2048 });
        let res = server.call_tool("create_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("initiated"));
    }

    #[tokio::test]
    async fn test_delete_vm() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api2/json/nodes/pve1/qemu/100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("delete_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("initiated"));
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
        let server = McpServer::new(client, false);

        let args = json!({
            "node": "pve1",
            "vmid": 102,
            "ostemplate": "local:vztmpl/ubuntu.tar.gz",
            "hostname": "test-ct"
        });
        let res = server.call_tool("create_container", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("initiated"));
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
            .and(path(
                "/api2/json/nodes/pve1/qemu/100/snapshot/snap1/rollback",
            ))
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
        let server = McpServer::new(client, false);

        // Test List
        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("list_snapshots", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("snap1"));

        // Test Create
        let args = json!({ "node": "pve1", "vmid": 100, "snapname": "snap1" });
        let res = server.call_tool("snapshot_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("created"));

        // Test Rollback
        let args = json!({ "node": "pve1", "vmid": 100, "snapname": "snap1" });
        let res = server.call_tool("rollback_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Rollback"));

        // Test Delete
        let args = json!({ "node": "pve1", "vmid": 100, "snapname": "snap1" });
        let res = server.call_tool("delete_snapshot", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Delete"));
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
        let server = McpServer::new(client, false);

        let args = json!({ "node": "pve1", "vmid": 200 });
        let res = server.call_tool("delete_container", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("initiated"));
    }

    #[tokio::test]
    async fn test_clone_vm() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/clone"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        let args = json!({ "node": "pve1", "vmid": 100, "newid": 102, "name": "cloned-vm" });
        let res = server.call_tool("clone_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Clone initiated"));
    }

    #[tokio::test]
    async fn test_migrate_vm() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/migrate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        let args = json!({ "node": "pve1", "vmid": 100, "target_node": "pve2" });
        let res = server.call_tool("migrate_vm", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Migration initiated"));
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
        let server = McpServer::new(client, false);

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
        let server = McpServer::new(client, false);

        // Update both
        let args = json!({
            "node": "pve1",
            "vmid": 200,
            "cores": 2,
            "memory": 1024,
            "disk_gb": 5
        });

        let res = server
            .call_tool("update_container_resources", &args)
            .await
            .unwrap();
        let content = res["content"][0]["text"].as_str().unwrap();

        assert!(content.contains("Resource config updated"));
        assert!(content.contains("Disk rootfs resize initiated"));
    }

    #[tokio::test]
    async fn test_update_vm_resources() {
        let mock_server = MockServer::start().await;

        // Mock Config Update
        Mock::given(method("PUT"))
            .and(path("/api2/json/nodes/pve1/qemu/100/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock Disk Resize
        Mock::given(method("PUT"))
            .and(path("/api2/json/nodes/pve1/qemu/100/resize"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Update both
        let args = json!({
            "node": "pve1",
            "vmid": 100,
            "cores": 4,
            "memory": 4096,
            "sockets": 2,
            "disk_gb": 10
        });

        let res = server
            .call_tool("update_vm_resources", &args)
            .await
            .unwrap();
        let content = res["content"][0]["text"].as_str().unwrap();

        assert!(content.contains("Resource config updated"));
        assert!(content.contains("Disk rootfs resize initiated"));
    }

    #[tokio::test]
    async fn test_backup_tools() {
        let mock_server = MockServer::start().await;

        // Mock list_backups
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/storage/local/content"))
            // .and(query_param("content", "backup"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "volid": "local:backup/vzdump-qemu-100-2022.vma.zst", "content": "backup", "vmid": 100 }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock create_backup
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/vzdump"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        // Mock restore_backup (qemu)
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:..." })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test List
        let args = json!({ "node": "pve1", "storage": "local" });
        let res = server.call_tool("list_backups", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("vzdump-qemu"));

        // Test Create
        let args = json!({ "node": "pve1", "vmid": 100, "mode": "snapshot", "compress": "zstd" });
        let res = server.call_tool("create_backup", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Backup initiated"));

        // Test Restore
        let args = json!({
            "node": "pve1",
            "vmid": 105,
            "archive": "local:backup/vzdump-qemu-100.vma.zst",
            "type": "qemu",
            "storage": "local-lvm"
        });
        let res = server.call_tool("restore_backup", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Restore initiated"));
    }

    #[tokio::test]
    async fn test_task_monitoring() {
        let mock_server = MockServer::start().await;

        let upid = "UPID:pve1:00000000:00000000:00000000:test:qmstart:100:root@pam:";

        // Mock running status
        Mock::given(method("GET"))
            .and(path(format!("/api2/json/nodes/pve1/tasks/{}/status", upid)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": { "status": "stopped", "exitstatus": "OK" }
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test get_task_status
        let args = json!({ "node": "pve1", "upid": upid });
        let res = server.call_tool("get_task_status", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("stopped"));

        // Test wait_for_task (should return immediately because we mocked stopped)
        let args = json!({ "node": "pve1", "upid": upid, "timeout": 5 });
        let res = server.call_tool("wait_for_task", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Task finished"));
    }

    #[tokio::test]
    async fn test_list_networks() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/network"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "iface": "vmbr0", "type": "bridge", "active": 1 }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        let args = json!({ "node": "pve1" });
        let res = server.call_tool("list_networks", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("vmbr0"));
    }

    #[tokio::test]
    async fn test_storage_tools() {
        let mock_server = MockServer::start().await;

        // Mock list_storage
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/storage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "storage": "local", "content": "iso,vztmpl,backup", "type": "dir", "active": 1 }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock list_isos
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/storage/local/content"))
            // .and(query_param("content", "iso"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "volid": "local:iso/debian-11.0.0-amd64-netinst.iso", "content": "iso" }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test list_storage
        let args = json!({ "node": "pve1" });
        let res = server.call_tool("list_storage", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("local"));

        // Test list_isos
        let args = json!({ "node": "pve1", "storage": "local" });
        let res = server.call_tool("list_isos", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("debian"));
    }

    #[tokio::test]
    async fn test_resources() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "vmid": 100, "node": "pve1", "type": "qemu", "status": "running" }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test resources/list via handle_request
        let req = crate::mcp::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/list".to_string(),
            params: None,
            id: Some(json!(1)),
        };
        let res = server.handle_request(req).await.unwrap();
        let resources = res["resources"].as_array().unwrap();
        assert!(resources.iter().any(|r| r["uri"] == "proxmox://vms"));

        // Test resources/read
        let req = crate::mcp::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({ "uri": "proxmox://vms" })),
            id: Some(json!(2)),
        };
        let res = server.handle_request(req).await.unwrap();
        let text = res["contents"][0]["text"].as_str().unwrap();
        assert!(text.contains("100"));
        assert!(text.contains("running"));
    }

    #[tokio::test]
    async fn test_cluster_tools() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "name": "pve1", "type": "node", "status": "online" }
                ]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/log"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "msg": "cluster ready" }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        let res = server
            .call_tool("get_cluster_status", &json!({}))
            .await
            .unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("pve1"));

        let res = server
            .call_tool("get_cluster_log", &json!({}))
            .await
            .unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("cluster ready"));
    }

    #[tokio::test]
    async fn test_firewall_tools() {
        let mock_server = MockServer::start().await;

        // Mock list_firewall_rules (Cluster)
        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/firewall/rules"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "pos": 0, "type": "in", "action": "ACCEPT", "comment": "Allow SSH" }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock add_firewall_rule (VM)
        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "vmid": 100, "node": "pve1", "type": "qemu", "status": "running" }
                ]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/firewall/rules"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock delete_firewall_rule (Cluster)
        Mock::given(method("DELETE"))
            .and(path("/api2/json/cluster/firewall/rules/0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test list (Cluster)
        let res = server
            .call_tool("list_firewall_rules", &json!({}))
            .await
            .unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Allow SSH"));

        // Test add (VM)
        let args = json!({
            "node": "pve1",
            "vmid": 100,
            "type": "in",
            "action": "DROP",
            "proto": "tcp",
            "dport": "80"
        });
        let res = server.call_tool("add_firewall_rule", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("added"));

        // Test delete (Cluster)
        let args = json!({ "pos": 0 });
        let res = server
            .call_tool("delete_firewall_rule", &args)
            .await
            .unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("deleted"));
    }

    #[tokio::test]
    async fn test_hardware_config() {
        let mock_server = MockServer::start().await;

        // Mock Update Config (Generic success for all calls)
        Mock::given(method("PUT"))
            .and(path("/api2/json/nodes/pve1/qemu/100/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // 1. Add Disk
        let args = json!({
            "node": "pve1",
            "vmid": 100,
            "device": "scsi1",
            "storage": "local-lvm",
            "size_gb": 32,
            "format": "qcow2"
        });
        let res = server.call_tool("add_disk", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("added"));

        // 2. Remove Disk
        let args = json!({
            "node": "pve1",
            "vmid": 100,
            "device": "scsi1"
        });
        let res = server.call_tool("remove_disk", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("removed"));

        // 3. Add Network
        let args = json!({
            "node": "pve1",
            "vmid": 100,
            "device": "net1",
            "bridge": "vmbr0",
            "model": "virtio"
        });
        let res = server.call_tool("add_network", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("added"));

        // 4. Remove Network
        let args = json!({
            "node": "pve1",
            "vmid": 100,
            "device": "net1"
        });
        let res = server.call_tool("remove_network", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("removed"));
    }

    #[tokio::test]
    async fn test_rrd_stats() {
        let mock_server = MockServer::start().await;

        // Mock Node Stats
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/rrddata"))
            // .and(query_param("timeframe", "hour"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "time": 1000, "cpu": 0.1 }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock VM Stats
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/qemu/100/rrddata"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "time": 1000, "cpu": 0.5 }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // 1. Get Node Stats
        let args = json!({ "node": "pve1", "timeframe": "hour" });
        let res = server.call_tool("get_node_stats", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("0.1"));

        // 2. Get VM Stats
        let args = json!({ "node": "pve1", "vmid": 100, "type": "qemu", "timeframe": "day" });
        let res = server.call_tool("get_vm_stats", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("0.5"));
    }

    #[tokio::test]
    async fn test_download_url() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/storage/local/download-url"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:pve1:..." })),
            )
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        let args = json!({
            "node": "pve1",
            "storage": "local",
            "url": "http://example.com/debian.iso",
            "filename": "debian.iso",
            "content": "iso"
        });

        let res = server.call_tool("download_url", &args).await.unwrap();
        let content = res["content"][0]["text"].as_str().unwrap();

        assert!(content.contains("Download initiated"));
        assert!(content.contains("UPID:pve1"));
    }

    #[tokio::test]
    async fn test_user_management() {
        let mock_server = MockServer::start().await;

        // Mock list_users
        Mock::given(method("GET"))
            .and(path("/api2/json/access/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "userid": "test@pve", "enable": 1 }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock create_user
        Mock::given(method("POST"))
            .and(path("/api2/json/access/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock delete_user
        Mock::given(method("DELETE"))
            .and(path("/api2/json/access/users/test@pve"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test List
        let res = server.call_tool("list_users", &json!({})).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("test@pve"));

        // Test Create
        let args = json!({
            "userid": "newuser@pve",
            "password": "password123",
            "email": "new@example.com",
            "groups": ["admin"]
        });
        let res = server.call_tool("create_user", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("created"));

        // Test Delete
        let args = json!({ "userid": "test@pve" });
        let res = server.call_tool("delete_user", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("deleted"));
    }

    #[tokio::test]
    async fn test_cluster_storage_management() {
        let mock_server = MockServer::start().await;

        // Mock list_cluster_storage
        Mock::given(method("GET"))
            .and(path("/api2/json/storage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "storage": "local", "type": "dir" }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock add_storage
        Mock::given(method("POST"))
            .and(path("/api2/json/storage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock delete_storage
        Mock::given(method("DELETE"))
            .and(path("/api2/json/storage/teststorage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock update_storage
        Mock::given(method("PUT"))
            .and(path("/api2/json/storage/teststorage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test List
        let res = server
            .call_tool("list_cluster_storage", &json!({}))
            .await
            .unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("local"));

        // Test Add
        let args = json!({
            "storage": "teststorage",
            "type": "nfs",
            "server": "1.2.3.4",
            "export": "/srv/nfs",
            "content": "iso,backup"
        });
        let res = server.call_tool("add_storage", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("added"));

        // Test Update
        let args = json!({
            "storage": "teststorage",
            "enable": false
        });
        let res = server.call_tool("update_storage", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("updated"));

        // Test Delete
        let args = json!({ "storage": "teststorage" });
        let res = server.call_tool("delete_storage", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("deleted"));
    }

    #[tokio::test]
    async fn test_qemu_agent_tools() {
        let mock_server = MockServer::start().await;

        // Mock Ping
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/agent/ping"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": {} })))
            .mount(&mock_server)
            .await;

        // Mock Exec
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/agent/exec"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "pid": 1234 } })),
            )
            .mount(&mock_server)
            .await;

        // Mock Exec Status
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/qemu/100/agent/exec-status"))
            // .and(query_param("pid", "1234"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "data": { "exited": 1, "out-data": "hello" } })),
            )
            .mount(&mock_server)
            .await;

        // Mock File Read
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/qemu/100/agent/file-read"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "data": { "content": "file content" } })),
            )
            .mount(&mock_server)
            .await;

        // Mock File Write
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/qemu/100/agent/file-write"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": {} })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test Ping
        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("vm_agent_ping", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("Pong"));

        // Test Exec
        let args = json!({ "node": "pve1", "vmid": 100, "command": "echo hello" });
        let res = server.call_tool("vm_exec", &args).await.unwrap();
        assert!(res["content"][0]["text"].as_str().unwrap().contains("1234"));

        // Test Exec Status
        let args = json!({ "node": "pve1", "vmid": 100, "pid": 1234 });
        let res = server.call_tool("vm_exec_status", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("hello"));

        // Test Read File
        let args = json!({ "node": "pve1", "vmid": 100, "file": "/tmp/test" });
        let res = server.call_tool("vm_read_file", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("file content"));

        // Test Write File
        let args = json!({ "node": "pve1", "vmid": 100, "file": "/tmp/test", "content": "foo" });
        let res = server.call_tool("vm_write_file", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("File written"));
    }

    #[tokio::test]
    async fn test_lazy_loading() {
        let mock_server = MockServer::start().await;
        // Mock cluster/resources for full list check later
        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": []
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, true); // lazy_mode = true

        // 1. Check initial tool list
        let req = crate::mcp::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: None,
            id: Some(json!(1)),
        };
        let res = server.handle_request(req).await.unwrap();
        let tools = res["tools"].as_array().unwrap();

        // Should contain load_all_tools
        assert!(tools.iter().any(|t| t["name"] == "load_all_tools"));
        // Should NOT contain list_vms
        assert!(!tools.iter().any(|t| t["name"] == "list_vms"));
        // Should contain list_nodes (as meta tool)
        assert!(tools.iter().any(|t| t["name"] == "list_nodes"));

        // 2. Load all tools
        let res_load = server
            .call_tool("load_all_tools", &json!({}))
            .await
            .unwrap();
        assert!(res_load["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("loaded"));

        // 3. Check tool list again
        let req = crate::mcp::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: None,
            id: Some(json!(2)),
        };
        let res = server.handle_request(req).await.unwrap();
        let tools = res["tools"].as_array().unwrap();

        // Should contain list_vms now
        assert!(tools.iter().any(|t| t["name"] == "list_vms"));
    }

    #[tokio::test]
    async fn test_pool_management() {
        let mock_server = MockServer::start().await;

        // Mock list_pools
        Mock::given(method("GET"))
            .and(path("/api2/json/pools"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "poolid": "testpool", "comment": "test comment" }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock create_pool
        Mock::given(method("POST"))
            .and(path("/api2/json/pools"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock get_pool_details
        Mock::given(method("GET"))
            .and(path("/api2/json/pools/testpool"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": { "poolid": "testpool", "comment": "test comment", "members": [] }
            })))
            .mount(&mock_server)
            .await;

        // Mock update_pool
        Mock::given(method("PUT"))
            .and(path("/api2/json/pools/testpool"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock delete_pool
        Mock::given(method("DELETE"))
            .and(path("/api2/json/pools/testpool"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test List
        let res = server.call_tool("list_pools", &json!({})).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("testpool"));

        // Test Create
        let args = json!({ "poolid": "newpool", "comment": "new" });
        let res = server.call_tool("create_pool", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("created"));

        // Test Details
        let args = json!({ "poolid": "testpool" });
        let res = server.call_tool("get_pool_details", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("members"));

        // Test Update
        let args = json!({ "poolid": "testpool", "comment": "updated" });
        let res = server.call_tool("update_pool", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("updated"));

        // Test Delete
        let args = json!({ "poolid": "testpool" });
        let res = server.call_tool("delete_pool", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("deleted"));
    }

    #[tokio::test]
    async fn test_ha_management() {
        let mock_server = MockServer::start().await;

        // Mock list_ha_resources
        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/ha/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "sid": "vm:100", "state": "started" }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock list_ha_groups
        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/ha/groups"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "group": "testgroup" }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock add_ha_resource
        Mock::given(method("POST"))
            .and(path("/api2/json/cluster/ha/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock update_ha_resource
        Mock::given(method("PUT"))
            .and(path("/api2/json/cluster/ha/resources/vm:100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock delete_ha_resource
        Mock::given(method("DELETE"))
            .and(path("/api2/json/cluster/ha/resources/vm:100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test List Resources
        let res = server
            .call_tool("list_ha_resources", &json!({}))
            .await
            .unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("vm:100"));

        // Test List Groups
        let res = server
            .call_tool("list_ha_groups", &json!({}))
            .await
            .unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("testgroup"));

        // Test Add
        let args = json!({ "sid": "vm:101", "state": "started" });
        let res = server.call_tool("add_ha_resource", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("added"));

        // Test Update
        let args = json!({ "sid": "vm:100", "state": "stopped" });
        let res = server.call_tool("update_ha_resource", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("updated"));

        // Test Remove
        let args = json!({ "sid": "vm:100" });
        let res = server.call_tool("remove_ha_resource", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("removed"));
    }

    #[tokio::test]
    async fn test_roles_and_acls() {
        let mock_server = MockServer::start().await;

        // Mock list_roles
        Mock::given(method("GET"))
            .and(path("/api2/json/access/roles"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "roleid": "Admin", "privs": "VM.Config.HW,VM.Config.Disk" }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock create_role
        Mock::given(method("POST"))
            .and(path("/api2/json/access/roles"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        // Mock list_acls
        Mock::given(method("GET"))
            .and(path("/api2/json/access/acl"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "path": "/", "user": "root@pam", "role": "Administrator" }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Mock update_acl
        Mock::given(method("PUT"))
            .and(path("/api2/json/access/acl"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test List Roles
        let res = server.call_tool("list_roles", &json!({})).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Admin"));

        // Test Create Role
        let args = json!({ "roleid": "NewRole", "privs": "VM.Audit" });
        let res = server.call_tool("create_role", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("created"));

        // Test List ACLs
        let res = server.call_tool("list_acls", &json!({})).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("root@pam"));

        // Test Update ACL
        let args = json!({ "path": "/vms/100", "users": "test@pve", "roles": "PVEVMAdmin" });
        let res = server.call_tool("update_acl", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("updated"));
    }

    #[tokio::test]
    async fn test_apt_and_services() {
        let mock_server = MockServer::start().await;

        // Mock list_apt_updates
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/apt/update"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{ "Package": "pve-manager", "Version": "7.0.1" }]
            })))
            .mount(&mock_server)
            .await;

        // Mock run_apt_update
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/apt/update"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:pve1:..." })),
            )
            .mount(&mock_server)
            .await;

        // Mock list_services
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/services"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{ "service": "pvestatd", "state": "running" }]
            })))
            .mount(&mock_server)
            .await;

        // Mock manage_service (restart)
        Mock::given(method("POST"))
            .and(path("/api2/json/nodes/pve1/services/pvestatd/restart"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "data": "UPID:pve1:..." })),
            )
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test APT List
        let args = json!({ "node": "pve1" });
        let res = server.call_tool("list_apt_updates", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("pve-manager"));

        // Test APT Run
        let res = server.call_tool("run_apt_update", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("UPID:pve1"));

        // Test Services List
        let res = server.call_tool("list_services", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("pvestatd"));

        // Test Service Manage
        let args = json!({ "node": "pve1", "service": "pvestatd", "action": "restart" });
        let res = server.call_tool("manage_service", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("restart initiated"));
    }

    #[tokio::test]
    async fn test_cloudinit_and_tags() {
        let mock_server = MockServer::start().await;

        // Mock get_vm_config (needed for add/remove tag logic)
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes/pve1/qemu/100/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": { "tags": "prod,linux" }
            })))
            .mount(&mock_server)
            .await;

        // Mock update_config (Cloud-Init and Tags)
        // We use a broader match here because multiple tools use this endpoint
        Mock::given(method("PUT"))
            .and(path("/api2/json/nodes/pve1/qemu/100/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": null })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test Cloud-Init
        let args = json!({
            "node": "pve1",
            "vmid": 100,
            "ciuser": "admin",
            "ipconfig0": "ip=dhcp"
        });
        let res = server.call_tool("set_vm_cloudinit", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("updated"));

        // Test Set Tags
        let args = json!({ "node": "pve1", "vmid": 100, "tags": "dev,test" });
        let res = server.call_tool("set_tags", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Tags set"));

        // Test Add Tag (Logic test mostly)
        let args = json!({ "node": "pve1", "vmid": 100, "tags": "newtag" });
        let res = server.call_tool("add_tag", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Tags added"));

        // Test Remove Tag
        let args = json!({ "node": "pve1", "vmid": 100, "tags": "prod" });
        let res = server.call_tool("remove_tag", &args).await.unwrap();
        assert!(res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Tags removed"));
    }

    #[tokio::test]
    async fn test_console_url() {
        // We don't really need a mock server running for this, but create_test_client uses it.
        // We can just use a dummy URL.
        let client = ProxmoxClient::new("https://pve.example.com", 8006, true).unwrap();
        let server = McpServer::new(client, false);

        // Test QEMU NoVNC (default)
        let args = json!({ "node": "pve1", "vmid": 100 });
        let res = server.call_tool("get_console_url", &args).await.unwrap();
        let url = res["content"][0]["text"].as_str().unwrap();

        assert!(url.contains("https://pve.example.com:8006/"));
        assert!(url.contains("console=kvm"));
        assert!(url.contains("novnc=1"));
        assert!(url.contains("vmid=100"));
        assert!(url.contains("node=pve1"));

        // Test LXC xterm.js
        let args = json!({ "node": "pve1", "vmid": 200, "type": "lxc", "console": "xtermjs" });
        let res = server.call_tool("get_console_url", &args).await.unwrap();
        let url = res["content"][0]["text"].as_str().unwrap();

        assert!(url.contains("console=lxc"));
        assert!(url.contains("xtermjs=1"));
    }

    #[tokio::test]
    async fn test_api_error_handling() {
        let mock_server = MockServer::start().await;

        // Mock 401 Unauthorized
        Mock::given(method("GET"))
            .and(path("/api2/json/nodes"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        // Mock 404 Not Found (using a different endpoint for variety)
        Mock::given(method("GET"))
            .and(path("/api2/json/cluster/resources"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server.uri());
        let server = McpServer::new(client, false);

        // Test 401
        let req = crate::mcp::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({ "name": "list_nodes", "arguments": {} })),
            id: Some(json!(1)),
        };
        // We capture stdout in run_stdio, but handle_request returns Result<Value>.
        // Wait, handle_request returns Err on failure. run_stdio wraps it.
        // We can check the error returned by handle_request.
        let res = server.handle_request(req).await;
        assert!(res.is_err());
        let err = res.err().unwrap();
        // The error is anyhow::Error wrapping ProxmoxError.
        let pve_err = err.downcast_ref::<crate::proxmox::ProxmoxError>().unwrap();
        match pve_err {
            crate::proxmox::ProxmoxError::Api(status, _) => assert_eq!(status.as_u16(), 401),
            _ => panic!("Expected Api error"),
        }

        // Test 404
        let req = crate::mcp::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({ "name": "list_vms", "arguments": {} })),
            id: Some(json!(2)),
        };
        let res = server.handle_request(req).await;
        assert!(res.is_err());
        let err = res.err().unwrap();
        let pve_err = err.downcast_ref::<crate::proxmox::ProxmoxError>().unwrap();
        match pve_err {
            crate::proxmox::ProxmoxError::Api(status, _) => assert_eq!(status.as_u16(), 404),
            _ => panic!("Expected Api error"),
        }
    }
}
