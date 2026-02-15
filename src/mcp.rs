use crate::proxmox::ProxmoxClient;
use anyhow::Result;
use log::error;
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
            if reader.read_line(&mut line)? == 0 {
                break;
            }
            let input = line.trim();
            if input.is_empty() {
                continue;
            }
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
                                            let c = match status.as_u16() {
                                                401 | 403 => -32001,
                                                404 => -32004,
                                                _ => -32603,
                                            };
                                            (
                                                c,
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
                        println!("{}", serde_json::to_string(&json_resp)?);
                        io::stdout().flush()?;
                        if self.check_notification() {
                            println!(
                                "{}",
                                serde_json::to_string(
                                    &json!({"jsonrpc": "2.0", "method": "notifications/tools/list_changed"})
                                )?
                            );
                            io::stdout().flush()?;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse JSON-RPC: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> Result<Value> {
        match req.method.as_str() {
            "initialize" => Ok(
                json!({"protocolVersion": "2024-11-05", "serverInfo": {"name": "proxmox-mcp-rs", "version": env!("CARGO_PKG_VERSION")}, "capabilities": {"tools": {"listChanged": true}, "resources": {}}}),
            ),
            "notifications/initialized" => Ok(Value::Null),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": self.get_tool_definitions()})),
            "tools/call" => {
                if let Some(params) = req.params {
                    self.call_tool(
                        params.get("name").and_then(|n| n.as_str()).unwrap_or(""),
                        params.get("arguments").unwrap_or(&Value::Null),
                    )
                    .await
                } else {
                    anyhow::bail!("Missing params");
                }
            }
            "resources/list" => Ok(json!({"resources": self.get_resource_definitions()})),
            "resources/read" => {
                if let Some(params) = req.params {
                    self.handle_resource_read(
                        params.get("uri").and_then(|n| n.as_str()).unwrap_or(""),
                    )
                    .await
                } else {
                    anyhow::bail!("Missing params");
                }
            }
            _ => anyhow::bail!("Method not found: {}", req.method),
        }
    }

    fn get_resource_definitions(&self) -> Vec<Value> {
        vec![
            json!({"uri": "proxmox://vms", "name": "VMs", "description": "List of VMs and containers", "mimeType": "application/json"}),
        ]
    }

    fn get_tool_definitions(&self) -> Vec<Value> {
        {
            let state = self.state.lock().unwrap();
            if state.lazy_mode && !state.tools_loaded {
                return vec![
                    json!({"name": "load_all_tools", "description": "Load all tools", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
                    json!({"name": "get_cluster_status", "description": "Cluster status", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
                    json!({"name": "list_nodes", "description": "List nodes", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
                ];
            }
        }
        let mut tools = Vec::new();
        tools.extend(self.tool_defs_consolidated());
        tools.extend(self.tool_defs_cluster());
        tools.extend(self.tool_defs_vm_lifecycle());
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
                Ok(
                    json!({"contents": [{"uri": uri, "mimeType": "application/json", "text": serde_json::to_string(&vms)?}]}),
                )
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
            "list_nodes" => Ok(
                json!({ "content": [{ "type": "text", "text": serde_json::to_string(&self.get_client(args)?.get_nodes().await?)? }] }),
            ),
            "list_vms" => Ok(
                json!({ "content": [{ "type": "text", "text": serde_json::to_string(&self.get_client(args)?.get_all_vms().await?)? }] }),
            ),
            "list_containers" => {
                let vms = self.get_client(args)?.get_all_vms().await?;
                let ct: Vec<_> = vms
                    .into_iter()
                    .filter(|v| v.vm_type.as_deref() == Some("lxc"))
                    .collect();
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&ct)? }] }))
            }
            "vm_power_action" => self.handle_vm_power_action(args).await,
            "manage_resource" => self.handle_manage_resource(args).await,
            "manage_resource_config" => self.handle_manage_resource_config(args).await,
            "manage_snapshot_backup" => self.handle_manage_snapshot_backup(args).await,
            "manage_node_system" => self.handle_manage_node_system(args).await,
            "manage_cluster_config" => self.handle_manage_cluster_config(args).await,
            "manage_tags" => self.handle_manage_tags(args).await,
            "list_storage" => self.handle_list_storage(args).await,
            "list_cluster_storage" => self.handle_list_cluster_storage(args).await,
            "list_networks" => self.handle_list_networks(args).await,
            "list_firewall_rules" => self.handle_list_firewall_rules(args).await,
            "list_firewall_aliases" => self.handle_list_firewall_aliases(args).await,
            "list_security_groups" => self.handle_list_security_groups(args).await,
            "list_security_group_rules" => self.handle_list_security_group_rules(args).await,
            "list_tasks" => self.handle_list_tasks(args).await,
            "list_backups" => self.handle_list_backups(args).await,
            "list_snapshots" => self.handle_snapshot_list(args).await,
            "list_templates" => {
                let n = args
                    .get("node")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing node"))?;
                let s = args
                    .get("storage")
                    .and_then(|v| v.as_str())
                    .unwrap_or("local");
                let t = self
                    .get_client(args)?
                    .get_storage_content(
                        n,
                        s,
                        Some(
                            args.get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("vztmpl"),
                        ),
                    )
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&t)? }] }))
            }
            "list_isos" => self.handle_list_isos(args).await,
            "list_pools" => self.handle_list_pools(args).await,
            "list_replication_jobs" => self.handle_list_replication_jobs(args).await,
            "list_ha_resources" => self.handle_list_ha_resources(args).await,
            "list_ha_groups" => self.handle_list_ha_groups(args).await,
            "list_users" => self.handle_list_users(args).await,
            "list_roles" => self.handle_list_roles(args).await,
            "list_acls" => self.handle_list_acls(args).await,
            "list_apt_updates" => self.handle_list_apt_updates(args).await,
            "list_services" => self.handle_list_services(args).await,
            "list_certificates" => self.handle_list_certificates(args).await,
            "list_pci_devices" => self.handle_list_pci_devices(args).await,
            "list_usb_devices" => self.handle_list_usb_devices(args).await,
            "list_pci_mappings" => self.handle_list_pci_mappings(args).await,
            "list_usb_mappings" => self.handle_list_usb_mappings(args).await,
            "list_metric_servers" => self.handle_list_metric_servers(args).await,
            "list_sdn_zones" => self.handle_list_sdn_zones(args).await,
            "list_sdn_vnets" => self.handle_list_sdn_vnets(args).await,
            "list_ceph_pools" => self.handle_list_ceph_pools(args).await,
            "list_ceph_osds" => self.handle_list_ceph_osds(args).await,
            "list_ceph_monitors" => self.handle_list_ceph_monitors(args).await,
            "list_backup_schedules" => self.handle_list_backup_schedules(args).await,
            "list_repositories" => self.handle_list_repositories(args).await,
            "bulk_vm_action" => self.handle_bulk_vm_action(args).await,
            "scan_storage_remote" => self.handle_scan_storage_remote(args).await,
            "wait_for_task" => self.handle_wait_for_task(args).await,
            "apply_sdn_changes" => self.handle_apply_sdn_changes(args).await,
            "vm_agent_ping" => self.handle_vm_agent_ping(args).await,
            "vm_exec_status" => self.handle_vm_exec_status(args).await,
            "vm_read_file" => self.handle_vm_read_file(args).await,
            "vm_write_file" => self.handle_vm_write_file(args).await,
            "download_url" => self.handle_download_url(args).await,
            "get_cluster_status" => self.handle_get_cluster_status(args).await,
            "get_cluster_log" => self.handle_get_cluster_log(args).await,
            "get_node_stats" => self.handle_get_node_stats(args).await,
            "get_vm_stats" => self.handle_get_vm_stats(args).await,
            "get_vm_config" => self.handle_get_vm_config(args).await,
            "get_task_status" => self.handle_get_task_status(args).await,
            "read_task_log" => self.handle_read_task_log(args).await,
            "get_storage_volume" => self.handle_get_storage_volume(args).await,
            "get_pool_details" => self.handle_get_pool_details(args).await,
            "get_subscription_info" => self.handle_get_subscription_info(args).await,
            "get_apt_versions" => self.handle_get_apt_versions(args).await,
            "get_ceph_status" => self.handle_get_ceph_status(args).await,
            "get_console_url" => {
                let n = args
                    .get("node")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing node"))?;
                let id = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let url = self.get_client(args)?.get_console_url(
                    n,
                    id,
                    args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu"),
                    args.get("console").and_then(|v| v.as_str()),
                )?;
                Ok(json!({ "content": [{ "type": "text", "text": url }] }))
            }
            _ => anyhow::bail!("Unknown tool: {}", name),
        }
    }

    fn tool_defs_consolidated(&self) -> Vec<Value> {
        vec![
            json!({"name": "vm_power_action", "description": "Power action", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}, "vmid": {"type": "integer"}, "action": {"type": "string", "enum": ["start", "stop", "shutdown", "reboot", "reset", "suspend", "resume"]}, "type": {"type": "string", "enum": ["qemu", "lxc"]}}, "required": ["vmid", "action"]}}),
            json!({"name": "manage_resource", "description": "Lifecycle", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}, "vmid": {"type": "integer"}, "action": {"type": "string", "enum": ["create", "delete", "clone", "migrate", "template"]}, "type": {"type": "string", "enum": ["qemu", "lxc"]}}, "required": ["action", "type"]}}),
            json!({"name": "manage_resource_config", "description": "Config", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}, "vmid": {"type": "integer"}, "action": {"type": "string", "enum": ["update_resources", "add_disk", "remove_disk", "add_network", "remove_network", "set_cloudinit", "exec", "add_lxc_mountpoint", "add_pci_device", "add_usb_device", "remove_vm_device"]}, "type": {"type": "string", "enum": ["qemu", "lxc"]}}, "required": ["vmid", "action"]}}),
            json!({"name": "manage_snapshot_backup", "description": "Snapshots/Backups", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}, "action": {"type": "string", "enum": ["snapshot_create", "snapshot_rollback", "snapshot_delete", "backup_create", "backup_restore", "create", "update", "delete"]}, "vmid": {"type": "integer"}}, "required": ["node", "action"]}}),
            json!({"name": "manage_node_system", "description": "Node system", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}, "action": {"type": "string", "enum": ["manage_service", "apt_update", "cert_renew", "upload_certificate", "add_repository", "update_repository_state", "set_subscription_key"]}}, "required": ["node", "action"]}}),
            json!({"name": "manage_cluster_config", "description": "Cluster config", "inputSchema": {"type": "object", "properties": {"action": {"type": "string", "enum": ["add", "update", "delete", "create", "remove"]}, "type": {"type": "string", "enum": ["storage", "sdn", "firewall_alias", "security_group", "pool", "role", "replication", "ha", "mapping", "metric", "ceph", "user", "acl"]}}, "required": ["action", "type"]}}),
            json!({"name": "manage_tags", "description": "Manage tags", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}, "vmid": {"type": "integer"}, "action": {"type": "string", "enum": ["add", "remove", "set"]}, "tags": {"type": "string"}, "type": {"type": "string", "enum": ["qemu", "lxc"]}}, "required": ["vmid", "action", "tags"]}}),
        ]
    }

    fn tool_defs_cluster(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_nodes", "description": "List nodes", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "get_cluster_status", "description": "Cluster status", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "get_cluster_log", "description": "Cluster log", "inputSchema": {"type": "object", "properties": {"limit": {"type": "integer"}}, "required": []}}),
            json!({"name": "get_node_stats", "description": "Node stats", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
        ]
    }

    fn tool_defs_vm_lifecycle(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_vms", "description": "List VMs", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "list_containers", "description": "List LXC", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
        ]
    }

    fn tool_defs_storage(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_storage", "description": "List storage", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
            json!({"name": "list_cluster_storage", "description": "List storage (cluster)", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "list_templates", "description": "List templates", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}, "storage": {"type": "string"}}, "required": ["node"]}}),
            json!({"name": "list_isos", "description": "List ISOs", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}, "storage": {"type": "string"}}, "required": ["node", "storage"]}}),
        ]
    }

    fn tool_defs_network(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_networks", "description": "List networks", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
        ]
    }

    fn tool_defs_firewall_aliases(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_firewall_aliases", "description": "List aliases", "inputSchema": {"type": "object", "properties": {"level": {"type": "string", "enum": ["cluster", "node"]}, "node": {"type": "string"}}, "required": ["level"]}}),
        ]
    }

    fn tool_defs_firewall_security_groups(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_security_groups", "description": "List security groups", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "list_security_group_rules", "description": "List security rules", "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}}),
        ]
    }

    fn tool_defs_system(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_tasks", "description": "List tasks", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}, "limit": {"type": "integer"}}, "required": ["node"]}}),
            json!({ "name": "list_services", "description": "List services", "inputSchema": { "type": "object", "properties": { "node": { "type": "string" } }, "required": ["node"] } }),
        ]
    }

    fn tool_defs_apt(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_repositories", "description": "List repositories", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
            json!({"name": "list_apt_updates", "description": "List updates", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
        ]
    }

    fn tool_defs_certificates(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_certificates", "description": "List certificates", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
        ]
    }

    fn tool_defs_access(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_users", "description": "List users", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "list_roles", "description": "List roles", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "list_acls", "description": "List ACLs", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
        ]
    }

    fn tool_defs_ha(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_pools", "description": "List pools", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "list_replication_jobs", "description": "List replication", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "list_ha_resources", "description": "List HA", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "list_ha_groups", "description": "List HA groups", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
        ]
    }

    fn tool_defs_sdn(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_sdn_zones", "description": "List zones", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "list_sdn_vnets", "description": "List vnets", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
        ]
    }

    fn tool_defs_ceph(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_ceph_pools", "description": "List pools (Ceph)", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
            json!({"name": "list_ceph_osds", "description": "List OSDs", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
            json!({"name": "list_ceph_monitors", "description": "List monitors", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
        ]
    }

    fn tool_defs_backup_schedule(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_backup_schedules", "description": "List backup jobs", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
        ]
    }

    fn tool_defs_mapping(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_pci_mappings", "description": "List PCI mappings", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
            json!({"name": "list_usb_mappings", "description": "List USB mappings", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
        ]
    }

    fn tool_defs_metric_server(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_metric_servers", "description": "List metric servers", "inputSchema": {"type": "object", "properties": {}, "required": []}}),
        ]
    }

    fn tool_defs_misc(&self) -> Vec<Value> {
        vec![
            json!({"name": "list_pci_devices", "description": "List PCI devices", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
            json!({"name": "list_usb_devices", "description": "List USB devices", "inputSchema": {"type": "object", "properties": {"node": {"type": "string"}}, "required": ["node"]}}),
        ]
    }

    async fn handle_vm_power_action(&self, args: &Value) -> Result<Value> {
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let client = self.get_client(args)?;
        let (node, vm_type) = if let Some(n) = args.get("node").and_then(|v| v.as_str()) {
            (
                n.to_string(),
                args.get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("qemu")
                    .to_string(),
            )
        } else {
            client.find_vm_location(vmid).await?
        };
        let res = client
            .vm_action(&node, vmid, action, Some(&vm_type))
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Action '{}' initiated. UPID: {}", action, res) }] }),
        )
    }

    async fn handle_manage_resource(&self, args: &Value) -> Result<Value> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let resource_type = args
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing type"))?;
        let client = self.get_client(args)?;
        let node = if let Some(n) = args.get("node").and_then(|v| v.as_str()) {
            n.to_string()
        } else if action != "create" {
            client
                .find_vm_location(
                    args.get("vmid")
                        .and_then(|v| v.as_i64())
                        .ok_or(anyhow::anyhow!("Missing vmid"))?,
                )
                .await?
                .0
        } else {
            anyhow::bail!("Missing node");
        };
        match action {
            "create" => {
                let mut p = args.as_object().unwrap().clone();
                p.remove("node");
                p.remove("action");
                p.remove("type");
                let res = client
                    .create_resource(&node, resource_type, &Value::Object(p))
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Create {} initiated. UPID: {}", resource_type, res) }] }),
                )
            }
            "delete" => {
                let vmid = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let res = client.delete_resource(&node, vmid, resource_type).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Delete {} initiated. UPID: {}", resource_type, res) }] }),
                )
            }
            "clone" => {
                let vmid = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let newid = args
                    .get("newid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing newid"))?;
                let res = client
                    .clone_resource(
                        &node,
                        vmid,
                        resource_type,
                        newid,
                        args.get("name").and_then(|v| v.as_str()),
                        args.get("target").and_then(|v| v.as_str()),
                        args.get("full").and_then(|v| v.as_bool()),
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Clone initiated. UPID: {}", res) }] }),
                )
            }
            "migrate" => {
                let vmid = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let target = args
                    .get("target_node")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing target_node"))?;
                let res = client
                    .migrate_resource(
                        &node,
                        vmid,
                        resource_type,
                        target,
                        args.get("online")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Migration initiated. UPID: {}", res) }] }),
                )
            }
            "template" => {
                let vmid = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let res = client.template_vm(&node, vmid).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Template created. UPID: {}", res) }] }),
                )
            }
            _ => anyhow::bail!("Unsupported action: {}", action),
        }
    }

    async fn handle_manage_resource_config(&self, args: &Value) -> Result<Value> {
        let vmid = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let client = self.get_client(args)?;
        let (node, rtype) = if let Some(n) = args.get("node").and_then(|v| v.as_str()) {
            (
                n.to_string(),
                args.get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("qemu")
                    .to_string(),
            )
        } else {
            client.find_vm_location(vmid).await?
        };
        match action {
            "update_resources" => self.handle_update_resources(args, &rtype).await,
            "add_disk" => {
                let dev = args
                    .get("device")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing device"))?;
                let stor = args
                    .get("storage")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing storage"))?;
                let size = args
                    .get("size_gb")
                    .and_then(|v| v.as_u64())
                    .ok_or(anyhow::anyhow!("Missing size_gb"))?;
                client
                    .add_virtual_disk(
                        &node,
                        vmid,
                        &rtype,
                        dev,
                        stor,
                        size,
                        args.get("format").and_then(|v| v.as_str()),
                        args.get("extra_options").and_then(|v| v.as_str()),
                    )
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Disk added" }] }))
            }
            "remove_disk" => {
                let dev = args
                    .get("device")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing device"))?;
                client.remove_virtual_disk(&node, vmid, &rtype, dev).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Disk removed" }] }))
            }
            "add_network" => {
                let dev = args
                    .get("device")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing device"))?;
                let bridge = args
                    .get("bridge")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing bridge"))?;
                client
                    .add_network_interface(
                        &node,
                        vmid,
                        &rtype,
                        dev,
                        args.get("model").and_then(|v| v.as_str()),
                        bridge,
                        args.get("mac").and_then(|v| v.as_str()),
                        args.get("extra_options").and_then(|v| v.as_str()),
                    )
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Network added" }] }))
            }
            "remove_network" => {
                let dev = args
                    .get("device")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing device"))?;
                client
                    .remove_network_interface(&node, vmid, &rtype, dev)
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Network removed" }] }))
            }
            "set_cloudinit" => {
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("node");
                p.remove("vmid");
                p.remove("type");
                client
                    .set_vm_cloudinit(&node, vmid, &Value::Object(p))
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Cloud-Init updated" }] }))
            }
            "exec" => {
                let cmd = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing command"))?;
                let cmd_vec: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
                let res = client
                    .agent_exec(
                        &node,
                        vmid,
                        &cmd_vec,
                        args.get("input_data").and_then(|v| v.as_str()),
                    )
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res)? }] }))
            }
            "add_lxc_mountpoint" => {
                let mp_id = args
                    .get("mp_id")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing mp_id"))?;
                let vol = args
                    .get("volume")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing volume"))?;
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing path"))?;
                client
                    .add_lxc_mountpoint(
                        &node,
                        vmid,
                        mp_id,
                        vol,
                        path,
                        args.get("read_only").and_then(|v| v.as_bool()),
                        args.get("backup").and_then(|v| v.as_bool()),
                        args.get("extra_options").and_then(|v| v.as_str()),
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Mount point {} added to CT {}", mp_id, vmid) }] }),
                )
            }
            "add_lxc_bind_mount" => {
                let mp_id = args
                    .get("mp_id")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing mp_id"))?;
                let src = args
                    .get("source")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing source"))?;
                let tgt = args
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing target"))?;
                client
                    .add_lxc_bind_mount(
                        &node,
                        vmid,
                        mp_id,
                        src,
                        tgt,
                        args.get("read_only").and_then(|v| v.as_bool()),
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Mount point {} added to CT {}", mp_id, vmid) }] }),
                )
            }
            "add_pci_device" => {
                let dev_id = args
                    .get("device_id")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing device_id"))?;
                let host = args
                    .get("host")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing host"))?;
                client
                    .add_pci_device(
                        &node,
                        vmid,
                        "qemu",
                        dev_id,
                        host,
                        args.get("pcie").and_then(|v| v.as_bool()),
                        args.get("mdev").and_then(|v| v.as_str()),
                        args.get("extra_options").and_then(|v| v.as_str()),
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("PCI device {} added", dev_id) }] }),
                )
            }
            "add_usb_device" => {
                let dev_id = args
                    .get("device_id")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing device_id"))?;
                let host = args
                    .get("host")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing host"))?;
                client
                    .add_usb_device(
                        &node,
                        vmid,
                        "qemu",
                        dev_id,
                        host,
                        args.get("usb3").and_then(|v| v.as_bool()),
                        args.get("extra_options").and_then(|v| v.as_str()),
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("USB device {} added", dev_id) }] }),
                )
            }
            "remove_vm_device" => {
                let dev_id = args
                    .get("device_id")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing device_id"))?;
                client.remove_vm_device(&node, vmid, "qemu", dev_id).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Device {} removed", dev_id) }] }),
                )
            }
            _ => anyhow::bail!("Unsupported config action"),
        }
    }

    async fn handle_manage_snapshot_backup(&self, args: &Value) -> Result<Value> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let client = self.get_client(args)?;
        match action {
            "snapshot_create" => {
                let vmid = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let res = client
                    .create_snapshot(
                        node,
                        vmid,
                        args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu"),
                        args.get("snapname")
                            .and_then(|v| v.as_str())
                            .ok_or(anyhow::anyhow!("Missing snapname"))?,
                        args.get("description").and_then(|v| v.as_str()),
                        args.get("vmstate")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Snapshot created: {}", res) }] }),
                )
            }
            "snapshot_rollback" => {
                let vmid = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let res = client
                    .rollback_snapshot(
                        node,
                        vmid,
                        args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu"),
                        args.get("snapname")
                            .and_then(|v| v.as_str())
                            .ok_or(anyhow::anyhow!("Missing snapname"))?,
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Rollback initiated: {}", res) }] }),
                )
            }
            "snapshot_delete" => {
                let vmid = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let res = client
                    .delete_snapshot(
                        node,
                        vmid,
                        args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu"),
                        args.get("snapname")
                            .and_then(|v| v.as_str())
                            .ok_or(anyhow::anyhow!("Missing snapname"))?,
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Delete initiated: {}", res) }] }),
                )
            }
            "backup_create" => {
                let vmid = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let res = client
                    .create_backup(
                        node,
                        vmid,
                        args.get("storage").and_then(|v| v.as_str()),
                        args.get("mode").and_then(|v| v.as_str()),
                        args.get("compress").and_then(|v| v.as_str()),
                        args.get("remove").and_then(|v| v.as_bool()),
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Backup initiated: {}", res) }] }),
                )
            }
            "backup_restore" => {
                let vmid = args
                    .get("vmid")
                    .and_then(|v| v.as_i64())
                    .ok_or(anyhow::anyhow!("Missing vmid"))?;
                let rtype = args
                    .get("type")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing type"))?;
                let arch = args
                    .get("archive")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing archive"))?;
                let res = client
                    .restore_backup(
                        node,
                        vmid,
                        rtype,
                        arch,
                        args.get("storage").and_then(|v| v.as_str()),
                        args.get("force").and_then(|v| v.as_bool()),
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Restore initiated: {}", res) }] }),
                )
            }
            _ => self.handle_manage_backup_schedule(args).await,
        }
    }

    async fn handle_manage_node_system(&self, args: &Value) -> Result<Value> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let client = self.get_client(args)?;
        match action {
            "manage_service" => {
                let s = args
                    .get("service")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing service"))?;
                let sa = args
                    .get("service_action")
                    .and_then(|v| v.as_str())
                    .or(args.get("action").and_then(|v| v.as_str()))
                    .ok_or(anyhow::anyhow!("Missing service_action"))?;
                let res = client.manage_service(node, s, sa).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Service {} {} initiated: {}", s, sa, res) }] }),
                )
            }
            "apt_update" => {
                let res = client.run_apt_update(node).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("APT update initiated: {}", res) }] }),
                )
            }
            "cert_renew" => {
                let res = client.renew_acme_certificate(node).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Cert renewal initiated: {}", res) }] }),
                )
            }
            "upload_certificate" => self.handle_upload_certificate(args).await,
            "add_repository" => self.handle_add_repository(args).await,
            "update_repository_state" => self.handle_update_repository_state(args).await,
            "set_subscription_key" => self.handle_set_subscription_key(args).await,
            _ => anyhow::bail!("Unsupported node action"),
        }
    }

    async fn handle_manage_cluster_config(&self, args: &Value) -> Result<Value> {
        let t = args
            .get("resource_type")
            .and_then(|v| v.as_str())
            .or_else(|| args.get("type").and_then(|v| v.as_str()))
            .ok_or(anyhow::anyhow!("Missing type or resource_type"))?;
        match t {
            "storage" | "dir" | "nfs" | "cifs" | "lvm" | "lvmthin" | "zfs" | "iscsi" | "rbd"
            | "cephfs" | "storage_content" => self.handle_manage_storage(args).await,
            "sdn" | "simple" | "vlan" | "qinq" | "vxlan" | "evpn" | "zone" | "vnet" => {
                self.handle_manage_sdn_resource(args).await
            }
            "firewall_alias" => self.handle_manage_firewall_alias(args).await,
            "security_group" => self.handle_manage_security_group(args).await,
            "pool" => self.handle_manage_pool(args).await,
            "role" => self.handle_manage_role(args).await,
            "replication" => self.handle_manage_replication(args).await,
            "ha" => self.handle_manage_ha_resource(args).await,
            "mapping" | "pci" | "usb" => self.handle_manage_mapping(args).await,
            "metric" | "influxdb" | "graphite" => self.handle_manage_metric_server(args).await,
            "ceph" => self.handle_manage_cluster_config_ceph(args).await,
            "user" => self.handle_manage_user(args).await,
            "acl" => self.handle_update_acl(args).await,
            _ => anyhow::bail!("Unsupported cluster type"),
        }
    }

    async fn handle_manage_cluster_config_ceph(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        match a {
            "create" => self.handle_create_ceph_pool(args).await,
            "delete" => self.handle_delete_ceph_pool(args).await,
            _ => anyhow::bail!("Unsupported ceph action"),
        }
    }

    async fn handle_manage_user(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        match a {
            "create" => self.handle_create_user(args).await,
            "delete" => self.handle_delete_user(args).await,
            _ => anyhow::bail!("Unsupported user action"),
        }
    }

    async fn handle_manage_firewall_alias(&self, args: &Value) -> Result<Value> {
        let l = args
            .get("level")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing level"))?;
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let n = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let node = args.get("node").and_then(|v| v.as_str());
        let client = self.get_client(args)?;
        match a {
            "create" | "update" => {
                let c = args
                    .get("cidr")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing cidr"))?;
                let comm = args.get("comment").and_then(|v| v.as_str());
                if a == "create" {
                    client.create_alias(l, node, n, c, comm).await?;
                } else {
                    client.update_alias(l, node, n, c, comm).await?;
                }
                Ok(json!({ "content": [{ "type": "text", "text": format!("Alias {} {}", n, a) }] }))
            }
            "delete" => {
                client.delete_alias(l, node, n).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Alias {} deleted", n) }] }),
                )
            }
            _ => anyhow::bail!("Unsupported alias action"),
        }
    }

    async fn handle_manage_security_group(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let n = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let client = self.get_client(args)?;
        match a {
            "create" => {
                client
                    .create_security_group(n, args.get("comment").and_then(|v| v.as_str()))
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Security group {} created", n) }] }),
                )
            }
            "delete" => {
                client.delete_security_group(n).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Security group {} deleted", n) }] }),
                )
            }
            "add" | "update" => {
                let mut rule = args.as_object().unwrap().clone();
                rule.remove("action");
                if let Some(ra) = rule.remove("rule_action") {
                    rule.insert("action".to_string(), ra);
                }
                rule.remove("name");
                rule.remove("type");
                rule.remove("resource_type");
                client
                    .add_security_group_rule(n, &Value::Object(rule))
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Rule added to group {}", n) }] }),
                )
            }
            _ => anyhow::bail!("Unsupported security group action"),
        }
    }

    async fn handle_manage_sdn_resource(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let r = args
            .get("resource_type")
            .and_then(|v| v.as_str())
            .or_else(|| {
                let t = args.get("type").and_then(|v| v.as_str())?;
                if ![
                    "sdn",
                    "storage",
                    "mapping",
                    "metric",
                    "ceph",
                    "firewall_alias",
                    "security_group",
                    "pool",
                    "role",
                    "replication",
                    "ha",
                    "user",
                    "acl",
                ]
                .contains(&t)
                {
                    Some(t)
                } else {
                    None
                }
            })
            .unwrap_or("zone");
        let client = self.get_client(args)?;
        match (r, a) {
            ("zone", "create")
            | ("simple", "create")
            | ("vlan", "create")
            | ("qinq", "create")
            | ("vxlan", "create")
            | ("evpn", "create") => {
                let z = args
                    .get("zone")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing zone"))?;
                let zt = args
                    .get("zone_type")
                    .and_then(|v| v.as_str())
                    .or(args.get("type").and_then(|v| v.as_str()))
                    .ok_or(anyhow::anyhow!("Missing zone_type"))?;
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("type");
                p.remove("resource_type");
                p.remove("zone");
                p.remove("zone_type");
                client.create_sdn_zone(z, zt, &Value::Object(p)).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("SDN Zone {} created", z) }] }),
                )
            }
            ("zone", "delete") => {
                client
                    .delete_sdn_zone(
                        args.get("zone")
                            .and_then(|v| v.as_str())
                            .ok_or(anyhow::anyhow!("Missing zone"))?,
                    )
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": "SDN Zone deleted" }] }))
            }
            ("vnet", "create") => {
                let v = args
                    .get("vnet")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing vnet"))?;
                let z = args
                    .get("zone")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing zone"))?;
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("type");
                p.remove("resource_type");
                p.remove("vnet");
                p.remove("zone");
                client.create_sdn_vnet(v, z, &Value::Object(p)).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("SDN Vnet {} created", v) }] }),
                )
            }
            ("vnet", "delete") => {
                client
                    .delete_sdn_vnet(
                        args.get("vnet")
                            .and_then(|v| v.as_str())
                            .ok_or(anyhow::anyhow!("Missing vnet"))?,
                    )
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": "SDN Vnet deleted" }] }))
            }
            _ => anyhow::bail!("Unsupported sdn action/type"),
        }
    }

    async fn handle_manage_storage(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let s = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;
        let client = self.get_client(args)?;
        match a {
            "add" => {
                let st = args
                    .get("storage_type")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        let t = args.get("type").and_then(|v| v.as_str())?;
                        if t != "storage" {
                            Some(t)
                        } else {
                            None
                        }
                    })
                    .ok_or(anyhow::anyhow!("Missing storage type"))?;
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("storage");
                p.remove("type");
                p.remove("storage_type");
                client
                    .add_storage(
                        s,
                        st,
                        args.get("content").and_then(|v| v.as_str()),
                        args.get("nodes").and_then(|v| {
                            v.as_array().map(|al| {
                                al.iter()
                                    .filter_map(|x| x.as_str().map(String::from))
                                    .collect()
                            })
                        }),
                        args.get("enable").and_then(|v| v.as_bool()),
                        Some(&p),
                    )
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Storage {} added", s) }] }),
                )
            }
            "update" => {
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("storage");
                client.update_storage(s, &p).await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("Storage {} updated", s) }] }),
                )
            }
            "delete" => {
                if args.get("type").and_then(|v| v.as_str()) == Some("storage_content") {
                    let n = args
                        .get("node")
                        .and_then(|v| v.as_str())
                        .ok_or(anyhow::anyhow!("Missing node"))?;
                    let vol = args
                        .get("volume")
                        .and_then(|v| v.as_str())
                        .ok_or(anyhow::anyhow!("Missing volume"))?;
                    client.delete_storage_content(n, s, vol).await?;
                    Ok(
                        json!({ "content": [{ "type": "text", "text": format!("Volume {} deleted", vol) }] }),
                    )
                } else {
                    client.delete_storage(s).await?;
                    Ok(
                        json!({ "content": [{ "type": "text", "text": format!("Storage {} deleted", s) }] }),
                    )
                }
            }
            _ => anyhow::bail!("Unsupported storage action"),
        }
    }

    async fn handle_manage_tags(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let id = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let r = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let t = args
            .get("tags")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing tags"))?;
        let client = self.get_client(args)?;
        match a {
            "add" => client.add_tag(n, id, r, t).await?,
            "remove" => client.remove_tag(n, id, r, t).await?,
            "set" => client.set_tags(n, id, r, t).await?,
            _ => anyhow::bail!("Unsupported tags action"),
        }
        Ok(json!({ "content": [{ "type": "text", "text": format!("Tags {} successful", a) }] }))
    }

    async fn handle_manage_pool(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let p = args
            .get("poolid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing poolid"))?;
        let client = self.get_client(args)?;
        match a {
            "create" => {
                client
                    .create_pool(p, args.get("comment").and_then(|v| v.as_str()))
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Pool created" }] }))
            }
            "update" => {
                let mut pa = args.as_object().unwrap().clone();
                pa.remove("action");
                pa.remove("poolid");
                client.update_pool(p, &Value::Object(pa)).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Pool updated" }] }))
            }
            "delete" => {
                client.delete_pool(p).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Pool deleted" }] }))
            }
            _ => anyhow::bail!("Unsupported pool action"),
        }
    }

    async fn handle_manage_role(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let r = args
            .get("roleid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing roleid"))?;
        let client = self.get_client(args)?;
        match a {
            "create" | "update" => {
                let pr = args
                    .get("privs")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing privs"))?;
                if a == "create" {
                    client.create_role(r, pr).await?;
                } else {
                    client
                        .update_role(
                            r,
                            pr,
                            args.get("append")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                        )
                        .await?;
                }
                Ok(json!({ "content": [{ "type": "text", "text": format!("Role {} {}", r, a) }] }))
            }
            "delete" => {
                client.delete_role(r).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Role deleted" }] }))
            }
            _ => anyhow::bail!("Unsupported role action"),
        }
    }

    async fn handle_manage_replication(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let client = self.get_client(args)?;
        match a {
            "create" => {
                let t = args
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing target"))?;
                client
                    .create_replication_job(
                        id,
                        t,
                        args.get("schedule").and_then(|v| v.as_str()),
                        args.get("rate").and_then(|v| v.as_f64()),
                        args.get("comment").and_then(|v| v.as_str()),
                        args.get("enable").and_then(|v| v.as_bool()),
                    )
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Replication created" }] }))
            }
            "update" => {
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("id");
                client.update_replication_job(id, &Value::Object(p)).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Replication updated" }] }))
            }
            "delete" => {
                client.delete_replication_job(id).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Replication deleted" }] }))
            }
            _ => anyhow::bail!("Unsupported replication action"),
        }
    }

    async fn handle_manage_ha_resource(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let s = args
            .get("sid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing sid"))?;
        let client = self.get_client(args)?;
        match a {
            "add" => {
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("sid");
                client.add_ha_resource(s, &Value::Object(p)).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "HA resource added" }] }))
            }
            "update" => {
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("sid");
                client.update_ha_resource(s, &Value::Object(p)).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "HA resource updated" }] }))
            }
            "delete" => {
                client.delete_ha_resource(s).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "HA resource removed" }] }))
            }
            _ => anyhow::bail!("Unsupported ha action"),
        }
    }

    async fn handle_manage_mapping(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let t = args
            .get("resource_type")
            .and_then(|v| v.as_str())
            .or_else(|| {
                let t = args.get("type").and_then(|v| v.as_str())?;
                if t != "mapping" {
                    Some(t)
                } else {
                    None
                }
            })
            .ok_or(anyhow::anyhow!("Missing resource type"))?;
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let client = self.get_client(args)?;
        match (t, a) {
            ("pci", "create") | ("pci", "update") => {
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("type");
                p.remove("resource_type");
                p.remove("id");
                if a == "create" {
                    client.create_pci_mapping(id, &Value::Object(p)).await?;
                } else {
                    client.update_pci_mapping(id, &Value::Object(p)).await?;
                }
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("PCI Mapping {} {}", id, a) }] }),
                )
            }
            ("pci", "delete") => {
                client.delete_pci_mapping(id).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "PCI Mapping deleted" }] }))
            }
            ("usb", "create") | ("usb", "update") => {
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("type");
                p.remove("resource_type");
                p.remove("id");
                if a == "create" {
                    client.create_usb_mapping(id, &Value::Object(p)).await?;
                } else {
                    client.update_usb_mapping(id, &Value::Object(p)).await?;
                }
                Ok(
                    json!({ "content": [{ "type": "text", "text": format!("USB Mapping {} {}", id, a) }] }),
                )
            }
            ("usb", "delete") => {
                client.delete_usb_mapping(id).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "USB Mapping deleted" }] }))
            }
            _ => anyhow::bail!("Unsupported mapping action/type"),
        }
    }

    async fn handle_manage_metric_server(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let client = self.get_client(args)?;
        match a {
            "create" => {
                let st = args
                    .get("server_type")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        let t = args.get("type").and_then(|v| v.as_str())?;
                        if t != "metric" {
                            Some(t)
                        } else {
                            None
                        }
                    })
                    .ok_or(anyhow::anyhow!("Missing server_type"))?;
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("id");
                p.remove("type");
                p.remove("server_type");
                client
                    .create_metric_server(id, st, &Value::Object(p))
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Metric server created" }] }))
            }
            "update" => {
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("id");
                client.update_metric_server(id, &Value::Object(p)).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Metric server updated" }] }))
            }
            "delete" => {
                client.delete_metric_server(id).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Metric server deleted" }] }))
            }
            _ => anyhow::bail!("Unsupported metric action"),
        }
    }

    async fn handle_manage_backup_schedule(&self, args: &Value) -> Result<Value> {
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let client = self.get_client(args)?;
        match a {
            "create" => {
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                client.create_backup_schedule(&Value::Object(p)).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Backup schedule created" }] }))
            }
            "update" => {
                let id = args
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing id"))?;
                let mut p = args.as_object().unwrap().clone();
                p.remove("action");
                p.remove("id");
                client.update_backup_schedule(id, &Value::Object(p)).await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Backup schedule updated" }] }))
            }
            "delete" => {
                client
                    .delete_backup_schedule(
                        args.get("id")
                            .and_then(|v| v.as_str())
                            .ok_or(anyhow::anyhow!("Missing id"))?,
                    )
                    .await?;
                Ok(json!({ "content": [{ "type": "text", "text": "Backup schedule deleted" }] }))
            }
            _ => anyhow::bail!("Unsupported backup schedule action"),
        }
    }

    // --- Listing Handlers ---

    async fn handle_list_storage(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let s = self.get_client(args)?.get_storage_list(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_list_cluster_storage(&self, args: &Value) -> Result<Value> {
        let s = self.get_client(args)?.get_cluster_storage().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_list_networks(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let net = self.get_client(args)?.get_network_interfaces(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&net)? }] }))
    }

    async fn handle_list_firewall_rules(&self, args: &Value) -> Result<Value> {
        let n = args.get("node").and_then(|v| v.as_str());
        let id = args.get("vmid").and_then(|v| v.as_i64());
        let r = self.get_client(args)?.get_firewall_rules(n, id).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&r)? }] }))
    }

    async fn handle_list_firewall_aliases(&self, args: &Value) -> Result<Value> {
        let l = args
            .get("level")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing level"))?;
        let n = args.get("node").and_then(|v| v.as_str());
        let al = self.get_client(args)?.get_aliases(l, n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&al)? }] }))
    }

    async fn handle_list_security_groups(&self, args: &Value) -> Result<Value> {
        let g = self.get_client(args)?.get_security_groups().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&g)? }] }))
    }

    async fn handle_list_security_group_rules(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let r = self.get_client(args)?.get_security_group_rules(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&r)? }] }))
    }

    async fn handle_list_tasks(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let t = self
            .get_client(args)?
            .list_tasks(n, args.get("limit").and_then(|v| v.as_u64()))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&t)? }] }))
    }

    async fn handle_list_backups(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let s = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;
        let b = self
            .get_client(args)?
            .get_backups(n, s, args.get("vmid").and_then(|v| v.as_i64()))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&b)? }] }))
    }

    async fn handle_snapshot_list(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let id = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let s = self
            .get_client(args)?
            .get_snapshots(
                n,
                id,
                args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu"),
            )
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_list_pools(&self, args: &Value) -> Result<Value> {
        let p = self.get_client(args)?.get_pools().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&p)? }] }))
    }

    async fn handle_list_replication_jobs(&self, args: &Value) -> Result<Value> {
        let j = self.get_client(args)?.get_replication_jobs().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&j)? }] }))
    }

    async fn handle_list_ha_resources(&self, args: &Value) -> Result<Value> {
        let r = self.get_client(args)?.get_ha_resources().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&r)? }] }))
    }

    async fn handle_list_ha_groups(&self, args: &Value) -> Result<Value> {
        let g = self.get_client(args)?.get_ha_groups().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&g)? }] }))
    }

    async fn handle_list_users(&self, args: &Value) -> Result<Value> {
        let u = self.get_client(args)?.get_users().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&u)? }] }))
    }

    async fn handle_list_roles(&self, args: &Value) -> Result<Value> {
        let r = self.get_client(args)?.get_roles().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&r)? }] }))
    }

    async fn handle_list_acls(&self, args: &Value) -> Result<Value> {
        let a = self.get_client(args)?.get_acls().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&a)? }] }))
    }

    async fn handle_list_apt_updates(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let u = self.get_client(args)?.get_apt_updates(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&u)? }] }))
    }

    async fn handle_list_services(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let s = self.get_client(args)?.get_services(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_list_certificates(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let c = self.get_client(args)?.get_certificates(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&c)? }] }))
    }

    async fn handle_list_pci_devices(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let d = self.get_client(args)?.get_pci_devices(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&d)? }] }))
    }

    async fn handle_list_usb_devices(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let d = self.get_client(args)?.get_usb_devices(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&d)? }] }))
    }

    async fn handle_list_pci_mappings(&self, args: &Value) -> Result<Value> {
        let m = self.get_client(args)?.get_pci_mappings().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&m)? }] }))
    }

    async fn handle_list_usb_mappings(&self, args: &Value) -> Result<Value> {
        let m = self.get_client(args)?.get_usb_mappings().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&m)? }] }))
    }

    async fn handle_list_metric_servers(&self, args: &Value) -> Result<Value> {
        let s = self.get_client(args)?.get_metric_servers().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_list_sdn_zones(&self, args: &Value) -> Result<Value> {
        let z = self.get_client(args)?.get_sdn_zones().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&z)? }] }))
    }

    async fn handle_list_sdn_vnets(&self, args: &Value) -> Result<Value> {
        let v = self.get_client(args)?.get_sdn_vnets().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&v)? }] }))
    }

    async fn handle_list_ceph_pools(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let p = self.get_client(args)?.get_ceph_pools(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&p)? }] }))
    }

    async fn handle_list_ceph_osds(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let o = self.get_client(args)?.get_ceph_osds(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&o)? }] }))
    }

    async fn handle_list_ceph_monitors(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let m = self.get_client(args)?.get_ceph_monitors(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&m)? }] }))
    }

    async fn handle_list_backup_schedules(&self, args: &Value) -> Result<Value> {
        let s = self.get_client(args)?.get_backup_schedules().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_list_repositories(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let r = self.get_client(args)?.get_repositories(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&r)? }] }))
    }

    async fn handle_list_isos(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let s = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;
        let i = self
            .get_client(args)?
            .get_storage_content(n, s, Some("iso"))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&i)? }] }))
    }

    // --- Legacy Handlers (kept for internal use) ---

    async fn handle_get_cluster_status(&self, args: &Value) -> Result<Value> {
        let s = self.get_client(args)?.get_cluster_status().await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_get_cluster_log(&self, args: &Value) -> Result<Value> {
        let l = self
            .get_client(args)?
            .get_cluster_log(args.get("limit").and_then(|v| v.as_u64()))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&l)? }] }))
    }

    async fn handle_get_node_stats(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let s = self
            .get_client(args)?
            .get_node_stats(
                n,
                args.get("timeframe").and_then(|v| v.as_str()),
                args.get("cf").and_then(|v| v.as_str()),
            )
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_get_vm_stats(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let id = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let s = self
            .get_client(args)?
            .get_resource_stats(
                n,
                id,
                args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu"),
                args.get("timeframe").and_then(|v| v.as_str()),
                args.get("cf").and_then(|v| v.as_str()),
            )
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_get_vm_config(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let id = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let c = self
            .get_client(args)?
            .get_vm_config(
                n,
                id,
                args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu"),
            )
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&c)? }] }))
    }

    async fn handle_get_task_status(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let u = args
            .get("upid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing upid"))?;
        let s = self.get_client(args)?.get_task_status(n, u).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_read_task_log(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let u = args
            .get("upid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing upid"))?;
        let l = self.get_client(args)?.get_task_log(n, u).await?;
        let mut text = String::new();
        for entry in l {
            if let Some(line) = entry.get("t").and_then(|v| v.as_str()) {
                text.push_str(line);
                text.push('\n');
            }
        }
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_storage_volume(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let s = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;
        let vo = args
            .get("volume")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing volume"))?;
        let i = self
            .get_client(args)?
            .get_storage_content_volume(n, s, vo)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&i)? }] }))
    }

    async fn handle_get_pool_details(&self, args: &Value) -> Result<Value> {
        let p = args
            .get("poolid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing poolid"))?;
        let d = self.get_client(args)?.get_pool_details(p).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&d)? }] }))
    }

    async fn handle_get_subscription_info(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let i = self.get_client(args)?.get_subscription(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&i)? }] }))
    }

    async fn handle_get_apt_versions(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let v = self.get_client(args)?.get_apt_versions(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&v)? }] }))
    }

    async fn handle_get_ceph_status(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let s = self.get_client(args)?.get_ceph_status(n).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&s)? }] }))
    }

    async fn handle_bulk_vm_action(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let ids = args
            .get("vmids")
            .and_then(|v| {
                v.as_array()
                    .map(|al| al.iter().filter_map(|x| x.as_i64()).collect::<Vec<i64>>())
            })
            .ok_or(anyhow::anyhow!("Missing vmids"))?;
        let a = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;
        let res = self
            .get_client(args)?
            .bulk_vm_action(n, ids, a, args.get("type").and_then(|v| v.as_str()))
            .await?;
        let mut report = Vec::new();
        for (id, r) in res {
            match r {
                Ok(upid) => report.push(format!("VM {}: Success ({})", id, upid)),
                Err(e) => report.push(format!("VM {}: Failed ({})", id, e)),
            }
        }
        Ok(json!({ "content": [{ "type": "text", "text": report.join("\n") }] }))
    }

    async fn handle_scan_storage_remote(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let t = args
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing type"))?;
        let s = args
            .get("server")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing server"))?;
        let res = self
            .get_client(args)?
            .scan_storage(
                n,
                t,
                s,
                args.get("username").and_then(|v| v.as_str()),
                args.get("password").and_then(|v| v.as_str()),
            )
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res)? }] }))
    }

    async fn handle_wait_for_task(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let u = args
            .get("upid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing upid"))?;
        let s = self
            .get_client(args)?
            .wait_for_task(
                n,
                u,
                args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(60),
            )
            .await?;
        let es = s
            .get("exitstatus")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Task finished: {}\nFull details:\n{}", es, serde_json::to_string(&s)?) }] }),
        )
    }

    async fn handle_apply_sdn_changes(&self, args: &Value) -> Result<Value> {
        let u = self.get_client(args)?.apply_sdn().await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("SDN changes applied: {}", u) }] }),
        )
    }

    async fn handle_vm_agent_ping(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let id = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        self.get_client(args)?.agent_ping(n, id).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Pong" }] }))
    }

    async fn handle_vm_exec_status(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let id = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let p = args
            .get("pid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing pid"))?;
        let res = self.get_client(args)?.agent_exec_status(n, id, p).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res)? }] }))
    }

    async fn handle_vm_read_file(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let id = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let f = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing file"))?;
        let res = self.get_client(args)?.agent_file_read(n, id, f).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res)? }] }))
    }

    async fn handle_vm_write_file(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let id = args
            .get("vmid")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing vmid"))?;
        let f = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing file"))?;
        let c = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing content"))?;
        self.get_client(args)?
            .agent_file_write(n, id, f, c, args.get("encode").and_then(|v| v.as_bool()))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "File written" }] }))
    }

    async fn handle_download_url(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let s = args
            .get("storage")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing storage"))?;
        let u = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing url"))?;
        let f = args
            .get("filename")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing filename"))?;
        let c = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing content"))?;
        let res = self
            .get_client(args)?
            .download_url(
                n,
                s,
                u,
                f,
                c,
                args.get("checksum").and_then(|v| v.as_str()),
                args.get("checksum_algorithm").and_then(|v| v.as_str()),
            )
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Download initiated: {}", res) }] }),
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

    async fn handle_upload_certificate(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let c = args
            .get("certificates")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing certificates"))?;
        let k = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing key"))?;
        self.get_client(args)?
            .upload_certificate(
                n,
                c,
                k,
                args.get("force").and_then(|v| v.as_bool()),
                args.get("restart").and_then(|v| v.as_bool()),
            )
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Cert uploaded" }] }))
    }

    async fn handle_add_repository(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let h = args
            .get("handle")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing handle"))?;
        self.get_client(args)?.add_repository(n, h).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Repo {} added", h) }] }))
    }

    async fn handle_update_repository_state(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let p = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing path"))?;
        let i = args
            .get("index")
            .and_then(|v| v.as_u64())
            .ok_or(anyhow::anyhow!("Missing index"))? as usize;
        let e = args
            .get("enabled")
            .and_then(|v| v.as_bool())
            .ok_or(anyhow::anyhow!("Missing enabled"))?;
        self.get_client(args)?
            .change_repository_state(n, p, i, e)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Repo state updated" }] }))
    }

    async fn handle_set_subscription_key(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let k = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing key"))?;
        self.get_client(args)?.set_subscription(n, k).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Sub key set" }] }))
    }

    async fn handle_create_user(&self, args: &Value) -> Result<Value> {
        let u = args
            .get("userid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing userid"))?;
        let p = args
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing password"))?;
        self.get_client(args)?
            .create_user(
                u,
                p,
                args.get("email").and_then(|v| v.as_str()),
                args.get("firstname").and_then(|v| v.as_str()),
                args.get("lastname").and_then(|v| v.as_str()),
                args.get("expire").and_then(|v| v.as_i64()),
                args.get("enable").and_then(|v| v.as_bool()),
                args.get("comment").and_then(|v| v.as_str()),
                args.get("groups").and_then(|v| {
                    v.as_array().map(|al| {
                        al.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                }),
            )
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("User {} created", u) }] }))
    }

    async fn handle_delete_user(&self, args: &Value) -> Result<Value> {
        let u = args
            .get("userid")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing userid"))?;
        self.get_client(args)?.delete_user(u).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("User {} deleted", u) }] }))
    }

    async fn handle_update_acl(&self, args: &Value) -> Result<Value> {
        let p = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing path"))?;
        let mut params = args.as_object().unwrap().clone();
        params.remove("path");
        self.get_client(args)?
            .update_acl(p, &Value::Object(params))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "ACL updated" }] }))
    }

    async fn handle_create_ceph_pool(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let mut p = args.as_object().unwrap().clone();
        p.remove("node");
        p.remove("name");
        let u = self
            .get_client(args)?
            .create_ceph_pool(n, name, &Value::Object(p))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Ceph pool created: {}", u) }] }))
    }

    async fn handle_delete_ceph_pool(&self, args: &Value) -> Result<Value> {
        let n = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let u = self
            .get_client(args)?
            .delete_ceph_pool(
                n,
                name,
                args.get("remove_storages")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            )
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Ceph pool deleted: {}", u) }] }))
    }
}
