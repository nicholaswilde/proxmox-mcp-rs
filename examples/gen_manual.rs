use clap::CommandFactory;
use clap_complete::{generate_to, Shell};
use clap_mangen::Man;
use proxmox_mcp_rs::cli::Args;
use std::fs;

fn main() -> std::io::Result<()> {
    let out_dir = "assets";
    let man_dir = format!("{}/man", out_dir);
    let comp_dir = format!("{}/completions", out_dir);

    fs::create_dir_all(&man_dir)?;
    fs::create_dir_all(&comp_dir)?;

    let mut cmd = Args::command();
    cmd.build();

    // Generate Man Page
    let man = Man::new(cmd.clone());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;
    fs::write(format!("{}/proxmox-mcp-rs.1", man_dir), buffer)?;
    println!("Man page generated in {}", man_dir);

    // Generate Completions
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
        generate_to(shell, &mut cmd, "proxmox-mcp-rs", &comp_dir)?;
    }
    println!("Completions generated in {}", comp_dir);

    Ok(())
}
