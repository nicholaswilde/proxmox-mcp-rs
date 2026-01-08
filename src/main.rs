mod proxmox;
mod mcp;
mod settings;
mod http_server;
mod tests;

use clap::Parser;
use log::{info, error};
use proxmox::ProxmoxClient;
use mcp::McpServer;
use std::process;
use settings::Settings;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

#[derive(Parser, Debug)]
#[command(author, version = env!("PROJECT_VERSION"), about, long_about = None)]
struct Args {
    /// Config file path
    #[arg(short, long, env = "PROXMOX_CONFIG")]
    config: Option<String>,

    /// Proxmox Host (e.g., 192.168.1.10)
    #[arg(short = 'H', long, env = "PROXMOX_HOST")]
    host: Option<String>,

    /// Proxmox Port (default: 8006)
    #[arg(short = 'p', long, env = "PROXMOX_PORT")]
    port: Option<u16>,

    /// Proxmox User (e.g., root@pam)
    #[arg(short = 'u', long, env = "PROXMOX_USER")]
    user: Option<String>,

    /// Proxmox Password
    #[arg(short = 'P', long, env = "PROXMOX_PASSWORD", conflicts_with_all = ["token_name", "token_value"])]
    password: Option<String>,

    /// API Token Name (e.g., mytoken)
    #[arg(short = 'n', long, env = "PROXMOX_TOKEN_NAME", requires = "token_value")]
    token_name: Option<String>,

    /// API Token Value (UUID)
    #[arg(short = 'v', long, env = "PROXMOX_TOKEN_VALUE", requires = "token_name")]
    token_value: Option<String>,

    /// Disable SSL verification (for self-signed certs)
    #[arg(short = 'k', long, env = "PROXMOX_NO_VERIFY_SSL", default_value_t = false)]
    no_verify_ssl: bool,

    /// Log level (error, warn, info, debug, trace)
    #[arg(short = 'L', long, env = "PROXMOX_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Enable logging to a file
    #[arg(long, env = "PROXMOX_LOG_FILE_ENABLE", default_value_t = false)]
    log_file_enable: bool,

    /// Log file directory
    #[arg(long, env = "PROXMOX_LOG_DIR", default_value = ".")]
    log_dir: String,

    /// Log filename prefix
    #[arg(long, env = "PROXMOX_LOG_FILENAME", default_value = "proxmox-mcp-rs.log")]
    log_filename: String,

    /// Log rotation strategy (daily, hourly, never)
    #[arg(long, env = "PROXMOX_LOG_ROTATE", default_value = "daily")]
    log_rotate: String,

    /// Server type (stdio or http)
    #[arg(short = 't', long, env = "PROXMOX_SERVER_TYPE")]
    server_type: Option<String>,

    /// HTTP Port (only for http type)
    #[arg(short = 'l', long, env = "PROXMOX_HTTP_PORT")]
    http_port: Option<u16>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize Logging
    let _guard = {
        let filter_layer = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&args.log_level));

        let stdout_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_filter(filter_layer.clone());

        let file_layer = if args.log_file_enable {
            let rotation = match args.log_rotate.to_lowercase().as_str() {
                "hourly" => Rotation::HOURLY,
                "never" => Rotation::NEVER,
                _ => Rotation::DAILY,
            };

            let file_appender = RollingFileAppender::builder()
                .rotation(rotation)
                .filename_prefix(&args.log_filename)
                .build(&args.log_dir)
                .expect("Failed to create log file appender");

            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            
            Some((tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_filter(filter_layer), guard))
        } else {
            None
        };

        // Initialize LogTracer to capture log::info! calls
        tracing_log::LogTracer::init().expect("Failed to init LogTracer");

        let registry = tracing_subscriber::registry().with(stdout_layer);

        if let Some((layer, guard)) = file_layer {
            registry.with(layer).init();
            Some(guard)
        } else {
            registry.init();
            None
        }
    };

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
    if let Some(port) = args.port {
        settings.port = Some(port);
    }
    if let Some(user) = args.user {
        settings.user = Some(user);
    }
    if let Some(password) = args.password {
        settings.password = Some(password);
    }
    if let Some(token_name) = args.token_name {
        settings.token_name = Some(token_name);
    }
    if let Some(token_value) = args.token_value {
        settings.token_value = Some(token_value);
    }
    if args.no_verify_ssl {
        settings.no_verify_ssl = Some(true);
    }
    if let Some(st) = args.server_type {
        settings.server_type = Some(st);
    }
    if let Some(hp) = args.http_port {
        settings.http_port = Some(hp);
    }
    
    // We don't override log settings in `settings` struct because we used them directly from CLI args
    // to initialize logging BEFORE loading other settings (so we can log config errors).
    
    if let Err(e) = settings.validate() {
        error!("Configuration error: {}", e);
        process::exit(1);
    }

    // Safe to unwrap because validate() checks these
    let host = settings.host.unwrap();
    let port = settings.port.unwrap_or(8006);
    let user = settings.user.unwrap();
    let password = settings.password;
    let token_name = settings.token_name;
    let token_value = settings.token_value;
    let no_verify_ssl = settings.no_verify_ssl.unwrap_or(false);
    let server_type = settings.server_type.unwrap_or_else(|| "stdio".to_string());
    let http_port = settings.http_port.unwrap_or(3000);

    info!("Connecting to Proxmox at {}:{}", host, port);

    let mut client = match ProxmoxClient::new(&host, port, !no_verify_ssl) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create client: {}", e);
            process::exit(1);
        }
    };

    if let (Some(t_name), Some(t_value)) = (token_name, token_value) {
        info!("Using API Token authentication");
        client.set_api_token(&user, &t_name, &t_value);
    } else if let Some(pass) = password {
        if let Err(e) = client.login(&user, &pass).await {
            error!("Authentication failed: {}", e);
            process::exit(1);
        }
    } else {
         error!("No authentication method provided");
         process::exit(1);
    }

    let mut server = McpServer::new(client);
    
    match server_type.as_str() {
        "http" => {
            info!("Starting MCP Server (HTTP transport) on port {}...", http_port);
            if let Err(e) = http_server::run_http_server(server, http_port).await {
                error!("HTTP Server error: {}", e);
                process::exit(1);
            }
        },
        "stdio" | _ => {
            info!("Starting MCP Server (stdio transport)...");
            if let Err(e) = server.run_stdio().await {
                error!("Server error: {}", e);
                process::exit(1);
            }
        }
    }
}