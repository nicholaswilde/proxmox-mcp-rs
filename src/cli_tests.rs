#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::{CommandFactory, Parser};

    #[test]
    fn test_version_format() {
        let cmd = Args::command();
        let version = cmd.get_version().expect("Version should be set");

        // Check that version does not start with 'v'
        assert!(
            !version.starts_with('v'),
            "Version should not start with 'v': {}",
            version
        );

        // Check that version contains only numbers and dots (e.g. 0.3.29)
        // It might contain -dirty or other suffixes if built locally, but let's check basic format
        // Actually, env!("CARGO_PKG_VERSION") usually returns just the version number.
        // Let's verify it matches basic semver pattern start.
        let first_char = version.chars().next().unwrap();
        assert!(
            first_char.is_numeric(),
            "Version should start with a number: {}",
            version
        );
    }

    #[test]
    fn test_args_parsing() {
        let args = Args::try_parse_from(&[
            "proxmox-mcp-rs",
            "--host", "1.2.3.4",
            "--user", "root@pam",
            "--password", "secret",
            "--port", "8006",
            "--no-verify-ssl",
            "--log-level", "debug",
            "--log-file-enable",
            "--server-type", "http",
            "--lazy-mode"
        ]).unwrap();

        assert_eq!(args.host, Some("1.2.3.4".into()));
        assert_eq!(args.user, Some("root@pam".into()));
        assert_eq!(args.password, Some("secret".into()));
        assert_eq!(args.port, Some(8006));
        assert!(args.no_verify_ssl);
        assert_eq!(args.log_level, "debug");
        assert!(args.log_file_enable);
        assert_eq!(args.server_type, Some("http".into()));
        assert!(args.lazy_mode);
    }

    #[test]
    fn test_args_token_requirements() {
        // Missing token_value
        let res = Args::try_parse_from(&[
            "proxmox-mcp-rs",
            "--token-name", "mytoken"
        ]);
        assert!(res.is_err());

        // Missing token_name
        let res = Args::try_parse_from(&[
            "proxmox-mcp-rs",
            "--token-value", "myvalue"
        ]);
        assert!(res.is_err());

        // Both provided
        let args = Args::try_parse_from(&[
            "proxmox-mcp-rs",
            "--token-name", "mytoken",
            "--token-value", "myvalue",
            "--host", "h",
            "--user", "u"
        ]).unwrap();
        assert_eq!(args.token_name, Some("mytoken".into()));
        assert_eq!(args.token_value, Some("myvalue".into()));
    }
}
