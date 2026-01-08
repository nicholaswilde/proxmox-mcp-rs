mod proxmox;
mod mcp;
mod settings;

use clap::Parser;
use log::{info, error};
use proxmox::ProxmoxClient;
use mcp::McpServer;
use std::process;
use settings::Settings;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Config file path
    #[arg(short, long)]
    config: Option<String>,

    /// Proxmox Host (e.g., 192.168.1.10:8006)
    #[arg(long, env = "PROXMOX_HOST")]
    host: Option<String>,

    /// Proxmox User (e.g., root@pam)
    #[arg(long, env = "PROXMOX_USER")]
    user: Option<String>,

    /// Proxmox Password
    #[arg(long, env = "PROXMOX_PASSWORD")]
    password: Option<String>,

    /// Disable SSL verification (for self-signed certs)
    #[arg(long, default_value_t = false)]
    no_verify_ssl: bool,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    let mut settings = match Settings::new(args.config.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    // Override settings with CLI arguments if provided
    if let Some(host) = args.host {
        settings.host = Some(host);
    }
    if let Some(user) = args.user {
        settings.user = Some(user);
    }
    if let Some(password) = args.password {
        settings.password = Some(password);
    }
    // For boolean flag, if it's true in CLI, it overrides settings to true.
    // If CLI is false (default), we keep settings value (which might be true or false).
    if args.no_verify_ssl {
        settings.no_verify_ssl = Some(true);
    }

    if let Err(e) = settings.validate() {
        error!("Configuration error: {}", e);
        process::exit(1);
    }

    // Safe to unwrap because validate() checks these
    let host = settings.host.unwrap();
    let user = settings.user.unwrap();
    let password = settings.password.unwrap();
    let no_verify_ssl = settings.no_verify_ssl.unwrap_or(false);

    info!("Connecting to Proxmox at {}", host);

    let mut client = match ProxmoxClient::new(&host, !no_verify_ssl) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create client: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = client.login(&user, &password).await {
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