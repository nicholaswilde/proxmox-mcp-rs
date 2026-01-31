#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::CommandFactory;

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
}
