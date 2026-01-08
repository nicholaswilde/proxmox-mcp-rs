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

pub struct McpServer {
    client: ProxmoxClient,
}

impl McpServer {
    pub fn new(client: ProxmoxClient) -> Self {
        Self { client }
    }

    pub async fn run(&mut self) -> Result<()> {
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

    async fn handle_request(&self, req: JsonRpcRequest) -> Result<Value> {
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
            })
        ]
    }

    async fn call_tool(&self, name: &str, args: &Value) -> Result<Value> {
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
            _ => anyhow::bail!("Unknown tool: {}", name),
        }
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
