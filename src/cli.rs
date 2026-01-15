use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(author, version = env!("PROJECT_VERSION"), about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Config file path
    #[arg(short, long, env = "PROXMOX_CONFIG")]
    pub config: Option<String>,

    /// Proxmox Host (e.g., 192.168.1.10)
    #[arg(short = 'H', long, env = "PROXMOX_HOST")]
    pub host: Option<String>,

    /// Proxmox Port (default: 8006)
    #[arg(short = 'p', long, env = "PROXMOX_PORT")]
    pub port: Option<u16>,

    /// Proxmox User (e.g., root@pam)
    #[arg(short = 'u', long, env = "PROXMOX_USER")]
    pub user: Option<String>,

    /// Proxmox Password
    #[arg(short = 'P', long, env = "PROXMOX_PASSWORD", conflicts_with_all = ["token_name", "token_value"])]
    pub password: Option<String>,

    /// API Token Name (e.g., mytoken)
    #[arg(
        short = 'n',
        long,
        env = "PROXMOX_TOKEN_NAME",
        requires = "token_value"
    )]
    pub token_name: Option<String>,

    /// API Token Value (UUID)
    #[arg(
        short = 'v',
        long,
        env = "PROXMOX_TOKEN_VALUE",
        requires = "token_name"
    )]
    pub token_value: Option<String>,

    /// Disable SSL verification (for self-signed certs)
    #[arg(
        short = 'k',
        long,
        env = "PROXMOX_NO_VERIFY_SSL",
        default_value_t = false
    )]
    pub no_verify_ssl: bool,

    /// Log level (error, warn, info, debug, trace)
    #[arg(short = 'L', long, env = "PROXMOX_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    /// Enable logging to a file
    #[arg(long, env = "PROXMOX_LOG_FILE_ENABLE", default_value_t = false)]
    pub log_file_enable: bool,

    /// Log file directory
    #[arg(long, env = "PROXMOX_LOG_DIR", default_value = ".")]
    pub log_dir: String,

    /// Log filename prefix
    #[arg(
        long,
        env = "PROXMOX_LOG_FILENAME",
        default_value = "proxmox-mcp-rs.log"
    )]
    pub log_filename: String,

    /// Log rotation strategy (daily, hourly, never)
    #[arg(long, env = "PROXMOX_LOG_ROTATE", default_value = "daily")]
    pub log_rotate: String,

    /// Server type (stdio or http)
    #[arg(short = 't', long, env = "PROXMOX_SERVER_TYPE")]
    pub server_type: Option<String>,

    /// HTTP Host (only for http type)
    #[arg(long, env = "PROXMOX_HTTP_HOST")]
    pub http_host: Option<String>,

    /// HTTP Port (only for http type)
    #[arg(short = 'l', long, env = "PROXMOX_HTTP_PORT")]
    pub http_port: Option<u16>,

    /// HTTP Auth Token (only for http type)
    #[arg(long, env = "PROXMOX_HTTP_AUTH_TOKEN")]
    pub http_auth_token: Option<String>,

    /// Enable Lazy Loading mode (starts with minimal tools)
    #[arg(long, env = "PROXMOX_LAZY_MODE", default_value_t = false)]
    pub lazy_mode: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate shell completion scripts
    Completions {
        /// The shell to generate the script for
        #[arg(value_enum)]
        shell: Shell,
    },
}
