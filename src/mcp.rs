use crate::proxmox::ProxmoxClient;
use anyhow::Result;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

struct McpState {
    lazy_mode: bool,
    tools_loaded: bool,
    should_notify: bool,
}

#[derive(Clone)]
pub struct McpServer {
    clients: HashMap<String, ProxmoxClient>,
    default_id: String,
    state: Arc<Mutex<McpState>>,
}

impl McpServer {
    pub fn new(
        clients: HashMap<String, ProxmoxClient>,
        default_id: String,
        lazy_mode: bool,
    ) -> Self {
        Self {
            clients,
            default_id,
            state: Arc::new(Mutex::new(McpState {
                lazy_mode,
                tools_loaded: !lazy_mode,
                should_notify: false,
            })),
        }
    }

    fn get_client(&self, args: &Value) -> Result<&ProxmoxClient> {
        let id = args.get("instance").and_then(|v| v.as_str());
        if let Some(id) = id {
            self.clients
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("Instance '{}' not found", id))
        } else {
            self.clients
                .get(&self.default_id)
                .ok_or_else(|| anyhow::anyhow!("Default instance not found"))
        }
    }

    pub fn check_notification(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.should_notify {
            state.should_notify = false;
            true
        } else {
            false
        }
    }

    pub async fn run_stdio(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut line = String::new();

        loop {
            line.clear();
            let bytes = reader.read_line(&mut line)?;
            if bytes == 0 {
                break; // EOF
            }

            let input = line.trim();
            if input.is_empty() {
                continue;
            }

            debug!("Received: {}", input);

            match serde_json::from_str::<JsonRpcRequest>(input) {
                Ok(req) => {
                    let id = req.id.clone();
                    let resp = self.handle_request(req).await;

                    if let Some(req_id) = id {
                        let json_resp = match resp {
                            Ok(result) => JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: Some(req_id),
                                result: Some(result),
                                error: None,
                            },
                            Err(e) => {
                                let (code, message, data) = if let Some(pve_err) =
                                    e.downcast_ref::<crate::proxmox::ProxmoxError>()
                                {
                                    match pve_err {
                                        crate::proxmox::error::ProxmoxError::Auth(_) => {
                                            (-32001, pve_err.to_string(), None)
                                        }
                                        crate::proxmox::error::ProxmoxError::NotFound(_) => {
                                            (-32004, pve_err.to_string(), None)
                                        }
                                        crate::proxmox::error::ProxmoxError::Timeout(_) => {
                                            (-32002, pve_err.to_string(), None)
                                        }
                                        crate::proxmox::error::ProxmoxError::Api(status, msg) => {
                                            let code = match status.as_u16() {
                                                401 | 403 => -32001,
                                                404 => -32004,
                                                _ => -32603,
                                            };
                                            (
                                                code,
                                                format!("API Error {}: {}", status, msg),
                                                Some(
                                                    json!({ "status": status.as_u16(), "details": msg }),
                                                ),
                                            )
                                        }
                                        _ => (-32603, pve_err.to_string(), None),
                                    }
                                } else {
                                    (-32603, e.to_string(), None)
                                };

                                JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: Some(req_id),
                                    result: None,
                                    error: Some(JsonRpcError {
                                        code,
                                        message,
                                        data,
                                    }),
                                }
                            }
                        };

                        let out = serde_json::to_string(&json_resp)?;
                        println!("{}", out);
                        io::stdout().flush()?;

                        // Check for notification (e.g. tool list changed)
                        if self.check_notification() {
                            let notification = json!({
                                "jsonrpc": "2.0",
                                "method": "notifications/tools/list_changed"
                            });
                            let out = serde_json::to_string(&notification)?;
                            println!("{}", out);
                            io::stdout().flush()?;
                        }
                    } else {
                        // Notification, no response expected
                        if let Err(e) = resp {
                            error!("Error handling notification: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse JSON-RPC: {}", e);
                    // Technically should send parse error if ID is known, but usually can't recover ID.
                }
            }
        }
        Ok(())
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> Result<Value> {
        match req.method.as_str() {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "proxmox-mcp-rs",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {
                        "listChanged": true
                    },
                    "resources": {}
                }
            })),
            "notifications/initialized" => {
                info!("Client initialized");
                Ok(Value::Null)
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({
                "tools": self.get_tool_definitions()
            })),
            "tools/call" => {
                if let Some(params) = req.params {
                    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let args = params.get("arguments").unwrap_or(&Value::Null);
                    self.call_tool(name, args).await
                } else {
                    anyhow::bail!("Missing params for tools/call");
                }
            }
            "resources/list" => Ok(json!({
                "resources": self.get_resource_definitions()
            })),
            "resources/read" => {
                if let Some(params) = req.params {
                    let uri = params.get("uri").and_then(|n| n.as_str()).unwrap_or("");
                    self.handle_resource_read(uri).await
                } else {
                    anyhow::bail!("Missing params for resources/read");
                }
            }
            _ => {
                // Ignore unknown methods or return error?
                // For MCP, unknown methods should probably be ignored if they are notifications,
                // or error if request.
                anyhow::bail!("Method not found: {}", req.method);
            }
        }
    }

    fn get_resource_definitions(&self) -> Vec<Value> {
        vec![
            json!({
                "uri": "proxmox://vms",
                "name": "List of VMs",
                "description": "A live list of all VMs and Containers",
                "mimeType": "application/json"
            }),
            // Add more resources here, e.g., templates for nodes
            // json!({ "uri": "proxmox://node/{node}/syslog", ... }) - Dynamic resources are harder to list statically
        ]
    }

    fn get_tool_definitions(&self) -> Vec<Value> {
        {
            let state = self.state.lock().unwrap();
            if state.lazy_mode && !state.tools_loaded {
                return vec![
                    json!({
                        "name": "load_all_tools",
                        "description": "Load all Proxmox tools (VMs, containers, storage, etc.). Use this to access full functionality.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    }),
                    json!({
                        "name": "get_cluster_status",
                        "description": "Get cluster status",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    }),
                    json!({
                        "name": "list_nodes",
                        "description": "List all nodes in the Proxmox cluster",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    }),
                ];
            }
        }

        let mut tools = Vec::new();
        tools.extend(self.tool_defs_cluster());
        tools.extend(self.tool_defs_vm_lifecycle());
        tools.extend(self.tool_defs_vm_config());
        tools.extend(self.tool_defs_storage());
        tools.extend(self.tool_defs_network());
        tools.extend(self.tool_defs_firewall_aliases());
        tools.extend(self.tool_defs_firewall_security_groups());
        tools.extend(self.tool_defs_system());
        tools.extend(self.tool_defs_apt());
        tools.extend(self.tool_defs_certificates());
        tools.extend(self.tool_defs_access());
        tools.extend(self.tool_defs_ha());
        tools.extend(self.tool_defs_sdn());
        tools.extend(self.tool_defs_ceph());
        tools.extend(self.tool_defs_backup_schedule());
        tools.extend(self.tool_defs_mapping());
        tools.extend(self.tool_defs_metric_server());
        tools.extend(self.tool_defs_misc());
        tools
    }

    async fn handle_resource_read(&self, uri: &str) -> Result<Value> {
        match uri {
            "proxmox://vms" => {
                let client = self
                    .clients
                    .get(&self.default_id)
                    .expect("Default client missing");
                let vms = client.get_all_vms().await?;
                let content = serde_json::to_string(&vms)?;
                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": content
                    }]
                }))
            }
            _ => anyhow::bail!("Resource not found: {}", uri),
        }
    }

    pub async fn call_tool(&self, name: &str, args: &Value) -> Result<Value> {
        match name {
            "load_all_tools" => {
                let mut state = self.state.lock().unwrap();
                state.tools_loaded = true;
                state.should_notify = true;
                Ok(json!({ "content": [{ "type": "text", "text": "All tools loaded." }] }))
            }
            "list_nodes" => {
                let client = self.get_client(args)?;
                let nodes = client.get_nodes().await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string(&nodes)? }] }),
                )
            }
            "list_vms" => {
                let client = self.get_client(args)?;
                let vms = client.get_all_vms().await?;
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&vms)? }] }))
            }
            "list_containers" => {
                let client = self.get_client(args)?;
                let vms = client.get_all_vms().await?;
                let containers: Vec<_> = vms
                    .into_iter()
                    .filter(|vm| vm.vm_type.as_deref() == Some("lxc"))
                    .collect();
                Ok(
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string(&containers)? }] }),
                )
            }
            "start_vm" => self.handle_vm_action(args, "start", None).await,
            "bulk_vm_action" => self.handle_bulk_vm_action(args).await,
            "start_container" => self.handle_vm_action(args, "start", Some("lxc")).await,
            "stop_vm" => self.handle_vm_action(args, "stop", None).await,
            "stop_container" => self.handle_vm_action(args, "stop", Some("lxc")).await,
            "shutdown_vm" => self.handle_vm_action(args, "shutdown", None).await,
            "shutdown_container" => self.handle_vm_action(args, "shutdown", Some("lxc")).await,
            "reboot_vm" => self.handle_vm_action(args, "reboot", None).await,
            "template_vm" => self.handle_template_vm(args).await,
            "create_vm" => self.handle_create(args, "qemu").await,
            "create_container" => self.handle_create(args, "lxc").await,
            "delete_vm" => self.handle_delete(args, "qemu").await,
            "delete_container" => self.handle_delete(args, "lxc").await,
            "reset_vm" => self.handle_reset(args, "qemu").await,
            "reset_container" => self.handle_reset(args, "lxc").await,
            "scan_storage_remote" => self.handle_scan_storage_remote(args).await,
            "list_templates" => {
                let node = args
                    .get("node")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing node"))?;
                let storage = args
                    .get("storage")
                    .and_then(|v| v.as_str())
                    .unwrap_or("local");
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .or(Some("vztmpl"));

                let client = self.get_client(args)?;
                let templates = client.get_storage_content(node, storage, content).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string(&templates)? }] }),
                )
            }
            "update_vm_resources" => self.handle_update_resources(args, "qemu").await,
            "update_container_resources" => self.handle_update_resources(args, "lxc").await,
            "list_snapshots" => self.handle_snapshot_list(args).await,
            "snapshot_vm" => self.handle_snapshot_create(args).await,
            "rollback_vm" => self.handle_snapshot_rollback(args).await,
            "delete_snapshot" => self.handle_snapshot_delete(args).await,
            "clone_vm" => self.handle_clone(args).await,
            "migrate_vm" => self.handle_migrate(args).await,
            "list_backups" => self.handle_list_backups(args).await,
            "create_backup" => self.handle_create_backup(args).await,
            "restore_backup" => self.handle_restore_backup(args).await,
            "get_task_status" => self.handle_get_task_status(args).await,
            "list_tasks" => self.handle_list_tasks(args).await,
            "wait_for_task" => self.handle_wait_for_task(args).await,
            "list_networks" => self.handle_list_networks(args).await,
            "create_network_bridge" => self.handle_create_network_bridge(args).await,
            "create_network_bond" => self.handle_create_network_bond(args).await,
            "update_network_interface" => self.handle_update_network_interface(args).await,
            "delete_network_interface" => self.handle_delete_network_interface(args).await,
            "apply_network_config" => self.handle_apply_network_config(args).await,
            "revert_network_config" => self.handle_revert_network_config(args).await,
            "list_storage" => self.handle_list_storage(args).await,
            "list_isos" => self.handle_list_isos(args).await,
            "get_cluster_status" => self.handle_get_cluster_status(args).await,
            "get_cluster_log" => self.handle_get_cluster_log(args).await,
            "list_firewall_rules" => self.handle_list_firewall_rules(args).await,
            "add_firewall_rule" => self.handle_add_firewall_rule(args).await,
            "delete_firewall_rule" => self.handle_delete_firewall_rule(args).await,
            "list_firewall_aliases" => self.handle_list_firewall_aliases(args).await,
            "create_firewall_alias" => self.handle_create_firewall_alias(args).await,
            "update_firewall_alias" => self.handle_update_firewall_alias(args).await,
            "delete_firewall_alias" => self.handle_delete_firewall_alias(args).await,
            "list_security_groups" => self.handle_list_security_groups(args).await,
            "create_security_group" => self.handle_create_security_group(args).await,
            "delete_security_group" => self.handle_delete_security_group(args).await,
            "list_security_group_rules" => self.handle_list_security_group_rules(args).await,
            "add_security_group_rule" => self.handle_add_security_group_rule(args).await,
            "add_disk" => self.handle_add_disk(args).await,
            "remove_disk" => self.handle_remove_disk(args).await,
            "add_network" => self.handle_add_network(args).await,
            "remove_network" => self.handle_remove_network(args).await,
            "get_node_stats" => self.handle_get_node_stats(args).await,
            "get_vm_stats" => self.handle_get_vm_stats(args).await,
            "read_task_log" => self.handle_read_task_log(args).await,
            "get_vm_config" => self.handle_get_vm_config(args).await,
            "download_url" => self.handle_download_url(args).await,
            "delete_storage_content" => self.handle_delete_storage_content(args).await,
            "get_storage_volume" => self.handle_get_storage_volume(args).await,
            "list_users" => self.handle_list_users(args).await,
            "create_user" => self.handle_create_user(args).await,
            "delete_user" => self.handle_delete_user(args).await,
            "list_cluster_storage" => self.handle_list_cluster_storage(args).await,
            "add_storage" => self.handle_add_storage(args).await,
            "delete_storage" => self.handle_delete_storage(args).await,
            "update_storage" => self.handle_update_storage(args).await,
            "get_console_url" => {
                let node = args
                    .get("node")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing node"))?;
                let vmid = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
                let console_type = args.get("console").and_then(|v| v.as_str());

                let client = self.get_client(args)?;
                let url = client.get_console_url(node, vmid, vm_type, console_type)?;
                Ok(json!({ "content": [{ "type": "text", "text": url }] }))
            }
            "vm_agent_ping" => self.handle_vm_agent_ping(args).await,
            "vm_exec" => self.handle_vm_exec(args).await,
            "vm_exec_status" => self.handle_vm_exec_status(args).await,
            "vm_read_file" => self.handle_vm_read_file(args).await,
            "vm_write_file" => self.handle_vm_write_file(args).await,
            "list_pools" => self.handle_list_pools(args).await,
            "create_pool" => self.handle_create_pool(args).await,
            "get_pool_details" => self.handle_get_pool_details(args).await,
            "update_pool" => self.handle_update_pool(args).await,
            "delete_pool" => self.handle_delete_pool(args).await,
            "list_replication_jobs" => self.handle_list_replication_jobs(args).await,
            "create_replication_job" => self.handle_create_replication_job(args).await,
            "update_replication_job" => self.handle_update_replication_job(args).await,
            "delete_replication_job" => self.handle_delete_replication_job(args).await,
            "list_ha_resources" => self.handle_list_ha_resources(args).await,
            "list_ha_groups" => self.handle_list_ha_groups(args).await,
            "add_ha_resource" => self.handle_add_ha_resource(args).await,
            "update_ha_resource" => self.handle_update_ha_resource(args).await,
            "remove_ha_resource" => self.handle_remove_ha_resource(args).await,
            "list_roles" => self.handle_list_roles(args).await,
            "create_role" => self.handle_create_role(args).await,
            "update_role" => self.handle_update_role(args).await,
            "delete_role" => self.handle_delete_role(args).await,
            "list_acls" => self.handle_list_acls(args).await,
            "update_acl" => self.handle_update_acl(args).await,
            "list_apt_updates" => self.handle_list_apt_updates(args).await,
            "run_apt_update" => self.handle_run_apt_update(args).await,
            "get_apt_versions" => self.handle_get_apt_versions(args).await,
            "list_repositories" => self.handle_list_repositories(args).await,
            "add_repository" => self.handle_add_repository(args).await,
            "update_repository_state" => self.handle_update_repository_state(args).await,
            "list_services" => self.handle_list_services(args).await,
            "manage_service" => self.handle_manage_service(args).await,
            "list_certificates" => self.handle_list_certificates(args).await,
            "upload_certificate" => self.handle_upload_certificate(args).await,
            "generate_acme_certificate" => self.handle_generate_acme_certificate(args).await,
            "set_vm_cloudinit" => self.handle_set_vm_cloudinit(args).await,
            "add_tag" => self.handle_add_tag(args).await,
            "remove_tag" => self.handle_remove_tag(args).await,
            "set_tags" => self.handle_set_tags(args).await,
            "get_subscription_info" => self.handle_get_subscription_info(args).await,
            "set_subscription_key" => self.handle_set_subscription_key(args).await,
            "check_subscription" => self.handle_check_subscription(args).await,
            "create_cluster" => self.handle_create_cluster(args).await,
            "get_cluster_join_info" => self.handle_get_cluster_join_info(args).await,
            "join_cluster" => self.handle_join_cluster(args).await,
            "list_pci_devices" => self.handle_list_pci_devices(args).await,
            "list_usb_devices" => self.handle_list_usb_devices(args).await,
            "add_pci_device" => self.handle_add_pci_device(args).await,
            "add_usb_device" => self.handle_add_usb_device(args).await,
            "remove_vm_device" => self.handle_remove_vm_device(args).await,
            "add_lxc_mountpoint" => self.handle_add_lxc_mountpoint(args).await,
            "add_lxc_bind_mount" => self.handle_add_lxc_bind_mount(args).await,
            "remove_lxc_mountpoint" => self.handle_remove_lxc_mountpoint(args).await,
            "list_pci_mappings" => self.handle_list_pci_mappings(args).await,
            "create_pci_mapping" => self.handle_create_pci_mapping(args).await,
            "update_pci_mapping" => self.handle_update_pci_mapping(args).await,
            "delete_pci_mapping" => self.handle_delete_pci_mapping(args).await,
            "list_usb_mappings" => self.handle_list_usb_mappings(args).await,
            "create_usb_mapping" => self.handle_create_usb_mapping(args).await,
            "update_usb_mapping" => self.handle_update_usb_mapping(args).await,
            "delete_usb_mapping" => self.handle_delete_usb_mapping(args).await,
            "list_metric_servers" => self.handle_list_metric_servers(args).await,
            "create_metric_server" => self.handle_create_metric_server(args).await,
            "update_metric_server" => self.handle_update_metric_server(args).await,
            "delete_metric_server" => self.handle_delete_metric_server(args).await,
            "list_sdn_zones" => self.handle_list_sdn_zones(args).await,
            "create_sdn_zone" => self.handle_create_sdn_zone(args).await,
            "delete_sdn_zone" => self.handle_delete_sdn_zone(args).await,
            "list_sdn_vnets" => self.handle_list_sdn_vnets(args).await,
            "create_sdn_vnet" => self.handle_create_sdn_vnet(args).await,
            "delete_sdn_vnet" => self.handle_delete_sdn_vnet(args).await,
            "apply_sdn_changes" => self.handle_apply_sdn_changes(args).await,
            "get_ceph_status" => self.handle_get_ceph_status(args).await,
            "list_ceph_pools" => self.handle_list_ceph_pools(args).await,
            "create_ceph_pool" => self.handle_create_ceph_pool(args).await,
            "delete_ceph_pool" => self.handle_delete_ceph_pool(args).await,
            "list_ceph_osds" => self.handle_list_ceph_osds(args).await,
            "list_ceph_monitors" => self.handle_list_ceph_monitors(args).await,
            "list_backup_schedules" => self.handle_list_backup_schedules(args).await,
            "create_backup_schedule" => self.handle_create_backup_schedule(args).await,
            "update_backup_schedule" => self.handle_update_backup_schedule(args).await,
            "delete_backup_schedule" => self.handle_delete_backup_schedule(args).await,
            _ => anyhow::bail!("Unknown tool: {}", name),
        }
    }

    async fn handle_list_backup_schedules(&self, args: &Value) -> Result<Value> {
        let schedules = self.get_client(args)?.get_backup_schedules().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&schedules)? }] }))
    }

    async fn handle_create_backup_schedule(&self, args: &Value) -> Result<Value> {
        self.get_client(args)?.create_backup_schedule(args).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Backup schedule created" }] }))
    }

    async fn handle_update_backup_schedule(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("id");
        self.get_client(args)?
            .update_backup_schedule(id, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Backup schedule {} updated", id) }] }),
        )
    }

    async fn handle_delete_backup_schedule(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        self.get_client(args)?.delete_backup_schedule(id).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Backup schedule {} deleted", id) }] }),
        )
    }

    async fn handle_get_ceph_status(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let status = self.get_client(args)?.get_ceph_status(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&status)? }] }))
    }

    async fn handle_list_ceph_pools(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let pools = self.get_client(args)?.get_ceph_pools(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&pools)? }] }))
    }

    async fn handle_create_ceph_pool(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;

        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("node");
        params.remove("name");

        let upid = self
            .get_client(args)?
            .create_ceph_pool(node, name, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Ceph pool creation initiated. UPID: {}", upid) }] }),
        )
    }

    async fn handle_delete_ceph_pool(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let remove_storages = args
            .get("remove_storages")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let upid = self
            .get_client(args)?
            .delete_ceph_pool(node, name, remove_storages)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Ceph pool deletion initiated. UPID: {}", upid) }] }),
        )
    }

    async fn handle_list_ceph_osds(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let osds = self.get_client(args)?.get_ceph_osds(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&osds)? }] }))
    }

    async fn handle_list_ceph_monitors(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let mons = self.get_client(args)?.get_ceph_monitors(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&mons)? }] }))
    }

    async fn handle_list_metric_servers(&self, args: &Value) -> Result<Value> {
        let servers = self.get_client(args)?.get_metric_servers().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&servers)? }] }))
    }

    async fn handle_create_metric_server(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let server_type = args
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing type"))?;

        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("id");
        params.remove("type");

        self.get_client(args)?
            .create_metric_server(id, server_type, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Metric server {} created", id) }] }),
        )
    }

    async fn handle_update_metric_server(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;

        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("id");

        self.get_client(args)?
            .update_metric_server(id, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Metric server {} updated", id) }] }),
        )
    }

    async fn handle_delete_metric_server(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;

        self.get_client(args)?.delete_metric_server(id).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Metric server {} deleted", id) }] }),
        )
    }

    async fn handle_list_sdn_zones(&self, args: &Value) -> Result<Value> {
        let zones = self.get_client(args)?.get_sdn_zones().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&zones)? }] }))
    }

    async fn handle_list_pci_mappings(&self, args: &Value) -> Result<Value> {
        let mappings = self.get_client(args)?.get_pci_mappings().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&mappings)? }] }))
    }

    async fn handle_create_pci_mapping(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("id");
        self.get_client(args)?
            .create_pci_mapping(id, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("PCI Mapping {} created", id) }] }),
        )
    }

    async fn handle_update_pci_mapping(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("id");
        self.get_client(args)?
            .update_pci_mapping(id, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("PCI Mapping {} updated", id) }] }),
        )
    }

    async fn handle_delete_pci_mapping(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        self.get_client(args)?.delete_pci_mapping(id).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("PCI Mapping {} deleted", id) }] }),
        )
    }

    async fn handle_list_usb_mappings(&self, args: &Value) -> Result<Value> {
        let mappings = self.get_client(args)?.get_usb_mappings().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&mappings)? }] }))
    }

    async fn handle_create_usb_mapping(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("id");
        self.get_client(args)?
            .create_usb_mapping(id, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("USB Mapping {} created", id) }] }),
        )
    }

    async fn handle_update_usb_mapping(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("id");
        self.get_client(args)?
            .update_usb_mapping(id, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("USB Mapping {} updated", id) }] }),
        )
    }

    async fn handle_delete_usb_mapping(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        self.get_client(args)?.delete_usb_mapping(id).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("USB Mapping {} deleted", id) }] }),
        )
    }

    async fn handle_create_sdn_zone(&self, args: &Value) -> Result<Value> {
        let zone = args
            .get("zone")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing zone"))?;
        let zone_type = args
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing type"))?;

        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("zone");
        params.remove("type");

        self.get_client(args)?
            .create_sdn_zone(zone, zone_type, &Value::Object(params))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("SDN Zone {} created", zone) }] }))
    }

    async fn handle_delete_sdn_zone(&self, args: &Value) -> Result<Value> {
        let zone = args
            .get("zone")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing zone"))?;
        self.get_client(args)?.delete_sdn_zone(zone).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("SDN Zone {} deleted", zone) }] }))
    }

    async fn handle_list_sdn_vnets(&self, args: &Value) -> Result<Value> {
        let vnets = self.get_client(args)?.get_sdn_vnets().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&vnets)? }] }))
    }

    async fn handle_create_sdn_vnet(&self, args: &Value) -> Result<Value> {
        let vnet = args
            .get("vnet")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing vnet"))?;
        let zone = args
            .get("zone")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing zone"))?;

        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("vnet");
        params.remove("zone");

        self.get_client(args)?
            .create_sdn_vnet(vnet, zone, &Value::Object(params))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("SDN Vnet {} created", vnet) }] }))
    }

    async fn handle_delete_sdn_vnet(&self, args: &Value) -> Result<Value> {
        let vnet = args
            .get("vnet")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing vnet"))?;
        self.get_client(args)?.delete_sdn_vnet(vnet).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("SDN Vnet {} deleted", vnet) }] }))
    }

    async fn handle_apply_sdn_changes(&self, args: &Value) -> Result<Value> {
        let upid = self.get_client(args)?.apply_sdn().await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("SDN changes applied. UPID: {}", upid) }] }),
        )
    }

    async fn handle_add_lxc_mountpoint(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let mp_id = args
            .get("mp_id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing mp_id"))?;
        let volume = args
            .get("volume")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing volume"))?;
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing path"))?;
        let read_only = args.get("read_only").and_then(|v| v.as_bool());
        let backup = args.get("backup").and_then(|v| v.as_bool());
        let extra_options = args.get("extra_options").and_then(|v| v.as_str());

        self.get_client(args)?
            .add_lxc_mountpoint(
                node,
                vmid,
                mp_id,
                volume,
                path,
                read_only,
                backup,
                extra_options,
            )
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Mount point {} added to CT {}", mp_id, vmid) }] }),
        )
    }

    async fn handle_add_lxc_bind_mount(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let mp_id = args
            .get("mp_id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing mp_id"))?;
        let source = args
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing source"))?;
        let target = args
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing target"))?;
        let read_only = args.get("read_only").and_then(|v| v.as_bool());

        self.get_client(args)?
            .add_lxc_bind_mount(node, vmid, mp_id, source, target, read_only)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Mount point {} added to CT {}", mp_id, vmid) }] }),
        )
    }

    async fn handle_remove_lxc_mountpoint(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let mp_id = args
            .get("mp_id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing mp_id"))?;

        self.get_client(args)?
            .remove_lxc_mountpoint(node, vmid, mp_id)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Mount point {} removed from CT {}", mp_id, vmid) }] }),
        )
    }

    async fn handle_list_pci_devices(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let devices = self.get_client(args)?.get_pci_devices(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&devices)? }] }))
    }

    async fn handle_list_usb_devices(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let devices = self.get_client(args)?.get_usb_devices(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&devices)? }] }))
    }

    async fn handle_add_pci_device(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let device_id = args
            .get("device_id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing device_id"))?;
        let host = args
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing host"))?;
        let pcie = args.get("pcie").and_then(|v| v.as_bool());
        let mdev = args.get("mdev").and_then(|v| v.as_str());
        let extra_options = args.get("extra_options").and_then(|v| v.as_str());

        self.get_client(args)?
            .add_pci_device(
                node,
                vmid,
                "qemu",
                device_id,
                host,
                pcie,
                mdev,
                extra_options,
            )
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("PCI device {} added to VM {}", device_id, vmid) }] }),
        )
    }

    async fn handle_add_usb_device(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let device_id = args
            .get("device_id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing device_id"))?;
        let host = args
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing host"))?;
        let usb3 = args.get("usb3").and_then(|v| v.as_bool());
        let extra_options = args.get("extra_options").and_then(|v| v.as_str());

        self.get_client(args)?
            .add_usb_device(node, vmid, "qemu", device_id, host, usb3, extra_options)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("USB device {} added to VM {}", device_id, vmid) }] }),
        )
    }

    async fn handle_remove_vm_device(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let device_id = args
            .get("device_id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing device_id"))?;

        self.get_client(args)?
            .remove_vm_device(node, vmid, "qemu", device_id)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Device {} removed from VM {}", device_id, vmid) }] }),
        )
    }

    async fn handle_create_cluster(&self, args: &Value) -> Result<Value> {
        let clustername = args
            .get("clustername")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing clustername"))?;
        let res = self.get_client(args)?.create_cluster(clustername).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Cluster creation initiated. Result: {}", res) }] }),
        )
    }

    async fn handle_get_cluster_join_info(&self, args: &Value) -> Result<Value> {
        let info = self.get_client(args)?.get_join_info().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&info)? }] }))
    }

    async fn handle_join_cluster(&self, args: &Value) -> Result<Value> {
        let hostname = args
            .get("hostname")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hostname"))?;
        let password = args
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing password"))?;
        let fingerprint = args
            .get("fingerprint")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing fingerprint"))?;

        let res = self
            .get_client(args)?
            .join_cluster(hostname, password, fingerprint)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Cluster join initiated. Result: {}", res) }] }),
        )
    }

    async fn handle_get_subscription_info(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let info = self.get_client(args)?.get_subscription(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&info)? }] }))
    }

    async fn handle_set_subscription_key(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing key"))?;
        self.get_client(args)?.set_subscription(node, key).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Subscription key set" }] }))
    }

    async fn handle_check_subscription(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        self.get_client(args)?.update_subscription(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Subscription check initiated" }] }))
    }

    async fn handle_vm_agent_ping(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;

        self.get_client(args)?.agent_ping(node, vmid).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Pong" }] }))
    }

    async fn handle_vm_exec(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let command_str = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing command"))?;
        let input_data = args.get("input_data").and_then(|v| v.as_str());

        // Naive splitting. Ideally we'd use shell-words parsing.
        let command: Vec<String> = command_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let res = self
            .get_client(args)?
            .agent_exec(node, vmid, &command, input_data)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res)? }] }))
    }

    async fn handle_vm_exec_status(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let pid = args
            .get("pid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing pid"))?;

        let res = self
            .get_client(args)?
            .agent_exec_status(node, vmid, pid)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res)? }] }))
    }

    async fn handle_vm_read_file(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let file = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing file"))?;

        let res = self
            .get_client(args)?
            .agent_file_read(node, vmid, file)
            .await?;
        // Result usually has "content" (read bytes) or "bytes" (count).
        // content is text if possible?
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res)? }] }))
    }

    async fn handle_vm_write_file(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let file = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing file"))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing content"))?;
        let encode = args.get("encode").and_then(|v| v.as_bool());

        self.get_client(args)?
            .agent_file_write(node, vmid, file, content, encode)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "File written" }] }))
    }

    async fn handle_list_cluster_storage(&self, args: &Value) -> Result<Value> {
        let storage = self.get_client(args)?.get_cluster_storage().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&storage)? }] }))
    }

    async fn handle_add_storage(&self, args: &Value) -> Result<Value> {
        let storage = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage ID"))?;
        let storage_type = args
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage type"))?;

        let content = args.get("content").and_then(|v| v.as_str());
        let nodes = args.get("nodes").and_then(|v| {
            v.as_array().map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
        });
        let enable = args.get("enable").and_then(|v| v.as_bool());

        // Collect extra params
        let mut extra = serde_json::Map::new();
        let common_fields = [
            "path", "server", "share", "export", "username", "password", "pool", "vgname",
        ];

        for field in common_fields {
            if let Some(val) = args.get(field) {
                extra.insert(field.to_string(), val.clone());
            }
        }

        self.get_client(args)?
            .add_storage(
                storage,
                storage_type,
                content,
                nodes,
                enable,
                if extra.is_empty() { None } else { Some(&extra) },
            )
            .await?;

        Ok(json!({ "content": [{ "type": "text", "text": format!("Storage {} added", storage) }] }))
    }

    async fn handle_delete_storage(&self, args: &Value) -> Result<Value> {
        let storage = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage ID"))?;

        self.get_client(args)?.delete_storage(storage).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Storage {} deleted", storage) }] }),
        )
    }

    async fn handle_update_storage(&self, args: &Value) -> Result<Value> {
        let storage = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage ID"))?;

        let mut params = serde_json::Map::new();

        if let Some(c) = args.get("content") {
            params.insert("content".to_string(), c.clone());
        }
        if let Some(n) = args.get("nodes") {
            params.insert("nodes".to_string(), n.clone());
        }
        if let Some(e) = args.get("enable") {
            params.insert(
                "disable".to_string(),
                json!(if e.as_bool().unwrap_or(true) { 0 } else { 1 }),
            );
        }

        if params.is_empty() {
            return Ok(json!({ "content": [{ "type": "text", "text": "No changes requested" }] }));
        }

        self.get_client(args)?
            .update_storage(storage, &params)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Storage {} updated", storage) }] }),
        )
    }

    async fn handle_list_users(&self, args: &Value) -> Result<Value> {
        let users = self.get_client(args)?.get_users().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&users)? }] }))
    }

    async fn handle_create_user(&self, args: &Value) -> Result<Value> {
        let userid = args
            .get("userid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing userid"))?;
        let password = args
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing password"))?;

        let email = args.get("email").and_then(|v| v.as_str());
        let firstname = args.get("firstname").and_then(|v| v.as_str());
        let lastname = args.get("lastname").and_then(|v| v.as_str());
        let comment = args.get("comment").and_then(|v| v.as_str());
        let expire = args.get("expire").and_then(|v| v.as_i64());
        let enable = args.get("enable").and_then(|v| v.as_bool());

        let groups = args.get("groups").and_then(|v| {
            v.as_array().map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
        });

        self.get_client(args)?
            .create_user(
                userid, password, email, firstname, lastname, expire, enable, comment, groups,
            )
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("User {} created", userid) }] }))
    }

    async fn handle_delete_user(&self, args: &Value) -> Result<Value> {
        let userid = args
            .get("userid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing userid"))?;

        self.get_client(args)?.delete_user(userid).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("User {} deleted", userid) }] }))
    }

    async fn handle_download_url(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let storage = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing url"))?;
        let filename = args
            .get("filename")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing filename"))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing content"))?;

        let checksum = args.get("checksum").and_then(|v| v.as_str());
        let checksum_algorithm = args.get("checksum_algorithm").and_then(|v| v.as_str());

        let upid = self
            .get_client(args)?
            .download_url(
                node,
                storage,
                url,
                filename,
                content,
                checksum,
                checksum_algorithm,
            )
            .await?;

        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Download initiated. UPID: {}", upid) }] }),
        )
    }

    async fn handle_delete_storage_content(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let storage = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;
        let volume = args
            .get("volume")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing volume"))?;

        self.get_client(args)?
            .delete_storage_content(node, storage, volume)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Deleted volume {}", volume) }] }))
    }

    async fn handle_get_storage_volume(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let storage = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;
        let volume = args
            .get("volume")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing volume"))?;

        let info = self
            .get_client(args)?
            .get_storage_content_volume(node, storage, volume)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&info)? }] }))
    }

    async fn handle_get_node_stats(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let timeframe = args.get("timeframe").and_then(|v| v.as_str());
        let cf = args.get("cf").and_then(|v| v.as_str());

        let stats = self
            .get_client(args)?
            .get_node_stats(node, timeframe, cf)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&stats)? }] }))
    }

    async fn handle_get_vm_stats(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let timeframe = args.get("timeframe").and_then(|v| v.as_str());
        let cf = args.get("cf").and_then(|v| v.as_str());

        let stats = self
            .get_client(args)?
            .get_resource_stats(node, vmid, vm_type, timeframe, cf)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&stats)? }] }))
    }

    async fn handle_add_disk(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let device = args
            .get("device")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing device"))?;
        let storage = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;
        let size_gb = args
            .get("size_gb")
            .and_then(|v| v.as_u64())
            .ok_or(anyhow::anyhow!("Missing size_gb"))?;

        let format = args.get("format").and_then(|v| v.as_str());
        let extra_options = args.get("extra_options").and_then(|v| v.as_str());

        self.get_client(args)?
            .add_virtual_disk(
                node,
                vmid,
                vm_type,
                device,
                storage,
                size_gb,
                format,
                extra_options,
            )
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Disk {} added to {} {}", device, vm_type, vmid) }] }),
        )
    }

    async fn handle_remove_disk(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let device = args
            .get("device")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing device"))?;

        self.get_client(args)?
            .remove_virtual_disk(node, vmid, vm_type, device)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Disk {} removed from {} {}", device, vm_type, vmid) }] }),
        )
    }

    async fn handle_add_network(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let device = args
            .get("device")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing device"))?;
        let bridge = args
            .get("bridge")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing bridge"))?;

        let model = args.get("model").and_then(|v| v.as_str());
        let mac = args.get("mac").and_then(|v| v.as_str());
        let extra_options = args.get("extra_options").and_then(|v| v.as_str());

        self.get_client(args)?
            .add_network_interface(
                node,
                vmid,
                vm_type,
                device,
                model,
                bridge,
                mac,
                extra_options,
            )
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Network interface {} added to {} {}", device, vm_type, vmid) }] }),
        )
    }

    async fn handle_remove_network(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let device = args
            .get("device")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing device"))?;

        self.get_client(args)?
            .remove_network_interface(node, vmid, vm_type, device)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Network interface {} removed from {} {}", device, vm_type, vmid) }] }),
        )
    }

    async fn handle_list_firewall_rules(&self, args: &Value) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str());
        let vmid = args.get("vmid").and_then(|v| v.as_i64());

        let rules = self
            .get_client(args)?
            .get_firewall_rules(node, vmid)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&rules)? }] }))
    }

    async fn handle_add_firewall_rule(&self, args: &Value) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str());
        let vmid = args.get("vmid").and_then(|v| v.as_i64());

        // Construct params object excluding node/vmid
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("node");
        params.remove("vmid");

        self.get_client(args)?
            .add_firewall_rule(node, vmid, &Value::Object(params))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Firewall rule added" }] }))
    }

    async fn handle_delete_firewall_rule(&self, args: &Value) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str());
        let vmid = args.get("vmid").and_then(|v| v.as_i64());
        let pos = args
            .get("pos")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing rule position"))?;

        self.get_client(args)?
            .delete_firewall_rule(node, vmid, pos)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Firewall rule {} deleted", pos) }] }),
        )
    }

    async fn handle_list_firewall_aliases(&self, args: &Value) -> Result<Value> {
        let level = args
            .get("level")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing level"))?;
        let node = args.get("node").and_then(|v| v.as_str());

        let aliases = self.get_client(args)?.get_aliases(level, node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&aliases)? }] }))
    }

    async fn handle_create_firewall_alias(&self, args: &Value) -> Result<Value> {
        let level = args
            .get("level")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing level"))?;
        let node = args.get("node").and_then(|v| v.as_str());
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let cidr = args
            .get("cidr")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing cidr"))?;
        let comment = args.get("comment").and_then(|v| v.as_str());

        self.get_client(args)?
            .create_alias(level, node, name, cidr, comment)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Firewall alias created" }] }))
    }

    async fn handle_update_firewall_alias(&self, args: &Value) -> Result<Value> {
        let level = args
            .get("level")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing level"))?;
        let node = args.get("node").and_then(|v| v.as_str());
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let cidr = args
            .get("cidr")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing cidr"))?;
        let comment = args.get("comment").and_then(|v| v.as_str());

        self.get_client(args)?
            .update_alias(level, node, name, cidr, comment)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Firewall alias updated" }] }))
    }

    async fn handle_delete_firewall_alias(&self, args: &Value) -> Result<Value> {
        let level = args
            .get("level")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing level"))?;
        let node = args.get("node").and_then(|v| v.as_str());
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;

        self.get_client(args)?
            .delete_alias(level, node, name)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Firewall alias deleted" }] }))
    }

    async fn handle_list_security_groups(&self, args: &Value) -> Result<Value> {
        let groups = self.get_client(args)?.get_security_groups().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&groups)? }] }))
    }

    async fn handle_create_security_group(&self, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let comment = args.get("comment").and_then(|v| v.as_str());

        self.get_client(args)?
            .create_security_group(name, comment)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Security group '{}' created", name) }] }),
        )
    }

    async fn handle_delete_security_group(&self, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;

        self.get_client(args)?.delete_security_group(name).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Security group '{}' deleted", name) }] }),
        )
    }

    async fn handle_list_security_group_rules(&self, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;

        let rules = self
            .get_client(args)?
            .get_security_group_rules(name)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&rules)? }] }))
    }

    async fn handle_add_security_group_rule(&self, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;

        let mut rule = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        rule.remove("name");

        self.get_client(args)?
            .add_security_group_rule(name, &Value::Object(rule))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Rule added to security group '{}'", name) }] }),
        )
    }

    async fn handle_get_cluster_status(&self, args: &Value) -> Result<Value> {
        let status = self.get_client(args)?.get_cluster_status().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&status)? }] }))
    }

    async fn handle_get_cluster_log(&self, args: &Value) -> Result<Value> {
        let limit = args.get("limit").and_then(|v| v.as_u64());
        let log = self.get_client(args)?.get_cluster_log(limit).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&log)? }] }))
    }

    async fn handle_list_storage(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;

        let storage = self.get_client(args)?.get_storage_list(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&storage)? }] }))
    }

    async fn handle_list_isos(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let storage = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;

        let isos = self
            .get_client(args)?
            .get_storage_content(node, storage, Some("iso"))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&isos)? }] }))
    }

    async fn handle_list_networks(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;

        let networks = self.get_client(args)?.get_network_interfaces(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&networks)? }] }))
    }

    async fn handle_create_network_bridge(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let iface = args
            .get("iface")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing iface"))?;

        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("node");
        params.remove("iface");

        self.get_client(args)?
            .create_network_bridge(node, iface, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Bridge {} created on {}", iface, node) }] }),
        )
    }

    async fn handle_create_network_bond(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let iface = args
            .get("iface")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing iface"))?;

        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("node");
        params.remove("iface");

        self.get_client(args)?
            .create_network_bond(node, iface, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Bond {} created on {}", iface, node) }] }),
        )
    }

    async fn handle_update_network_interface(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let iface = args
            .get("iface")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing iface"))?;

        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("node");
        params.remove("iface");

        self.get_client(args)?
            .update_network_interface(node, iface, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Interface {} updated on {}", iface, node) }] }),
        )
    }

    async fn handle_delete_network_interface(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let iface = args
            .get("iface")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing iface"))?;

        self.get_client(args)?
            .delete_network_interface(node, iface)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Interface {} deleted on {}", iface, node) }] }),
        )
    }

    async fn handle_apply_network_config(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let upid = self.get_client(args)?.apply_network_config(node).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Network config applied on {}. UPID: {}", node, upid) }] }),
        )
    }

    async fn handle_revert_network_config(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        self.get_client(args)?.revert_network_config(node).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Network config reverted on {}", node) }] }),
        )
    }

    async fn handle_get_task_status(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let upid = args
            .get("upid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing upid"))?;

        let status = self.get_client(args)?.get_task_status(node, upid).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&status)? }] }))
    }

    async fn handle_list_tasks(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let limit = args.get("limit").and_then(|v| v.as_u64());

        let tasks = self.get_client(args)?.list_tasks(node, limit).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&tasks)? }] }))
    }

    async fn handle_wait_for_task(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let upid = args
            .get("upid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing upid"))?;
        let timeout = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(60);

        let status = self
            .get_client(args)?
            .wait_for_task(node, upid, timeout)
            .await?;
        let exit_status = status
            .get("exitstatus")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Task finished with status: {}\nFull details:\n{}", exit_status, serde_json::to_string(&status)?) }] }),
        )
    }

    async fn handle_list_backups(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let storage = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;
        let vmid = args.get("vmid").and_then(|v| v.as_i64());

        let backups = self
            .get_client(args)?
            .get_backups(node, storage, vmid)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&backups)? }] }))
    }

    async fn handle_create_backup(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;

        let storage = args.get("storage").and_then(|v| v.as_str());
        let mode = args.get("mode").and_then(|v| v.as_str());
        let compress = args.get("compress").and_then(|v| v.as_str());
        let remove = args.get("remove").and_then(|v| v.as_bool());

        let res = self
            .get_client(args)?
            .create_backup(node, vmid, storage, mode, compress, remove)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Backup initiated. UPID: {}", res) }] }),
        )
    }

    async fn handle_restore_backup(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let archive = args
            .get("archive")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing archive"))?;
        let vm_type = args
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing type"))?;

        let storage = args.get("storage").and_then(|v| v.as_str());
        let force = args.get("force").and_then(|v| v.as_bool());

        let res = self
            .get_client(args)?
            .restore_backup(node, vmid, vm_type, archive, storage, force)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Restore initiated. UPID: {}", res) }] }),
        )
    }

    async fn handle_clone(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let newid = args
            .get("newid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing newid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");

        let name = args.get("name").and_then(|v| v.as_str());
        let target = args.get("target").and_then(|v| v.as_str());
        let full = args.get("full").and_then(|v| v.as_bool());

        let res = self
            .get_client(args)?
            .clone_resource(node, vmid, vm_type, newid, name, target, full)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Clone initiated. UPID: {}", res) }] }),
        )
    }

    async fn handle_migrate(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let target_node = args
            .get("target_node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing target_node"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let online = args
            .get("online")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let res = self
            .get_client(args)?
            .migrate_resource(node, vmid, vm_type, target_node, online)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Migration initiated. UPID: {}", res) }] }),
        )
    }

    async fn handle_snapshot_list(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");

        let snapshots = self
            .get_client(args)?
            .get_snapshots(node, vmid, vm_type)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&snapshots)? }] }))
    }

    async fn handle_snapshot_create(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let snapname = args
            .get("snapname")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing snapname"))?;
        let desc = args.get("description").and_then(|v| v.as_str());
        let vmstate = args
            .get("vmstate")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let res = self
            .get_client(args)?
            .create_snapshot(node, vmid, vm_type, snapname, desc, vmstate)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Snapshot '{}' created. UPID: {}", snapname, res) }] }),
        )
    }

    async fn handle_snapshot_rollback(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let snapname = args
            .get("snapname")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing snapname"))?;

        let res = self
            .get_client(args)?
            .rollback_snapshot(node, vmid, vm_type, snapname)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Rollback to '{}' initiated. UPID: {}", snapname, res) }] }),
        )
    }

    async fn handle_snapshot_delete(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let snapname = args
            .get("snapname")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing snapname"))?;

        let res = self
            .get_client(args)?
            .delete_snapshot(node, vmid, vm_type, snapname)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Delete snapshot '{}' initiated. UPID: {}", snapname, res) }] }),
        )
    }

    async fn handle_update_resources(&self, args: &Value, resource_type: &str) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;

        let mut output = Vec::new();

        // Handle Disk Resize
        if let Some(gb) = args.get("disk_gb").and_then(|v| v.as_i64()) {
            let disk = args
                .get("disk")
                .and_then(|v| v.as_str())
                .unwrap_or("rootfs");
            let size = format!("+{}G", gb);
            let upid = self
                .get_client(args)?
                .resize_disk(node, vmid, resource_type, disk, &size)
                .await?;
            output.push(format!(
                "Disk {} resize initiated (+{}GB). UPID: {}",
                disk, gb, upid
            ));
        }

        // Handle Config Update
        let mut config_params = serde_json::Map::new();
        if let Some(c) = args.get("cores") {
            config_params.insert("cores".to_string(), c.clone());
        }
        if let Some(m) = args.get("memory") {
            config_params.insert("memory".to_string(), m.clone());
        }
        if let Some(s) = args.get("sockets") {
            config_params.insert("sockets".to_string(), s.clone());
        }
        if let Some(s) = args.get("swap") {
            config_params.insert("swap".to_string(), s.clone());
        }

        if !config_params.is_empty() {
            self.get_client(args)?
                .update_config(node, vmid, resource_type, &Value::Object(config_params))
                .await?;
            output.push("Resource config updated.".to_string());
        }

        if output.is_empty() {
            output.push("No changes requested.".to_string());
        }

        Ok(json!({ "content": [{ "type": "text", "text": output.join("\n") }] }))
    }

    async fn handle_reset(&self, args: &Value, expected_type: &str) -> Result<Value> {
        let id_key = if expected_type == "qemu" {
            "vm_id"
        } else {
            "container_id"
        };
        let id_str = args
            .get(id_key)
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing {}", id_key))?;
        let vmid: i64 = id_str.parse()?;

        info!("Resetting {} {}...", expected_type, vmid);

        let (node, vm_type) = self.get_client(args)?.find_vm_location(vmid).await?;

        if vm_type != expected_type {
            anyhow::bail!("ID {} is not a {}", vmid, expected_type);
        }

        let action = if expected_type == "qemu" {
            "reset"
        } else {
            "reboot"
        };

        let res = self
            .get_client(args)?
            .vm_action(&node, vmid, action, Some(expected_type))
            .await?;

        info!(
            "Reset initiated for {} {}. UPID: {}",
            expected_type, vmid, res
        );
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Reset initiated. UPID: {}", res) }] }),
        )
    }

    async fn handle_create(&self, args: &Value, resource_type: &str) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;

        // Filter out "node" from args to send as params
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("node");

        let res = self
            .get_client(args)?
            .create_resource(node, resource_type, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Create {} initiated. UPID: {}", resource_type, res) }] }),
        )
    }

    async fn handle_delete(&self, args: &Value, resource_type: &str) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;

        let res = self
            .get_client(args)?
            .delete_resource(node, vmid, resource_type)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Delete {} initiated. UPID: {}", resource_type, res) }] }),
        )
    }

    async fn handle_bulk_vm_action(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmids = args
            .get("vmids")
            .and_then(|v| {
                v.as_array()
                    .map(|a| a.iter().filter_map(|x| x.as_i64()).collect::<Vec<i64>>())
            })
            .ok_or(anyhow::anyhow!("Missing or invalid vmids"))?;
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str());

        let results = self
            .get_client(args)?
            .bulk_vm_action(node, vmids, action, vm_type)
            .await?;

        // Transform the results into a friendly format for the user
        let mut report = Vec::new();
        for (vmid, res) in results {
            match res {
                Ok(upid) => report.push(format!("VM {}: Success (UPID: {})", vmid, upid)),
                Err(e) => report.push(format!("VM {}: Failed ({})", vmid, e)),
            }
        }

        Ok(json!({ "content": [{ "type": "text", "text": report.join("\n") }] }))
    }

    async fn handle_vm_action(
        &self,
        args: &Value,
        action: &str,
        forced_type: Option<&str>,
    ) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;

        let vm_type = if let Some(t) = forced_type {
            Some(t)
        } else {
            args.get("type").and_then(|v| v.as_str())
        };

        let res = self
            .get_client(args)?
            .vm_action(node, vmid, action, vm_type)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Action '{}' initiated. UPID: {}", action, res) }] }),
        )
    }

    async fn handle_scan_storage_remote(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let storage_type = args
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing type"))?;
        let server = args
            .get("server")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing server"))?;
        let user = args.get("username").and_then(|v| v.as_str());
        let password = args.get("password").and_then(|v| v.as_str());

        let result = self
            .get_client(args)?
            .scan_storage(node, storage_type, server, user, password)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&result)? }] }))
    }

    async fn handle_template_vm(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;

        let res = self.get_client(args)?.template_vm(node, vmid).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Template created. UPID: {}", res) }] }),
        )
    }

    async fn handle_read_task_log(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let upid = args
            .get("upid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing upid"))?;

        let log_entries = self.get_client(args)?.get_task_log(node, upid).await?;
        let mut log_text = String::new();
        for entry in log_entries {
            if let Some(line) = entry.get("t").and_then(|v| v.as_str()) {
                log_text.push_str(line);
                log_text.push('\n');
            }
        }

        Ok(json!({ "content": [{ "type": "text", "text": log_text }] }))
    }

    async fn handle_get_vm_config(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");

        let config = self
            .get_client(args)?
            .get_vm_config(node, vmid, vm_type)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&config)? }] }))
    }

    async fn handle_list_pools(&self, args: &Value) -> Result<Value> {
        let pools = self.get_client(args)?.get_pools().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&pools)? }] }))
    }

    async fn handle_create_pool(&self, args: &Value) -> Result<Value> {
        let poolid = args
            .get("poolid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing poolid"))?;
        let comment = args.get("comment").and_then(|v| v.as_str());
        self.get_client(args)?.create_pool(poolid, comment).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Pool {} created", poolid) }] }))
    }

    async fn handle_get_pool_details(&self, args: &Value) -> Result<Value> {
        let poolid = args
            .get("poolid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing poolid"))?;
        let details = self.get_client(args)?.get_pool_details(poolid).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&details)? }] }))
    }

    async fn handle_update_pool(&self, args: &Value) -> Result<Value> {
        let poolid = args
            .get("poolid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing poolid"))?;
        // Construct params excluding poolid
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("poolid");
        self.get_client(args)?
            .update_pool(poolid, &Value::Object(params))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Pool {} updated", poolid) }] }))
    }

    async fn handle_delete_pool(&self, args: &Value) -> Result<Value> {
        let poolid = args
            .get("poolid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing poolid"))?;
        self.get_client(args)?.delete_pool(poolid).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Pool {} deleted", poolid) }] }))
    }

    async fn handle_list_replication_jobs(&self, args: &Value) -> Result<Value> {
        let jobs = self.get_client(args)?.get_replication_jobs().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&jobs)? }] }))
    }

    async fn handle_create_replication_job(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let target = args
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing target"))?;
        let schedule = args.get("schedule").and_then(|v| v.as_str());
        let rate = args.get("rate").and_then(|v| v.as_f64());
        let comment = args.get("comment").and_then(|v| v.as_str());
        let enable = args.get("enable").and_then(|v| v.as_bool());

        self.get_client(args)?
            .create_replication_job(id, target, schedule, rate, comment, enable)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Replication job {} created", id) }] }),
        )
    }

    async fn handle_update_replication_job(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("id");
        self.get_client(args)?
            .update_replication_job(id, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Replication job {} updated", id) }] }),
        )
    }

    async fn handle_delete_replication_job(&self, args: &Value) -> Result<Value> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        self.get_client(args)?.delete_replication_job(id).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Replication job {} deleted", id) }] }),
        )
    }

    async fn handle_list_ha_resources(&self, args: &Value) -> Result<Value> {
        let resources = self.get_client(args)?.get_ha_resources().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&resources)? }] }))
    }

    async fn handle_list_ha_groups(&self, args: &Value) -> Result<Value> {
        let groups = self.get_client(args)?.get_ha_groups().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&groups)? }] }))
    }

    async fn handle_add_ha_resource(&self, args: &Value) -> Result<Value> {
        let sid = args
            .get("sid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing sid"))?;
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("sid");
        self.get_client(args)?
            .add_ha_resource(sid, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Resource {} added to HA", sid) }] }),
        )
    }

    async fn handle_update_ha_resource(&self, args: &Value) -> Result<Value> {
        let sid = args
            .get("sid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing sid"))?;
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("sid");
        self.get_client(args)?
            .update_ha_resource(sid, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("HA resource {} updated", sid) }] }),
        )
    }

    async fn handle_remove_ha_resource(&self, args: &Value) -> Result<Value> {
        let sid = args
            .get("sid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing sid"))?;
        self.get_client(args)?.delete_ha_resource(sid).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Resource {} removed from HA", sid) }] }),
        )
    }

    async fn handle_list_roles(&self, args: &Value) -> Result<Value> {
        let roles = self.get_client(args)?.get_roles().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&roles)? }] }))
    }

    async fn handle_create_role(&self, args: &Value) -> Result<Value> {
        let roleid = args
            .get("roleid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing roleid"))?;
        let privs = args
            .get("privs")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing privs"))?;
        self.get_client(args)?.create_role(roleid, privs).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Role {} created", roleid) }] }))
    }

    async fn handle_update_role(&self, args: &Value) -> Result<Value> {
        let roleid = args
            .get("roleid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing roleid"))?;
        let privs = args
            .get("privs")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing privs"))?;
        let append = args
            .get("append")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.get_client(args)?
            .update_role(roleid, privs, append)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Role {} updated", roleid) }] }))
    }

    async fn handle_delete_role(&self, args: &Value) -> Result<Value> {
        let roleid = args
            .get("roleid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing roleid"))?;
        self.get_client(args)?.delete_role(roleid).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Role {} deleted", roleid) }] }))
    }

    async fn handle_list_acls(&self, args: &Value) -> Result<Value> {
        let acls = self.get_client(args)?.get_acls().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&acls)? }] }))
    }

    async fn handle_update_acl(&self, args: &Value) -> Result<Value> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing path"))?;
        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("path");
        self.get_client(args)?
            .update_acl(path, &Value::Object(params))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("ACL for path {} updated", path) }] }),
        )
    }

    async fn handle_list_apt_updates(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let updates = self.get_client(args)?.get_apt_updates(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&updates)? }] }))
    }

    async fn handle_run_apt_update(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let upid = self.get_client(args)?.run_apt_update(node).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("APT update initiated. UPID: {}", upid) }] }),
        )
    }

    async fn handle_get_apt_versions(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let versions = self.get_client(args)?.get_apt_versions(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&versions)? }] }))
    }

    async fn handle_list_repositories(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let repos = self.get_client(args)?.get_repositories(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&repos)? }] }))
    }

    async fn handle_add_repository(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let handle = args
            .get("handle")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing handle"))?;
        self.get_client(args)?.add_repository(node, handle).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Repository {} added", handle) }] }),
        )
    }

    async fn handle_update_repository_state(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing path"))?;
        let index = args
            .get("index")
            .and_then(|v| v.as_u64())
            .ok_or(anyhow::anyhow!("Missing index"))? as usize;
        let enabled = args
            .get("enabled")
            .and_then(|v| v.as_bool())
            .ok_or(anyhow::anyhow!("Missing enabled"))?;

        self.get_client(args)?
            .change_repository_state(node, path, index, enabled)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Repository state updated" }] }))
    }

    async fn handle_list_services(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let services = self.get_client(args)?.get_services(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&services)? }] }))
    }

    async fn handle_manage_service(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let service = args
            .get("service")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing service"))?;
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;

        let upid = self
            .get_client(args)?
            .manage_service(node, service, action)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Service {} {} initiated. UPID: {}", service, action, upid) }] }),
        )
    }

    async fn handle_list_certificates(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let certs = self.get_client(args)?.get_certificates(node).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&certs)? }] }))
    }

    async fn handle_upload_certificate(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let certificates = args
            .get("certificates")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing certificates"))?;
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing key"))?;
        let force = args.get("force").and_then(|v| v.as_bool());
        let restart = args.get("restart").and_then(|v| v.as_bool());

        self.get_client(args)?
            .upload_certificate(node, certificates, key, force, restart)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Certificate uploaded successfully" }] }))
    }

    async fn handle_generate_acme_certificate(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let upid = self.get_client(args)?.renew_acme_certificate(node).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("ACME certificate renewal initiated. UPID: {}", upid) }] }),
        )
    }

    async fn handle_set_vm_cloudinit(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;

        let mut params = args
            .as_object()
            .ok_or(anyhow::anyhow!("Args must be object"))?
            .clone();
        params.remove("node");
        params.remove("vmid");

        self.get_client(args)?
            .set_vm_cloudinit(node, vmid, &Value::Object(params))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Cloud-Init config updated" }] }))
    }

    async fn handle_add_tag(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let tags = args
            .get("tags")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing tags"))?;

        self.get_client(args)?
            .add_tag(node, vmid, vm_type, tags)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Tags added" }] }))
    }

    async fn handle_remove_tag(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let tags = args
            .get("tags")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing tags"))?;

        self.get_client(args)?
            .remove_tag(node, vmid, vm_type, tags)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Tags removed" }] }))
    }

    async fn handle_set_tags(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let tags = args
            .get("tags")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing tags"))?;

        self.get_client(args)?
            .set_tags(node, vmid, vm_type, tags)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Tags set" }] }))
    }

    fn tool_defs_cluster(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_nodes",
                "description": "List cluster nodes",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "get_cluster_status",
                "description": "Get cluster status",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "get_cluster_log",
                "description": "Read cluster log",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Max lines" }
                    },
                    "required": []
                }
            }),
            json!({
                "name": "get_node_stats",
                "description": "Get node RRD stats",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "timeframe": { "type": "string", "enum": ["hour", "day", "week", "month", "year"], "description": "Timeframe" },
                        "cf": { "type": "string", "enum": ["AVERAGE", "MAX"], "description": "Consolidation function" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "create_cluster",
                "description": "Create new cluster",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "clustername": { "type": "string" }
                    },
                    "required": ["clustername"]
                }
            }),
            json!({
                "name": "get_cluster_join_info",
                "description": "Get cluster join info",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "join_cluster",
                "description": "Join cluster",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hostname": { "type": "string", "description": "Node IP/Hostname" },
                        "password": { "type": "string", "description": "Root password" },
                        "fingerprint": { "type": "string", "description": "Node fingerprint" }
                    },
                    "required": ["hostname", "password", "fingerprint"]
                }
            }),
        ]
    }

    fn tool_defs_vm_lifecycle(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_vms",
                "description": "List all VMs/Containers",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "list_containers",
                "description": "List all LXC containers",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "bulk_vm_action",
                "description": "Power action on multiple VMs",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmids": { "type": "array", "items": { "type": "integer" }, "description": "VM IDs" },
                        "action": { "type": "string", "enum": ["start", "stop", "shutdown", "suspend", "resume", "reboot"], "description": "Action" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"], "description": "Type (default: qemu)" }
                    },
                    "required": ["node", "vmids", "action"]
                }
            }),
            json!({
                "name": "start_vm",
                "description": "Start VM/Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer", "description": "VM ID" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"], "description": "Type (qemu/lxc)" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "start_container",
                "description": "Start LXC container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer", "description": "CT ID" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "stop_vm",
                "description": "Stop VM/Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer", "description": "VM ID" },
                         "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "stop_container",
                "description": "Stop LXC container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer", "description": "CT ID" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "shutdown_vm",
                "description": "Shutdown VM/Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "shutdown_container",
                "description": "Shutdown LXC container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "reboot_vm",
                "description": "Reboot VM/Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "create_vm",
                "description": "Create QEMU VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "Target node" },
                        "vmid": { "type": "integer", "description": "VM ID" },
                        "name": { "type": "string", "description": "Name" },
                        "memory": { "type": "integer", "description": "Memory (MB)" },
                        "cores": { "type": "integer", "description": "Cores" },
                        "sockets": { "type": "integer", "description": "Sockets" },
                        "net0": { "type": "string", "description": "Network (e.g. 'virtio,bridge=vmbr0')" },
                        "ide2": { "type": "string", "description": "ISO (e.g. 'local:iso/debian.iso')" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "create_container",
                "description": "Create LXC container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "Target node" },
                        "vmid": { "type": "integer", "description": "CT ID" },
                        "ostemplate": { "type": "string", "description": "Template (e.g. 'local:vztmpl/ubuntu...')" },
                        "hostname": { "type": "string" },
                        "password": { "type": "string", "description": "Root password" },
                        "memory": { "type": "integer", "description": "Memory (MB)" },
                        "cores": { "type": "integer", "description": "Cores" },
                        "rootfs": { "type": "string", "description": "Rootfs (e.g. 'local-lvm:8')" }
                    },
                    "required": ["node", "vmid", "ostemplate"]
                }
            }),
            json!({
                "name": "delete_vm",
                "description": "Delete QEMU VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "delete_container",
                "description": "Delete LXC container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "reset_vm",
                "description": "Reset VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "vm_id": { "type": "string", "description": "VM ID" }
                    },
                    "required": ["vm_id"]
                }
            }),
            json!({
                "name": "reset_container",
                "description": "Reset Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "container_id": { "type": "string", "description": "CT ID" }
                    },
                    "required": ["container_id"]
                }
            }),
            json!({
                "name": "template_vm",
                "description": "Template VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "clone_vm",
                "description": "Clone VM/Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "Source node" },
                        "vmid": { "type": "integer", "description": "Source ID" },
                        "newid": { "type": "integer", "description": "New ID" },
                        "name": { "type": "string", "description": "New Name" },
                        "target": { "type": "string", "description": "Target node" },
                        "full": { "type": "boolean", "description": "Full clone" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid", "newid"]
                }
            }),
            json!({
                "name": "migrate_vm",
                "description": "Migrate VM/Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "Source node" },
                        "vmid": { "type": "integer" },
                        "target_node": { "type": "string", "description": "Target node" },
                        "online": { "type": "boolean", "description": "Online migration" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid", "target_node"]
                }
            }),
            json!({
                "name": "get_vm_config",
                "description": "Get VM/Container config",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "get_console_url",
                "description": "Get console URL (NoVNC/xtermjs/spice)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "console": { "type": "string", "enum": ["novnc", "xtermjs", "spice"], "description": "Console type" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "get_vm_stats",
                "description": "Get VM/Container RRD stats",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "timeframe": { "type": "string", "enum": ["hour", "day", "week", "month", "year"], "description": "Timeframe" },
                        "cf": { "type": "string", "enum": ["AVERAGE", "MAX"], "description": "Consolidation function" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
        ]
    }

    fn tool_defs_vm_config(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "update_vm_resources",
                "description": "Update VM hardware (cores, RAM, sockets)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "cores": { "type": "integer", "description": "New cores" },
                        "memory": { "type": "integer", "description": "New RAM (MB)" },
                        "sockets": { "type": "integer", "description": "New sockets" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "update_container_resources",
                "description": "Update CT resources (cores, RAM, swap, disk)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "cores": { "type": "integer", "description": "New cores" },
                        "memory": { "type": "integer", "description": "New RAM (MB)" },
                        "swap": { "type": "integer", "description": "New swap (MB)" },
                        "disk_gb": { "type": "integer", "description": "Add size (GB)" },
                        "disk": { "type": "string", "description": "Disk (default: rootfs)" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "add_lxc_bind_mount",
                "description": "Add CT bind mount",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "mp_id": { "type": "string", "description": "Mount point ID (e.g. mp0)" },
                        "source": { "type": "string", "description": "Host path" },
                        "target": { "type": "string", "description": "CT path" },
                        "read_only": { "type": "boolean" }
                    },
                    "required": ["node", "vmid", "mp_id", "source", "target"]
                }
            }),
            json!({
                "name": "add_disk",
                "description": "Add virtual disk",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "device": { "type": "string", "description": "Device (e.g. 'scsi1')" },
                        "storage": { "type": "string", "description": "Storage ID" },
                        "size_gb": { "type": "integer", "description": "Size (GB)" },
                        "format": { "type": "string", "enum": ["raw", "qcow2", "vmdk"] },
                        "extra_options": { "type": "string" }
                    },
                    "required": ["node", "vmid", "device", "storage", "size_gb"]
                }
            }),
            json!({
                "name": "remove_disk",
                "description": "Remove virtual disk",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "device": { "type": "string", "description": "Device (e.g. 'scsi1')" }
                    },
                    "required": ["node", "vmid", "device"]
                }
            }),
            json!({
                "name": "add_network",
                "description": "Add network interface",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "device": { "type": "string", "description": "ID (e.g. 'net1')" },
                        "bridge": { "type": "string", "description": "Bridge (e.g. 'vmbr0')" },
                        "model": { "type": "string", "description": "Model (e.g. 'virtio')" },
                        "mac": { "type": "string", "description": "MAC address" },
                        "extra_options": { "type": "string" }
                    },
                    "required": ["node", "vmid", "device", "bridge"]
                }
            }),
            json!({
                "name": "remove_network",
                "description": "Remove network interface",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "device": { "type": "string", "description": "ID (e.g. 'net1')" }
                    },
                    "required": ["node", "vmid", "device"]
                }
            }),
            json!({
                "name": "set_vm_cloudinit",
                "description": "Set VM Cloud-Init",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "ciuser": { "type": "string", "description": "User" },
                        "cipassword": { "type": "string", "description": "Password" },
                        "sshkeys": { "type": "string", "description": "SSH keys (URL-encoded)" },
                        "ipconfig0": { "type": "string", "description": "IP Config (e.g. ip=dhcp)" },
                        "nameserver": { "type": "string", "description": "DNS Server" },
                        "searchdomain": { "type": "string", "description": "DNS Search Domain" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "add_tag",
                "description": "Add tags",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "tags": { "type": "string", "description": "Comma separated tags" }
                    },
                    "required": ["node", "vmid", "tags"]
                }
            }),
            json!({
                "name": "remove_tag",
                "description": "Remove tags",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "tags": { "type": "string", "description": "Comma separated tags" }
                    },
                    "required": ["node", "vmid", "tags"]
                }
            }),
            json!({
                "name": "set_tags",
                "description": "Set tags",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "tags": { "type": "string", "description": "Comma separated tags" }
                    },
                    "required": ["node", "vmid", "tags"]
                }
            }),
            json!({
                "name": "list_snapshots",
                "description": "List snapshots",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "snapshot_vm",
                "description": "Create snapshot",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "snapname": { "type": "string", "description": "Name" },
                        "description": { "type": "string", "description": "Description" },
                        "vmstate": { "type": "boolean", "description": "Save RAM (QEMU)" },
                         "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid", "snapname"]
                }
            }),
            json!({
                "name": "rollback_vm",
                "description": "Rollback to snapshot",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "snapname": { "type": "string", "description": "Name" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid", "snapname"]
                }
            }),
            json!({
                "name": "delete_snapshot",
                "description": "Delete snapshot",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "snapname": { "type": "string", "description": "Name" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid", "snapname"]
                }
            }),
            json!({
                "name": "list_backups",
                "description": "List backups",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "storage": { "type": "string" },
                        "vmid": { "type": "integer", "description": "Filter VM ID" }
                    },
                    "required": ["node", "storage"]
                }
            }),
            json!({
                "name": "create_backup",
                "description": "Create backup",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "storage": { "type": "string", "description": "Target storage" },
                        "mode": { "type": "string", "enum": ["snapshot", "suspend", "stop"], "description": "Mode" },
                        "compress": { "type": "string", "enum": ["zstd", "gzip", "lzo"], "description": "Compression" },
                        "remove": { "type": "boolean", "description": "Prune old?" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "restore_backup",
                "description": "Restore backup",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer", "description": "Restore to ID" },
                        "archive": { "type": "string", "description": "Volume ID (volid)" },
                        "storage": { "type": "string", "description": "Target storage" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "force": { "type": "boolean", "description": "Overwrite?" }
                    },
                    "required": ["node", "vmid", "archive", "type"]
                }
            }),
        ]
    }

    fn tool_defs_storage(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "scan_storage_remote",
                "description": "Scan remote storage",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "type": { "type": "string", "enum": ["nfs", "cifs", "iscsi", "lvm", "zfs", "pbs"], "description": "Type" },
                        "server": { "type": "string", "description": "Host/IP" },
                        "username": { "type": "string" },
                        "password": { "type": "string" }
                    },
                    "required": ["node", "type", "server"]
                }
            }),
            json!({
                "name": "list_templates",
                "description": "List CT templates",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "storage": { "type": "string", "description": "Storage (default: local)" },
                        "content": { "type": "string", "description": "Type (default: vztmpl)" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "list_storage",
                "description": "List node storage",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "list_cluster_storage",
                "description": "List cluster storage defs",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "list_isos",
                "description": "List ISOs",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "storage": { "type": "string" }
                    },
                    "required": ["node", "storage"]
                }
            }),
            json!({
                "name": "add_storage",
                "description": "Add storage def",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "storage": { "type": "string", "description": "ID" },
                        "type": { "type": "string", "enum": ["dir", "nfs", "cifs", "lvm", "lvmthin", "zfs", "iscsi", "rbd", "cephfs"], "description": "Type" },
                        "content": { "type": "string", "description": "Allowed types (e.g. 'iso,backup')" },
                        "nodes": { "type": "array", "items": { "type": "string" }, "description": "Restrict to nodes" },
                        "enable": { "type": "boolean", "description": "Enable (default: true)" },
                        "path": { "type": "string", "description": "Path" },
                        "server": { "type": "string", "description": "Server" },
                        "share": { "type": "string", "description": "Share" },
                        "export": { "type": "string", "description": "Export" },
                        "username": { "type": "string" },
                        "password": { "type": "string" },
                        "pool": { "type": "string", "description": "Pool" },
                        "vgname": { "type": "string", "description": "VG name" }
                    },
                    "required": ["storage", "type"]
                }
            }),
            json!({
                "name": "delete_storage",
                "description": "Delete storage def",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "storage": { "type": "string", "description": "ID" }
                    },
                    "required": ["storage"]
                }
            }),
            json!({
                "name": "update_storage",
                "description": "Update storage def",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "storage": { "type": "string", "description": "ID" },
                        "content": { "type": "string", "description": "Allowed types" },
                        "nodes": { "type": "string", "description": "Restrict to nodes" },
                        "enable": { "type": "boolean" }
                    },
                    "required": ["storage"]
                }
            }),
            json!({
                "name": "download_url",
                "description": "Download ISO/Template",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "storage": { "type": "string" },
                        "url": { "type": "string", "description": "Source URL" },
                        "filename": { "type": "string", "description": "Target filename" },
                        "content": { "type": "string", "enum": ["iso", "vztmpl"], "description": "Type" },
                        "checksum": { "type": "string" },
                        "checksum_algorithm": { "type": "string", "enum": ["md5", "sha1", "sha224", "sha256", "sha384", "sha512"] }
                    },
                    "required": ["node", "storage", "url", "filename", "content"]
                }
            }),
            json!({
                "name": "delete_storage_content",
                "description": "Delete storage volume",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "storage": { "type": "string" },
                        "volume": { "type": "string", "description": "Volume ID" }
                    },
                    "required": ["node", "storage", "volume"]
                }
            }),
            json!({
                "name": "get_storage_volume",
                "description": "Get storage volume info",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "storage": { "type": "string" },
                        "volume": { "type": "string" }
                    },
                    "required": ["node", "storage", "volume"]
                }
            }),
        ]
    }

    fn tool_defs_network(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_networks",
                "description": "List node network interfaces",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "create_network_bridge",
                "description": "Create network bridge",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "iface": { "type": "string", "description": "Name (e.g. vmbr0)" },
                        "bridge_ports": { "type": "string", "description": "Ports" },
                        "cidr": { "type": "string", "description": "IP/CIDR" },
                        "gateway": { "type": "string", "description": "Gateway IP" },
                        "autostart": { "type": "boolean" },
                        "comments": { "type": "string" }
                    },
                    "required": ["node", "iface"]
                }
            }),
            json!({
                "name": "create_network_bond",
                "description": "Create network bond",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "iface": { "type": "string", "description": "Interface Name (e.g. bond0)" },
                        "slaves": { "type": "string", "description": "Slaves (e.g. eno1 eno2)" },
                        "bond_mode": { "type": "string", "enum": ["balance-rr", "active-backup", "balance-xor", "broadcast", "802.3ad", "balance-tlb", "balance-alb", "unknown"], "description": "Bond mode" },
                        "bond_xmit_hash_policy": { "type": "string", "description": "Hash policy" },
                        "cidr": { "type": "string" },
                        "autostart": { "type": "boolean" },
                        "comments": { "type": "string" }
                    },
                    "required": ["node", "iface", "slaves"]
                }
            }),
            json!({
                "name": "update_network_interface",
                "description": "Update a network interface configuration",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "iface": { "type": "string" },
                        "bridge_ports": { "type": "string" },
                        "slaves": { "type": "string" },
                        "cidr": { "type": "string" },
                        "gateway": { "type": "string" },
                        "autostart": { "type": "boolean" },
                        "comments": { "type": "string" }
                    },
                    "required": ["node", "iface"]
                }
            }),
            json!({
                "name": "delete_network_interface",
                "description": "Delete a network interface configuration",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "iface": { "type": "string" }
                    },
                    "required": ["node", "iface"]
                }
            }),
            json!({
                "name": "apply_network_config",
                "description": "Apply pending network configuration changes",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "revert_network_config",
                "description": "Revert pending network configuration changes",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "list_firewall_rules",
                "description": "List firewall rules",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "Node (opt)" },
                        "vmid": { "type": "integer", "description": "VM ID (opt)" }
                    },
                    "required": []
                }
            }),
            json!({
                "name": "add_firewall_rule",
                "description": "Add a firewall rule",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["in", "out"], "description": "Direction" },
                        "action": { "type": "string", "enum": ["ACCEPT", "DROP", "REJECT"] },
                        "source": { "type": "string" },
                        "dest": { "type": "string" },
                        "proto": { "type": "string" },
                        "dport": { "type": "string" },
                        "sport": { "type": "string" },
                        "comment": { "type": "string" },
                        "enable": { "type": "integer", "description": "Enable rule (0 or 1)" }
                    },
                    "required": ["type", "action"]
                }
            }),
            json!({
                "name": "delete_firewall_rule",
                "description": "Delete firewall rule",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "pos": { "type": "integer", "description": "Index" }
                    },
                    "required": ["pos"]
                }
            }),
        ]
    }

    fn tool_defs_firewall_aliases(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_firewall_aliases",
                "description": "List firewall aliases",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "level": { "type": "string", "enum": ["cluster", "node"], "description": "Scope" },
                        "node": { "type": "string" }
                    },
                    "required": ["level"]
                }
            }),
            json!({
                "name": "create_firewall_alias",
                "description": "Create firewall alias",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "level": { "type": "string", "enum": ["cluster", "node"] },
                        "node": { "type": "string" },
                        "name": { "type": "string" },
                        "cidr": { "type": "string", "description": "CIDR" },
                        "comment": { "type": "string" }
                    },
                    "required": ["level", "name", "cidr"]
                }
            }),
            json!({
                "name": "update_firewall_alias",
                "description": "Update firewall alias",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "level": { "type": "string", "enum": ["cluster", "node"] },
                        "node": { "type": "string" },
                        "name": { "type": "string" },
                        "cidr": { "type": "string" },
                        "comment": { "type": "string" }
                    },
                    "required": ["level", "name", "cidr"]
                }
            }),
            json!({
                "name": "delete_firewall_alias",
                "description": "Delete firewall alias",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "level": { "type": "string", "enum": ["cluster", "node"] },
                        "node": { "type": "string" },
                        "name": { "type": "string" }
                    },
                    "required": ["level", "name"]
                }
            }),
        ]
    }

    fn tool_defs_certificates(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_certificates",
                "description": "List node certs",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "upload_certificate",
                "description": "Upload SSL cert/key",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "certificates": { "type": "string", "description": "PEM chain" },
                        "key": { "type": "string", "description": "PEM key" },
                        "force": { "type": "boolean" },
                        "restart": { "type": "boolean", "description": "Restart pveproxy?" }
                    },
                    "required": ["node", "certificates", "key"]
                }
            }),
            json!({
                "name": "generate_acme_certificate",
                "description": "Renew ACME cert",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
        ]
    }

    fn tool_defs_apt(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_repositories",
                "description": "List APT repos",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "add_repository",
                "description": "Add Proxmox repo",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "handle": { "type": "string", "description": "Handle" }
                    },
                    "required": ["node", "handle"]
                }
            }),
            json!({
                "name": "update_repository_state",
                "description": "Set repo state",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "path": { "type": "string" },
                        "index": { "type": "integer" },
                        "enabled": { "type": "boolean" }
                    },
                    "required": ["node", "path", "index", "enabled"]
                }
            }),
        ]
    }

    fn tool_defs_firewall_security_groups(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_security_groups",
                "description": "List security groups",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_security_group",
                "description": "Create security group",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "comment": { "type": "string" }
                    },
                    "required": ["name"]
                }
            }),
            json!({
                "name": "delete_security_group",
                "description": "Delete security group",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" }
                    },
                    "required": ["name"]
                }
            }),
            json!({
                "name": "list_security_group_rules",
                "description": "List group rules",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" }
                    },
                    "required": ["name"]
                }
            }),
            json!({
                "name": "add_security_group_rule",
                "description": "Add group rule",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "type": { "type": "string", "enum": ["in", "out"], "description": "Dir" },
                        "action": { "type": "string", "enum": ["ACCEPT", "DROP", "REJECT"] },
                        "source": { "type": "string" },
                        "dest": { "type": "string" },
                        "proto": { "type": "string" },
                        "dport": { "type": "string" },
                        "sport": { "type": "string" },
                        "comment": { "type": "string" },
                        "enable": { "type": "integer", "description": "Enable (0/1)" }
                    },
                    "required": ["name", "type", "action"]
                }
            }),
        ]
    }

    fn tool_defs_system(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_tasks",
                "description": "List node tasks",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "limit": { "type": "integer", "description": "Limit" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "get_task_status",
                "description": "Get task status",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "upid": { "type": "string", "description": "UPID" }
                    },
                    "required": ["node", "upid"]
                }
            }),
            json!({
                "name": "read_task_log",
                "description": "Read task log",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "upid": { "type": "string", "description": "UPID" }
                    },
                    "required": ["node", "upid"]
                }
            }),
            json!({
                "name": "wait_for_task",
                "description": "Wait for task",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "upid": { "type": "string", "description": "UPID" },
                        "timeout": { "type": "integer", "description": "Timeout (s)" }
                    },
                    "required": ["node", "upid"]
                }
            }),
            json!({
                "name": "list_services",
                "description": "List node services",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "manage_service",
                "description": "Manage node service",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "service": { "type": "string", "description": "Service" },
                        "action": { "type": "string", "enum": ["start", "stop", "restart", "reload"] }
                    },
                    "required": ["node", "service", "action"]
                }
            }),
            json!({
                "name": "list_apt_updates",
                "description": "List APT updates",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "run_apt_update",
                "description": "Run APT update",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "get_apt_versions",
                "description": "Get PVE versions",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "get_subscription_info",
                "description": "Get node subscription",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "set_subscription_key",
                "description": "Set sub key",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "key": { "type": "string" }
                    },
                    "required": ["node", "key"]
                }
            }),
            json!({
                "name": "check_subscription",
                "description": "Check subscription",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
        ]
    }

    fn tool_defs_access(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_users",
                "description": "List cluster users",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_user",
                "description": "Create user",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "userid": { "type": "string", "description": "User ID (e.g. user@pve)" },
                        "password": { "type": "string" },
                        "email": { "type": "string" },
                        "firstname": { "type": "string" },
                        "lastname": { "type": "string" },
                        "expire": { "type": "integer", "description": "Expiry (epoch)" },
                        "enable": { "type": "boolean" },
                        "comment": { "type": "string" },
                        "groups": { "type": "array", "items": { "type": "string" }, "description": "Groups" }
                    },
                    "required": ["userid", "password"]
                }
            }),
            json!({
                "name": "delete_user",
                "description": "Delete user",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "userid": { "type": "string" }
                    },
                    "required": ["userid"]
                }
            }),
            json!({
                "name": "list_roles",
                "description": "List roles",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_role",
                "description": "Create role",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "roleid": { "type": "string" },
                        "privs": { "type": "string", "description": "Comma separated privs" }
                    },
                    "required": ["roleid", "privs"]
                }
            }),
            json!({
                "name": "update_role",
                "description": "Update role",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "roleid": { "type": "string" },
                        "privs": { "type": "string" },
                        "append": { "type": "boolean" }
                    },
                    "required": ["roleid", "privs"]
                }
            }),
            json!({
                "name": "delete_role",
                "description": "Delete role",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "roleid": { "type": "string" }
                    },
                    "required": ["roleid"]
                }
            }),
            json!({
                "name": "list_acls",
                "description": "List ACLs",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "update_acl",
                "description": "Update ACL",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path (e.g. /vms/100)" },
                        "users": { "type": "string", "description": "Users" },
                        "groups": { "type": "string", "description": "Groups" },
                        "tokens": { "type": "string", "description": "API tokens" },
                        "roles": { "type": "string", "description": "Roles" },
                        "delete": { "type": "integer", "enum": [0, 1], "description": "Remove?" },
                        "propagate": { "type": "integer", "enum": [0, 1] }
                    },
                    "required": ["path", "roles"]
                }
            }),
        ]
    }

    fn tool_defs_ha(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_pools",
                "description": "List resource pools",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_pool",
                "description": "Create pool",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "poolid": { "type": "string" },
                        "comment": { "type": "string" }
                    },
                    "required": ["poolid"]
                }
            }),
            json!({
                "name": "get_pool_details",
                "description": "Get pool details",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "poolid": { "type": "string" }
                    },
                    "required": ["poolid"]
                }
            }),
            json!({
                "name": "update_pool",
                "description": "Update pool",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "poolid": { "type": "string" },
                        "comment": { "type": "string" },
                        "vms": { "type": "string", "description": "VM IDs" },
                        "storage": { "type": "string", "description": "Storage IDs" },
                        "delete": { "type": "integer", "enum": [0, 1] }
                    },
                    "required": ["poolid"]
                }
            }),
            json!({
                "name": "delete_pool",
                "description": "Delete pool",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "poolid": { "type": "string" }
                    },
                    "required": ["poolid"]
                }
            }),
            json!({
                "name": "list_replication_jobs",
                "description": "List replication jobs",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_replication_job",
                "description": "Create replication job",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "target": { "type": "string" },
                        "schedule": { "type": "string" },
                        "rate": { "type": "number", "description": "MB/s" },
                        "comment": { "type": "string" },
                        "enable": { "type": "boolean" }
                    },
                    "required": ["id", "target"]
                }
            }),
            json!({
                "name": "update_replication_job",
                "description": "Update replication job",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "schedule": { "type": "string" },
                        "rate": { "type": "number" },
                        "comment": { "type": "string" },
                        "enable": { "type": "boolean" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "delete_replication_job",
                "description": "Delete replication job",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "list_ha_resources",
                "description": "List HA resources",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "list_ha_groups",
                "description": "List HA groups",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "add_ha_resource",
                "description": "Add HA resource",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sid": { "type": "string", "description": "Service ID (e.g. vm:100)" },
                        "comment": { "type": "string" },
                        "group": { "type": "string" },
                        "max_relocate": { "type": "integer" },
                        "max_restart": { "type": "integer" },
                        "state": { "type": "string", "enum": ["started", "stopped", "enabled", "disabled", "ignored"] }
                    },
                    "required": ["sid"]
                }
            }),
            json!({
                "name": "update_ha_resource",
                "description": "Update HA resource",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sid": { "type": "string" },
                        "comment": { "type": "string" },
                        "group": { "type": "string" },
                        "max_relocate": { "type": "integer" },
                        "max_restart": { "type": "integer" },
                        "state": { "type": "string", "enum": ["started", "stopped", "enabled", "disabled", "ignored"] }
                    },
                    "required": ["sid"]
                }
            }),
            json!({
                "name": "remove_ha_resource",
                "description": "Remove HA resource",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sid": { "type": "string" }
                    },
                    "required": ["sid"]
                }
            }),
        ]
    }

    fn tool_defs_mapping(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_pci_mappings",
                "description": "List PCI mappings",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_pci_mapping",
                "description": "Create PCI mapping",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "map": { "type": "string", "description": "Entries" },
                        "description": { "type": "string" }
                    },
                    "required": ["id", "map"]
                }
            }),
            json!({
                "name": "update_pci_mapping",
                "description": "Update PCI mapping",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "map": { "type": "string" },
                        "description": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "delete_pci_mapping",
                "description": "Delete PCI mapping",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "list_usb_mappings",
                "description": "List USB mappings",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_usb_mapping",
                "description": "Create USB mapping",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "map": { "type": "string" },
                        "description": { "type": "string" }
                    },
                    "required": ["id", "map"]
                }
            }),
            json!({
                "name": "update_usb_mapping",
                "description": "Update USB mapping",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "map": { "type": "string" },
                        "description": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "delete_usb_mapping",
                "description": "Delete USB mapping",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }),
        ]
    }

    fn tool_defs_metric_server(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_metric_servers",
                "description": "List metric servers",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_metric_server",
                "description": "Add metric server",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "type": { "type": "string", "enum": ["influxdb", "graphite"] },
                        "server": { "type": "string" },
                        "port": { "type": "integer" },
                        "path": { "type": "string" },
                        "bucket": { "type": "string" },
                        "organization": { "type": "string" },
                        "token": { "type": "string" }
                    },
                    "required": ["id", "type", "server", "port"]
                }
            }),
            json!({
                "name": "update_metric_server",
                "description": "Update metric server",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "server": { "type": "string" },
                        "port": { "type": "integer" },
                        "disable": { "type": "integer", "enum": [0, 1] }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "delete_metric_server",
                "description": "Remove metric server",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }),
        ]
    }

    fn tool_defs_backup_schedule(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_backup_schedules",
                "description": "List backup schedules",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_backup_schedule",
                "description": "Create backup schedule",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "vmid": { "type": "string", "description": "VMIDs/all" },
                        "storage": { "type": "string" },
                        "schedule": { "type": "string", "description": "PVE schedule" },
                        "mode": { "type": "string", "enum": ["snapshot", "suspend", "stop"] },
                        "compress": { "type": "string", "enum": ["zstd", "gzip", "lzo"] },
                        "enabled": { "type": "boolean" },
                        "node": { "type": "string" },
                        "all": { "type": "boolean" },
                        "exclude": { "type": "string" }
                    },
                    "required": ["storage", "schedule"]
                }
            }),
            json!({
                "name": "update_backup_schedule",
                "description": "Update backup schedule",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Job ID" },
                        "vmid": { "type": "string" },
                        "storage": { "type": "string" },
                        "schedule": { "type": "string" },
                        "mode": { "type": "string", "enum": ["snapshot", "suspend", "stop"] },
                        "compress": { "type": "string", "enum": ["zstd", "gzip", "lzo"] },
                        "enabled": { "type": "boolean" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "delete_backup_schedule",
                "description": "Delete backup schedule",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }),
        ]
    }

    fn tool_defs_ceph(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "get_ceph_status",
                "description": "Get Ceph status",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "list_ceph_pools",
                "description": "List Ceph pools",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "create_ceph_pool",
                "description": "Create Ceph pool",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "name": { "type": "string" },
                        "pg_num": { "type": "integer" },
                        "add_storages": { "type": "integer", "description": "0/1" },
                        "min_size": { "type": "integer" },
                        "size": { "type": "integer" },
                        "crush_rule": { "type": "string" }
                    },
                    "required": ["node", "name"]
                }
            }),
            json!({
                "name": "delete_ceph_pool",
                "description": "Delete Ceph pool",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "name": { "type": "string" },
                        "remove_storages": { "type": "boolean" }
                    },
                    "required": ["node", "name"]
                }
            }),
            json!({
                "name": "list_ceph_osds",
                "description": "List Ceph OSDs",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "list_ceph_monitors",
                "description": "List Ceph Monitors",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
        ]
    }

    fn tool_defs_sdn(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_sdn_zones",
                "description": "List SDN Zones",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_sdn_zone",
                "description": "Create SDN Zone",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "zone": { "type": "string" },
                        "type": { "type": "string", "enum": ["simple", "vlan", "qinq", "vxlan", "evpn"] },
                        "mtu": { "type": "integer" },
                        "nodes": { "type": "string" }
                    },
                    "required": ["zone", "type"]
                }
            }),
            json!({
                "name": "delete_sdn_zone",
                "description": "Delete SDN Zone",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "zone": { "type": "string" }
                    },
                    "required": ["zone"]
                }
            }),
            json!({
                "name": "list_sdn_vnets",
                "description": "List SDN Vnets",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_sdn_vnet",
                "description": "Create SDN Vnet",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "vnet": { "type": "string" },
                        "zone": { "type": "string" },
                        "tag": { "type": "integer" },
                        "alias": { "type": "string" }
                    },
                    "required": ["vnet", "zone"]
                }
            }),
            json!({
                "name": "delete_sdn_vnet",
                "description": "Delete SDN Vnet",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "vnet": { "type": "string" }
                    },
                    "required": ["vnet"]
                }
            }),
            json!({
                "name": "apply_sdn_changes",
                "description": "Apply SDN changes",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
        ]
    }

    fn tool_defs_misc(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "vm_agent_ping",
                "description": "Ping VM Guest Agent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "vm_exec",
                "description": "Exec command in VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "command": { "type": "string", "description": "e.g. 'ls -l /'" },
                        "input_data": { "type": "string" }
                    },
                    "required": ["node", "vmid", "command"]
                }
            }),
            json!({
                "name": "vm_exec_status",
                "description": "Get VM exec status",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "pid": { "type": "integer" }
                    },
                    "required": ["node", "vmid", "pid"]
                }
            }),
            json!({
                "name": "vm_read_file",
                "description": "Read file from VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "file": { "type": "string" }
                    },
                    "required": ["node", "vmid", "file"]
                }
            }),
            json!({
                "name": "vm_write_file",
                "description": "Write file to VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "file": { "type": "string" },
                        "content": { "type": "string" },
                        "encode": { "type": "boolean" }
                    },
                    "required": ["node", "vmid", "file", "content"]
                }
            }),
            json!({
                "name": "list_pci_devices",
                "description": "List node PCI devices",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "list_usb_devices",
                "description": "List node USB devices",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "add_pci_device",
                "description": "Add PCI device to VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "device_id": { "type": "string", "description": "e.g. hostpci0" },
                        "host": { "type": "string", "description": "PCI ID/mapping" },
                        "pcie": { "type": "boolean" },
                        "mdev": { "type": "string" },
                        "extra_options": { "type": "string" }
                    },
                    "required": ["node", "vmid", "device_id", "host"]
                }
            }),
            json!({
                "name": "add_usb_device",
                "description": "Add USB device to VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "device_id": { "type": "string", "description": "e.g. usb0" },
                        "host": { "type": "string", "description": "host=ID/spice" },
                        "usb3": { "type": "boolean" },
                        "extra_options": { "type": "string" }
                    },
                    "required": ["node", "vmid", "device_id", "host"]
                }
            }),
            json!({
                "name": "remove_vm_device",
                "description": "Remove PCI/USB from VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "device_id": { "type": "string" }
                    },
                    "required": ["node", "vmid", "device_id"]
                }
            }),
            json!({
                "name": "add_lxc_mountpoint",
                "description": "Add CT mountpoint",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "mp_id": { "type": "string", "description": "e.g. mp0" },
                        "volume": { "type": "string" },
                        "path": { "type": "string" },
                        "read_only": { "type": "boolean" },
                        "backup": { "type": "boolean" },
                        "extra_options": { "type": "string" }
                    },
                    "required": ["node", "vmid", "mp_id", "volume", "path"]
                }
            }),
            json!({
                "name": "remove_lxc_mountpoint",
                "description": "Remove CT mountpoint",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "mp_id": { "type": "string", "description": "Mount point ID (e.g. mp0)" }
                    },
                    "required": ["node", "vmid", "mp_id"]
                }
            }),
        ]
    }
}
