use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use log::{error, info, debug};
use crate::proxmox::ProxmoxClient;

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

#[derive(Clone)]
pub struct McpServer {
    client: ProxmoxClient,
}

impl McpServer {
    pub fn new(client: ProxmoxClient) -> Self {
        Self { client }
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
                            Err(e) => JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: Some(req_id),
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32603, // Internal error
                                    message: e.to_string(),
                                    data: None,
                                }),
                            },
                        };
                        
                        let out = serde_json::to_string(&json_resp)?;
                        println!("{}", out);
                        io::stdout().flush()?;
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
            "initialize" => {
                Ok(json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "proxmox-mcp-rs",
                        "version": "0.1.0"
                    },
                    "capabilities": {
                        "tools": {},
                        "resources": {}
                    }
                }))
            },
            "notifications/initialized" => {
                info!("Client initialized");
                Ok(Value::Null)
            },
            "ping" => Ok(json!({})),
            "tools/list" => {
                Ok(json!({
                    "tools": self.get_tool_definitions()
                }))
            },
            "tools/call" => {
                if let Some(params) = req.params {
                    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let args = params.get("arguments").unwrap_or(&Value::Null);
                    self.call_tool(name, args).await
                } else {
                     anyhow::bail!("Missing params for tools/call");
                }
            },
            _ => {
                // Ignore unknown methods or return error?
                // For MCP, unknown methods should probably be ignored if they are notifications, 
                // or error if request.
                anyhow::bail!("Method not found: {}", req.method);
            }
        }
    }

    fn get_tool_definitions(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_nodes",
                "description": "List all nodes in the Proxmox cluster",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "list_vms",
                "description": "List all VMs and containers across all nodes",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "list_containers",
                "description": "List all LXC containers across all nodes",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "start_vm",
                "description": "Start a VM or container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "The node name" },
                        "vmid": { "type": "integer", "description": "The VM ID" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"], "description": "Type: qemu or lxc (optional, defaults to qemu if not found)" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "start_container",
                "description": "Start an LXC container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "The node name" },
                        "vmid": { "type": "integer", "description": "The Container ID" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "stop_vm",
                "description": "Stop (power off) a VM or container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "The node name" },
                        "vmid": { "type": "integer", "description": "The VM ID" },
                         "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "stop_container",
                "description": "Stop (power off) an LXC container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "The node name" },
                        "vmid": { "type": "integer", "description": "The Container ID" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "shutdown_vm",
                "description": "Gracefully shutdown a VM or container",
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
                "description": "Gracefully shutdown an LXC container",
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
                "description": "Reboot a VM or container",
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
                "description": "Create a new QEMU VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "Target node" },
                        "vmid": { "type": "integer", "description": "VM ID" },
                        "name": { "type": "string", "description": "VM Name" },
                        "memory": { "type": "integer", "description": "Memory in MB" },
                        "cores": { "type": "integer", "description": "Number of cores" },
                        "sockets": { "type": "integer", "description": "Number of sockets" },
                        "net0": { "type": "string", "description": "Network config (e.g. 'virtio,bridge=vmbr0')" },
                        "ide2": { "type": "string", "description": "CDROM/ISO (e.g. 'local:iso/debian.iso')" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "create_container",
                "description": "Create a new LXC Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "Target node" },
                        "vmid": { "type": "integer", "description": "VM ID" },
                        "ostemplate": { "type": "string", "description": "OS Template (e.g. 'local:vztmpl/ubuntu-20.04...')" },
                        "hostname": { "type": "string", "description": "Hostname" },
                        "password": { "type": "string", "description": "Root password" },
                        "memory": { "type": "integer", "description": "Memory in MB" },
                        "cores": { "type": "integer", "description": "Number of cores" },
                        "rootfs": { "type": "string", "description": "Rootfs config (e.g. 'local-lvm:8')" }
                    },
                    "required": ["node", "vmid", "ostemplate"]
                }
            }),
            json!({
                "name": "delete_vm",
                "description": "Delete a QEMU VM",
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
                "description": "Delete an LXC Container",
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
                "description": "Reset (Stop and Start) a VM",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "vm_id": { "type": "string", "description": "The VM ID" }
                    },
                    "required": ["vm_id"]
                }
            }),
            json!({
                "name": "reset_container",
                "description": "Reset (Stop and Start) a Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "container_id": { "type": "string", "description": "The Container ID" }
                    },
                    "required": ["container_id"]
                }
            }),
            json!({
                "name": "list_templates",
                "description": "List container templates on a storage",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "The node name" },
                        "storage": { "type": "string", "description": "Storage name (default: local)" },
                        "content": { "type": "string", "description": "Content type (default: vztmpl)" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "update_container_resources",
                "description": "Update LXC container resources (cores, memory, swap, disk)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "The node name" },
                        "vmid": { "type": "integer", "description": "The Container ID" },
                        "cores": { "type": "integer", "description": "New core count" },
                        "memory": { "type": "integer", "description": "New memory (MB)" },
                        "swap": { "type": "integer", "description": "New swap (MB)" },
                        "disk_gb": { "type": "integer", "description": "Additional disk size in GB to add (e.g. 2 for +2G)" },
                        "disk": { "type": "string", "description": "Disk to resize (default: rootfs)" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "list_snapshots",
                "description": "List snapshots for a VM or Container",
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
                "description": "Create a snapshot of a VM or Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "snapname": { "type": "string", "description": "Snapshot name" },
                        "description": { "type": "string", "description": "Snapshot description" },
                        "vmstate": { "type": "boolean", "description": "Save RAM content (only for QEMU)" },
                         "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid", "snapname"]
                }
            }),
            json!({
                "name": "rollback_vm",
                "description": "Rollback a VM or Container to a snapshot",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "snapname": { "type": "string", "description": "Snapshot name" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid", "snapname"]
                }
            }),
            json!({
                "name": "delete_snapshot",
                "description": "Delete a snapshot of a VM or Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "snapname": { "type": "string", "description": "Snapshot name" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid", "snapname"]
                }
            }),
            json!({
                "name": "clone_vm",
                "description": "Clone a VM or Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "Source node" },
                        "vmid": { "type": "integer", "description": "Source VM ID" },
                        "newid": { "type": "integer", "description": "New VM ID" },
                        "name": { "type": "string", "description": "New VM Name (optional)" },
                        "target": { "type": "string", "description": "Target node (optional)" },
                        "full": { "type": "boolean", "description": "Full clone (default: true)" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid", "newid"]
                }
            }),
             json!({
                "name": "migrate_vm",
                "description": "Migrate a VM or Container to another node",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "Source node" },
                        "vmid": { "type": "integer", "description": "VM ID" },
                        "target_node": { "type": "string", "description": "Target node" },
                        "online": { "type": "boolean", "description": "Online migration (default: false)" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] }
                    },
                    "required": ["node", "vmid", "target_node"]
                }
            })
        ]
    }

    pub async fn call_tool(&self, name: &str, args: &Value) -> Result<Value> {
        match name {
            "list_nodes" => {
                let nodes = self.client.get_nodes().await?;
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&nodes)? }] }))
            },
            "list_vms" => {
                let vms = self.client.get_all_vms().await?;
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&vms)? }] }))
            },
            "list_containers" => {
                let vms = self.client.get_all_vms().await?;
                let containers: Vec<_> = vms.into_iter()
                    .filter(|vm| vm.vm_type.as_deref() == Some("lxc"))
                    .collect();
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&containers)? }] }))
            },
            "start_vm" => self.handle_vm_action(args, "start", None).await,
            "start_container" => self.handle_vm_action(args, "start", Some("lxc")).await,
            "stop_vm" => self.handle_vm_action(args, "stop", None).await,
            "stop_container" => self.handle_vm_action(args, "stop", Some("lxc")).await,
            "shutdown_vm" => self.handle_vm_action(args, "shutdown", None).await,
            "shutdown_container" => self.handle_vm_action(args, "shutdown", Some("lxc")).await,
            "reboot_vm" => self.handle_vm_action(args, "reboot", None).await,
            "create_vm" => self.handle_create(args, "qemu").await,
            "create_container" => self.handle_create(args, "lxc").await,
            "delete_vm" => self.handle_delete(args, "qemu").await,
            "delete_container" => self.handle_delete(args, "lxc").await,
            "reset_vm" => self.handle_reset(args, "qemu").await,
            "reset_container" => self.handle_reset(args, "lxc").await,
            "list_templates" => {
                let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
                let storage = args.get("storage").and_then(|v| v.as_str()).unwrap_or("local");
                let content = args.get("content").and_then(|v| v.as_str()).or(Some("vztmpl"));
                
                let templates = self.client.get_storage_content(node, storage, content).await?;
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&templates)? }] }))
            },
            "update_container_resources" => self.handle_update_resources(args, "lxc").await,
            "list_snapshots" => self.handle_snapshot_list(args).await,
            "snapshot_vm" => self.handle_snapshot_create(args).await,
            "rollback_vm" => self.handle_snapshot_rollback(args).await,
            "delete_snapshot" => self.handle_snapshot_delete(args).await,
            "clone_vm" => self.handle_clone(args).await,
            "migrate_vm" => self.handle_migrate(args).await,
            _ => anyhow::bail!("Unknown tool: {}", name),
        }
    }

    async fn handle_clone(&self, args: &Value) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args.get("vmid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing vmid"))?;
        let newid = args.get("newid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing newid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        
        let name = args.get("name").and_then(|v| v.as_str());
        let target = args.get("target").and_then(|v| v.as_str());
        let full = args.get("full").and_then(|v| v.as_bool());

        let res = self.client.clone_resource(node, vmid, vm_type, newid, name, target, full).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Clone initiated. UPID: {}", res) }] }))
    }

    async fn handle_migrate(&self, args: &Value) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args.get("vmid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing vmid"))?;
        let target_node = args.get("target_node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing target_node"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let online = args.get("online").and_then(|v| v.as_bool()).unwrap_or(false);

        let res = self.client.migrate_resource(node, vmid, vm_type, target_node, online).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Migration initiated. UPID: {}", res) }] }))
    }

    async fn handle_snapshot_list(&self, args: &Value) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args.get("vmid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");

        let snapshots = self.client.get_snapshots(node, vmid, vm_type).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&snapshots)? }] }))
    }

    async fn handle_snapshot_create(&self, args: &Value) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args.get("vmid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let snapname = args.get("snapname").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing snapname"))?;
        let desc = args.get("description").and_then(|v| v.as_str());
        let vmstate = args.get("vmstate").and_then(|v| v.as_bool()).unwrap_or(false);

        let res = self.client.create_snapshot(node, vmid, vm_type, snapname, desc, vmstate).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Snapshot '{}' created. UPID: {}", snapname, res) }] }))
    }

    async fn handle_snapshot_rollback(&self, args: &Value) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args.get("vmid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let snapname = args.get("snapname").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing snapname"))?;

        let res = self.client.rollback_snapshot(node, vmid, vm_type, snapname).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Rollback to '{}' initiated. UPID: {}", snapname, res) }] }))
    }

    async fn handle_snapshot_delete(&self, args: &Value) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args.get("vmid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing vmid"))?;
        let vm_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu");
        let snapname = args.get("snapname").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing snapname"))?;

        let res = self.client.delete_snapshot(node, vmid, vm_type, snapname).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Delete snapshot '{}' initiated. UPID: {}", snapname, res) }] }))
    }

    async fn handle_update_resources(&self, args: &Value, resource_type: &str) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args.get("vmid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing vmid"))?;

        let mut output = Vec::new();

        // Handle Disk Resize
        if let Some(gb) = args.get("disk_gb").and_then(|v| v.as_i64()) {
            let disk = args.get("disk").and_then(|v| v.as_str()).unwrap_or("rootfs");
            let size = format!("+{}G", gb);
            let upid = self.client.resize_disk(node, vmid, resource_type, disk, &size).await?;
            output.push(format!("Disk {} resize initiated (+{}GB). UPID: {}", disk, gb, upid));
        }

        // Handle Config Update
        let mut config_params = serde_json::Map::new();
        if let Some(c) = args.get("cores") { config_params.insert("cores".to_string(), c.clone()); }
        if let Some(m) = args.get("memory") { config_params.insert("memory".to_string(), m.clone()); }
        if let Some(s) = args.get("swap") { config_params.insert("swap".to_string(), s.clone()); }

        if !config_params.is_empty() {
             self.client.update_config(node, vmid, resource_type, &Value::Object(config_params)).await?;
             output.push("Resource config updated.".to_string());
        }

        if output.is_empty() {
            output.push("No changes requested.".to_string());
        }

        Ok(json!({ "content": [{ "type": "text", "text": output.join("\n") }] }))
    }

    async fn handle_reset(&self, args: &Value, expected_type: &str) -> Result<Value> {
        let id_key = if expected_type == "qemu" { "vm_id" } else { "container_id" };
        let id_str = args.get(id_key).and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing {}", id_key))?;
        let vmid: i64 = id_str.parse()?;
        
        info!("Resetting {} {}...", expected_type, vmid);

        let (node, vm_type) = self.client.find_vm_location(vmid).await?;
        
        if vm_type != expected_type {
            anyhow::bail!("ID {} is not a {}", vmid, expected_type);
        }

        let action = if expected_type == "qemu" { "reset" } else { "reboot" };
        
        let res = self.client.vm_action(&node, vmid, action, Some(expected_type)).await?;
        
        info!("Reset initiated for {} {}. UPID: {}", expected_type, vmid, res);
        Ok(json!({ "content": [{ "type": "text", "text": format!("Reset initiated. UPID: {}", res) }] }))
    }

    async fn handle_create(&self, args: &Value, resource_type: &str) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
        
        // Filter out "node" from args to send as params
        let mut params = args.as_object().ok_or(anyhow::anyhow!("Args must be object"))?.clone();
        params.remove("node");
        
        let res = self.client.create_resource(node, resource_type, &Value::Object(params)).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Create {} initiated. UPID: {}", resource_type, res) }] }))
    }

    async fn handle_delete(&self, args: &Value, resource_type: &str) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args.get("vmid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing vmid"))?;

        let res = self.client.delete_resource(node, vmid, resource_type).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Delete {} initiated. UPID: {}", resource_type, res) }] }))
    }

    async fn handle_vm_action(&self, args: &Value, action: &str, forced_type: Option<&str>) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
        let vmid = args.get("vmid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing vmid"))?;
        
        let vm_type = if let Some(t) = forced_type {
            Some(t)
        } else {
            args.get("type").and_then(|v| v.as_str())
        };

        let res = self.client.vm_action(node, vmid, action, vm_type).await?;
        Ok(json!({ "content": [{ "type": "text", "text": format!("Action '{}' initiated. UPID: {}", action, res) }] }))
    }
}
