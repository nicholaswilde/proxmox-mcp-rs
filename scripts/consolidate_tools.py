import sys
import re

def consolidate():
    path = 'src/mcp.rs'
    with open(path, 'r') as f:
        content = f.read()

    # 1. Update tool_defs_vm_lifecycle
    content = re.sub(
        r'fn tool_defs_vm_lifecycle\(&self\) -> Vec<Value> \{.*?\}\n\s+fn',
        '''fn tool_defs_vm_lifecycle(&self) -> Vec<Value> {
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
        ]
    }

    fn''',
        content,
        flags=re.DOTALL
    )

    # 5. Fix call_tool match block
    match_block = '''    pub async fn call_tool(&self, name: &str, args: &Value) -> Result<Value> {
        match name {
            "load_all_tools" => {
                let mut state = self.state.lock().unwrap();
                state.tools_loaded = true;
                state.should_notify = true;
                Ok(json!({ "content": [{ "type": "text", "text": "All tools loaded." }] }))
            }
            "list_nodes" => {
                let nodes = self.get_client(args)?.get_nodes().await?;
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&nodes)? }] }))
            }
            "list_vms" => {
                let vms = self.get_client(args)?.get_all_vms().await?;
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&vms)? }] }))
            }
            "list_containers" => {
                let vms = self.get_client(args)?.get_all_vms().await?;
                let containers: Vec<_> = vms.into_iter().filter(|vm| vm.vm_type.as_deref() == Some("lxc")).collect();
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&containers)? }] }))
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
            "list_tasks" => self.handle_list_tasks(args).await,
            "list_backups" => self.handle_list_backups(args).await,
            "list_snapshots" => self.handle_snapshot_list(args).await,
            "list_templates" => {
                let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
                let storage = args.get("storage").and_then(|v| v.as_str()).unwrap_or("local");
                let templates = self.get_client(args)?.get_storage_content(node, storage, Some(args.get("content").and_then(|v| v.as_str()).unwrap_or("vztmpl"))).await?;
                Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&templates)? }] }))
            }
            "list_isos" => self.handle_list_isos(args).await,
            "bulk_vm_action" => self.handle_bulk_vm_action(args).await,
            "scan_storage_remote" => self.handle_scan_storage_remote(args).await,
            "get_console_url" => {
                let node = args.get("node").and_then(|v| v.as_str()).ok_or(anyhow::anyhow!("Missing node"))?;
                let vmid = args.get("vmid").and_then(|v| v.as_i64()).ok_or(anyhow::anyhow!("Missing vmid"))?;
                let url = self.get_client(args)?.get_console_url(node, vmid, args.get("type").and_then(|v| v.as_str()).unwrap_or("qemu"), args.get("console").and_then(|v| v.as_str()))?;
                Ok(json!({ "content": [{ "type": "text", "text": url }] }))
            }
            _ => anyhow::bail!("Unknown tool: {}", name),
        }
    }'''

    content = re.sub(
        r'pub async fn call_tool\(&self, name: &str, args: &Value\) -> Result<Value> \{.*?\}\n',
        match_block + '\n',
        content,
        flags=re.DOTALL
    )

    with open(path, 'w') as f:
        f.write(content)

if __name__ == "__main__":
    consolidate()
