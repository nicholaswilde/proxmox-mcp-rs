mod proxmox;
mod mcp;

use clap::Parser;
use log::{info, error};
use proxmox::ProxmoxClient;
use mcp::McpServer;
use std::process;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Proxmox Host (e.g., 192.168.1.10:8006)
    #[arg(long, env = "PROXMOX_HOST")]
    host: String,

    /// Proxmox User (e.g., root@pam)
    #[arg(long, env = "PROXMOX_USER")]
    user: String,

    /// Proxmox Password
    #[arg(long, env = "PROXMOX_PASSWORD")]
    password: String,

    /// Disable SSL verification (for self-signed certs)
    #[arg(long, default_value_t = false)]
    no_verify_ssl: bool,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    info!("Connecting to Proxmox at {}", args.host);

    let mut client = match ProxmoxClient::new(&args.host, !args.no_verify_ssl) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create client: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = client.login(&args.user, &args.password).await {
        error!("Authentication failed: {}", e);
        process::exit(1);
    }

    let mut server = McpServer::new(client);
    
    info!("Starting MCP Server (stdio transport)...");
    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        process::exit(1);
    }
}