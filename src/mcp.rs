use crate::proxmox::ProxmoxClient;
use anyhow::Result;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

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
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "proxmox-mcp-rs",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "tools": {},
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
            }),
            json!({
                "name": "list_backups",
                "description": "List backups on a specific storage",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "storage": { "type": "string" },
                        "vmid": { "type": "integer", "description": "Filter by VM ID (optional)" }
                    },
                    "required": ["node", "storage"]
                }
            }),
            json!({
                "name": "create_backup",
                "description": "Create a backup (vzdump) of a VM or Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "storage": { "type": "string", "description": "Target storage" },
                        "mode": { "type": "string", "enum": ["snapshot", "suspend", "stop"], "description": "Backup mode" },
                        "compress": { "type": "string", "enum": ["zstd", "gzip", "lzo"], "description": "Compression" },
                        "remove": { "type": "boolean", "description": "Remove old backups (prune)?" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "restore_backup",
                "description": "Restore a VM or Container from a backup",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer", "description": "ID to restore to" },
                        "archive": { "type": "string", "description": "Backup volume ID (volid)" },
                        "storage": { "type": "string", "description": "Target storage" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "force": { "type": "boolean", "description": "Overwrite existing?" }
                    },
                    "required": ["node", "vmid", "archive", "type"]
                }
            }),
            json!({
                "name": "get_task_status",
                "description": "Get the status of a specific task (UPID)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "upid": { "type": "string", "description": "Unique Process ID" }
                    },
                    "required": ["node", "upid"]
                }
            }),
            json!({
                "name": "list_tasks",
                "description": "List recent tasks on a node",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "limit": { "type": "integer", "description": "Max tasks to list (default: 50)" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "wait_for_task",
                "description": "Wait for a task to finish (with timeout)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "upid": { "type": "string", "description": "Unique Process ID" },
                        "timeout": { "type": "integer", "description": "Timeout in seconds (default: 60)" }
                    },
                    "required": ["node", "upid"]
                }
            }),
            json!({
                "name": "list_networks",
                "description": "List network interfaces and bridges on a node",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "The node name" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "list_storage",
                "description": "List all storage on a node",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "The node name" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "list_isos",
                "description": "List ISO images on a specific storage",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "The node name" },
                        "storage": { "type": "string", "description": "Storage name" }
                    },
                    "required": ["node", "storage"]
                }
            }),
            json!({
                "name": "get_cluster_status",
                "description": "Get cluster status information",
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
                        "limit": { "type": "integer", "description": "Max lines to read" }
                    },
                    "required": []
                }
            }),
            json!({
                "name": "list_firewall_rules",
                "description": "List firewall rules",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string", "description": "Node name (optional)" },
                        "vmid": { "type": "integer", "description": "VM ID (optional)" }
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
                "description": "Delete a firewall rule",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "pos": { "type": "integer", "description": "Rule position/index (optional if digest provided, but usually required)" }
                    },
                    "required": ["pos"]
                }
            }),
            json!({
                "name": "add_disk",
                "description": "Add a virtual disk to a VM or Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "device": { "type": "string", "description": "Device name (e.g. 'scsi1', 'virtio0')" },
                        "storage": { "type": "string", "description": "Storage ID (e.g. 'local-lvm')" },
                        "size_gb": { "type": "integer", "description": "Size in GB" },
                        "format": { "type": "string", "enum": ["raw", "qcow2", "vmdk"], "description": "Disk format (optional)" },
                        "extra_options": { "type": "string", "description": "Extra options string (e.g. 'discard=on,ssd=1')" }
                    },
                    "required": ["node", "vmid", "device", "storage", "size_gb"]
                }
            }),
            json!({
                "name": "remove_disk",
                "description": "Remove (detach/delete) a virtual disk",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "device": { "type": "string", "description": "Device name to remove (e.g. 'scsi1')" }
                    },
                    "required": ["node", "vmid", "device"]
                }
            }),
            json!({
                "name": "add_network",
                "description": "Add a network interface",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "device": { "type": "string", "description": "Interface ID (e.g. 'net1')" },
                        "bridge": { "type": "string", "description": "Bridge to attach to (e.g. 'vmbr0')" },
                        "model": { "type": "string", "description": "Model (e.g. 'virtio', 'e1000')" },
                        "mac": { "type": "string", "description": "MAC address (optional)" },
                        "extra_options": { "type": "string", "description": "Extra options string" }
                    },
                    "required": ["node", "vmid", "device", "bridge"]
                }
            }),
            json!({
                "name": "remove_network",
                "description": "Remove a network interface",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "device": { "type": "string", "description": "Interface ID to remove (e.g. 'net1')" }
                    },
                    "required": ["node", "vmid", "device"]
                }
            }),
            json!({
                "name": "get_node_stats",
                "description": "Get RRD statistics for a node",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "timeframe": { "type": "string", "enum": ["hour", "day", "week", "month", "year"], "description": "Timeframe (default: hour)" },
                        "cf": { "type": "string", "enum": ["AVERAGE", "MAX"], "description": "Consolidation function (default: AVERAGE)" }
                    },
                    "required": ["node"]
                }
            }),
            json!({
                "name": "get_vm_stats",
                "description": "Get RRD statistics for a VM or Container",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "type": { "type": "string", "enum": ["qemu", "lxc"] },
                        "timeframe": { "type": "string", "enum": ["hour", "day", "week", "month", "year"], "description": "Timeframe (default: hour)" },
                        "cf": { "type": "string", "enum": ["AVERAGE", "MAX"], "description": "Consolidation function (default: AVERAGE)" }
                    },
                    "required": ["node", "vmid"]
                }
            }),
            json!({
                "name": "read_task_log",
                "description": "Read the log of a specific task (UPID)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "upid": { "type": "string", "description": "Unique Process ID" }
                    },
                    "required": ["node", "upid"]
                }
            }),
            json!({
                "name": "get_vm_config",
                "description": "Get the configuration of a VM or Container",
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
                "name": "download_url",
                "description": "Download an ISO or Container template from a URL to storage",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "storage": { "type": "string" },
                        "url": { "type": "string", "description": "The URL to download from" },
                        "filename": { "type": "string", "description": "Target filename" },
                        "content": { "type": "string", "enum": ["iso", "vztmpl"], "description": "Content type" },
                        "checksum": { "type": "string", "description": "Optional checksum" },
                        "checksum_algorithm": { "type": "string", "enum": ["md5", "sha1", "sha224", "sha256", "sha384", "sha512"], "description": "Optional checksum algorithm" }
                    },
                    "required": ["node", "storage", "url", "filename", "content"]
                }
            }),
            json!({
                "name": "list_users",
                "description": "List all users in the cluster",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "create_user",
                "description": "Create a new user",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "userid": { "type": "string", "description": "User ID (e.g. user@pve)" },
                        "password": { "type": "string", "description": "Initial password" },
                        "email": { "type": "string", "description": "E-mail address" },
                        "firstname": { "type": "string", "description": "First name" },
                        "lastname": { "type": "string", "description": "Last name" },
                        "expire": { "type": "integer", "description": "Account expiration date (seconds since epoch)" },
                        "enable": { "type": "boolean", "description": "Enable the account (default: true)" },
                        "comment": { "type": "string", "description": "Comment/Note" },
                        "groups": { "type": "array", "items": { "type": "string" }, "description": "List of groups" }
                    },
                    "required": ["userid", "password"]
                }
            }),
            json!({
                "name": "delete_user",
                "description": "Delete a user",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "userid": { "type": "string", "description": "User ID to delete" }
                    },
                    "required": ["userid"]
                }
            }),
            json!({
                "name": "list_cluster_storage",
                "description": "List all storage definitions in the cluster configuration",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "add_storage",
                "description": "Add a new storage definition",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "storage": { "type": "string", "description": "Storage ID" },
                        "type": { "type": "string", "enum": ["dir", "nfs", "cifs", "lvm", "lvmthin", "zfs", "iscsi", "rbd", "cephfs"], "description": "Storage type" },
                        "content": { "type": "string", "description": "Allowed content types (comma separated, e.g. 'iso,backup')" },
                        "nodes": { "type": "array", "items": { "type": "string" }, "description": "Restrict to specific nodes" },
                        "enable": { "type": "boolean", "description": "Enable storage (default: true)" },
                        "path": { "type": "string", "description": "File system path (for dir, nfs, etc.)" },
                        "server": { "type": "string", "description": "Server IP/Hostname (for nfs, cifs, iscsi, etc.)" },
                        "share": { "type": "string", "description": "Share name (for cifs)" },
                        "export": { "type": "string", "description": "Export path (for nfs)" },
                        "username": { "type": "string", "description": "Username (for cifs)" },
                        "password": { "type": "string", "description": "Password (for cifs)" },
                        "pool": { "type": "string", "description": "Pool name (for zfs, rbd)" },
                        "vgname": { "type": "string", "description": "Volume Group name (for lvm)" }
                    },
                    "required": ["storage", "type"]
                }
            }),
            json!({
                "name": "delete_storage",
                "description": "Delete a storage definition",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "storage": { "type": "string", "description": "Storage ID" }
                    },
                    "required": ["storage"]
                }
            }),
            json!({
                "name": "update_storage",
                "description": "Update a storage definition",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "storage": { "type": "string", "description": "Storage ID" },
                        "content": { "type": "string", "description": "Allowed content types" },
                        "nodes": { "type": "string", "description": "Comma separated list of nodes" },
                        "enable": { "type": "boolean", "description": "Enable/Disable" }
                    },
                    "required": ["storage"]
                }
            }),
            json!({
                "name": "vm_agent_ping",
                "description": "Ping the QEMU Guest Agent inside a VM",
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
                "description": "Execute a command inside a VM via QEMU Agent (Async, returns PID)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "command": { "type": "string", "description": "Command to run (e.g. 'ls -l /')" },
                        "input_data": { "type": "string", "description": "Input data to pass to stdin" }
                    },
                    "required": ["node", "vmid", "command"]
                }
            }),
            json!({
                "name": "vm_exec_status",
                "description": "Get status/output of a command executed via QEMU Agent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "pid": { "type": "integer", "description": "PID from vm_exec" }
                    },
                    "required": ["node", "vmid", "pid"]
                }
            }),
            json!({
                "name": "vm_read_file",
                "description": "Read a file from inside a VM via QEMU Agent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "file": { "type": "string", "description": "Path to file" }
                    },
                    "required": ["node", "vmid", "file"]
                }
            }),
            json!({
                "name": "vm_write_file",
                "description": "Write to a file inside a VM via QEMU Agent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node": { "type": "string" },
                        "vmid": { "type": "integer" },
                        "file": { "type": "string", "description": "Path to file" },
                        "content": { "type": "string", "description": "Content to write" },
                        "encode": { "type": "boolean", "description": "Base64 encode content? (default: false)" }
                    },
                    "required": ["node", "vmid", "file", "content"]
                }
            }),
        ]
    }

    async fn handle_resource_read(&self, uri: &str) -> Result<Value> {
        match uri {
            "proxmox://vms" => {
                let vms = self.client.get_all_vms().await?;
                let content = serde_json::to_string_pretty(&vms)?;
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
            "list_nodes" => {
                let nodes = self.client.get_nodes().await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&nodes)? }] }),
                )
            }
            "list_vms" => {
                let vms = self.client.get_all_vms().await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&vms)? }] }),
                )
            }
            "list_containers" => {
                let vms = self.client.get_all_vms().await?;
                let containers: Vec<_> = vms
                    .into_iter()
                    .filter(|vm| vm.vm_type.as_deref() == Some("lxc"))
                    .collect();
                Ok(
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&containers)? }] }),
                )
            }
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

                let templates = self
                    .client
                    .get_storage_content(node, storage, content)
                    .await?;
                Ok(
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&templates)? }] }),
                )
            }
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
            "list_storage" => self.handle_list_storage(args).await,
            "list_isos" => self.handle_list_isos(args).await,
            "get_cluster_status" => self.handle_get_cluster_status(args).await,
            "get_cluster_log" => self.handle_get_cluster_log(args).await,
            "list_firewall_rules" => self.handle_list_firewall_rules(args).await,
            "add_firewall_rule" => self.handle_add_firewall_rule(args).await,
            "delete_firewall_rule" => self.handle_delete_firewall_rule(args).await,
            "add_disk" => self.handle_add_disk(args).await,
            "remove_disk" => self.handle_remove_disk(args).await,
            "add_network" => self.handle_add_network(args).await,
            "remove_network" => self.handle_remove_network(args).await,
            "get_node_stats" => self.handle_get_node_stats(args).await,
            "get_vm_stats" => self.handle_get_vm_stats(args).await,
            "read_task_log" => self.handle_read_task_log(args).await,
            "get_vm_config" => self.handle_get_vm_config(args).await,
            "download_url" => self.handle_download_url(args).await,
            "list_users" => self.handle_list_users().await,
            "create_user" => self.handle_create_user(args).await,
            "delete_user" => self.handle_delete_user(args).await,
            "list_cluster_storage" => self.handle_list_cluster_storage().await,
            "add_storage" => self.handle_add_storage(args).await,
            "delete_storage" => self.handle_delete_storage(args).await,
            "update_storage" => self.handle_update_storage(args).await,
            "vm_agent_ping" => self.handle_vm_agent_ping(args).await,
            "vm_exec" => self.handle_vm_exec(args).await,
            "vm_exec_status" => self.handle_vm_exec_status(args).await,
            "vm_read_file" => self.handle_vm_read_file(args).await,
            "vm_write_file" => self.handle_vm_write_file(args).await,
            _ => anyhow::bail!("Unknown tool: {}", name),
        }
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

        self.client.agent_ping(node, vmid).await?;
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
            .client
            .agent_exec(node, vmid, &command, input_data)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&res)? }] }))
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

        let res = self.client.agent_exec_status(node, vmid, pid).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&res)? }] }))
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

        let res = self.client.agent_file_read(node, vmid, file).await?;
        // Result usually has "content" (read bytes) or "bytes" (count).
        // content is text if possible?
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&res)? }] }))
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

        self.client
            .agent_file_write(node, vmid, file, content, encode)
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": "File written" }] }))
    }

    async fn handle_list_cluster_storage(&self) -> Result<Value> {
        let storage = self.client.get_cluster_storage().await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&storage)? }] }),
        )
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

        self.client
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

        self.client.delete_storage(storage).await?;
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

        self.client.update_storage(storage, &params).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Storage {} updated", storage) }] }),
        )
    }

    async fn handle_list_users(&self) -> Result<Value> {
        let users = self.client.get_users().await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&users)? }] }),
        )
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

        self.client
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

        self.client.delete_user(userid).await?;
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
            .client
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

    async fn handle_get_node_stats(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let timeframe = args.get("timeframe").and_then(|v| v.as_str());
        let cf = args.get("cf").and_then(|v| v.as_str());

        let stats = self.client.get_node_stats(node, timeframe, cf).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&stats)? }] }),
        )
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
            .client
            .get_resource_stats(node, vmid, vm_type, timeframe, cf)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&stats)? }] }),
        )
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

        self.client
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

        self.client
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

        self.client
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

        self.client
            .remove_network_interface(node, vmid, vm_type, device)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Network interface {} removed from {} {}", device, vm_type, vmid) }] }),
        )
    }

    async fn handle_list_firewall_rules(&self, args: &Value) -> Result<Value> {
        let node = args.get("node").and_then(|v| v.as_str());
        let vmid = args.get("vmid").and_then(|v| v.as_i64());

        let rules = self.client.get_firewall_rules(node, vmid).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&rules)? }] }),
        )
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

        self.client
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

        self.client.delete_firewall_rule(node, vmid, pos).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Firewall rule {} deleted", pos) }] }),
        )
    }

    async fn handle_get_cluster_status(&self, _args: &Value) -> Result<Value> {
        let status = self.client.get_cluster_status().await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&status)? }] }),
        )
    }

    async fn handle_get_cluster_log(&self, args: &Value) -> Result<Value> {
        let limit = args.get("limit").and_then(|v| v.as_u64());
        let log = self.client.get_cluster_log(limit).await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&log)? }] }))
    }

    async fn handle_list_storage(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;

        let storage = self.client.get_storage_list(node).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&storage)? }] }),
        )
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
            .client
            .get_storage_content(node, storage, Some("iso"))
            .await?;
        Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&isos)? }] }))
    }

    async fn handle_list_networks(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;

        let networks = self.client.get_network_interfaces(node).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&networks)? }] }),
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

        let status = self.client.get_task_status(node, upid).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&status)? }] }),
        )
    }

    async fn handle_list_tasks(&self, args: &Value) -> Result<Value> {
        let node = args
            .get("node")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing node"))?;
        let limit = args.get("limit").and_then(|v| v.as_u64());

        let tasks = self.client.list_tasks(node, limit).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&tasks)? }] }),
        )
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

        let start_time = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_secs(timeout);

        loop {
            if start_time.elapsed() > timeout_duration {
                anyhow::bail!("Timeout waiting for task {}", upid);
            }

            let status = self.client.get_task_status(node, upid).await?;
            // Status object has "status": "stopped" when done.
            // Also check "exitstatus": "OK" or "ERROR..."

            if let Some(s) = status.get("status").and_then(|v| v.as_str()) {
                if s == "stopped" {
                    let exit_status = status
                        .get("exitstatus")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    return Ok(
                        json!({ "content": [{ "type": "text", "text": format!("Task finished with status: {}\nFull details:\n{}", exit_status, serde_json::to_string_pretty(&status)?) }] }),
                    );
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
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

        let backups = self.client.get_backups(node, storage, vmid).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&backups)? }] }),
        )
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
            .client
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
            .client
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
            .client
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
            .client
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

        let snapshots = self.client.get_snapshots(node, vmid, vm_type).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&snapshots)? }] }),
        )
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
            .client
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
            .client
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
            .client
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
                .client
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
        if let Some(s) = args.get("swap") {
            config_params.insert("swap".to_string(), s.clone());
        }

        if !config_params.is_empty() {
            self.client
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

        let (node, vm_type) = self.client.find_vm_location(vmid).await?;

        if vm_type != expected_type {
            anyhow::bail!("ID {} is not a {}", vmid, expected_type);
        }

        let action = if expected_type == "qemu" {
            "reset"
        } else {
            "reboot"
        };

        let res = self
            .client
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
            .client
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
            .client
            .delete_resource(node, vmid, resource_type)
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Delete {} initiated. UPID: {}", resource_type, res) }] }),
        )
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

        let res = self.client.vm_action(node, vmid, action, vm_type).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Action '{}' initiated. UPID: {}", action, res) }] }),
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

        let log_entries = self.client.get_task_log(node, upid).await?;
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

        let config = self.client.get_vm_config(node, vmid, vm_type).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&config)? }] }),
        )
    }
}
