mod cli;
mod compress;
mod config;
mod runner;
mod tools;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use tools::Tool;

fn main() {
    let cli = Cli::parse();
    let cfg = Config::load();

    match cli.command {
        Commands::Info => {
            print_info(&cfg);
        }
        Commands::Init { global } => {
            create_config(global);
        }
        command => {
            let tool: Box<dyn Tool> = match command {
                Commands::Git { args } => Box::new(tools::git::GitTool::new(args)),
                Commands::Cargo { args } => Box::new(tools::cargo::CargoTool::new(args)),
                Commands::Ls { path } => Box::new(tools::fs::FsTool::new(
                    path.unwrap_or_else(|| ".".into()),
                    &cfg,
                )),
                Commands::Grep { pattern, path, rg } => Box::new(tools::grep::GrepTool::new(
                    pattern,
                    path.unwrap_or_else(|| ".".into()),
                    rg,
                )),
                Commands::Python { args } => Box::new(tools::python::PythonTool::new(args)),
                Commands::Docker { args } => Box::new(tools::docker::DockerTool::new(args)),
                Commands::Run { args } => Box::new(tools::generic::GenericTool::new(args)),
                Commands::Info | Commands::Init { .. } => unreachable!(),
            };

            let output = tool.run();
            println!("{output}");
        }
    }
}

fn print_info(cfg: &Config) {
    let types = config::detect_project();
    println!("[cx info]");
    println!("  version: {}", env!("CARGO_PKG_VERSION"));
    if types.is_empty() {
        println!("  project: (none detected)");
    } else {
        let names: Vec<String> = types.iter().map(|t| t.to_string()).collect();
        println!("  project: {}", names.join(", "));
    }
    println!("  max_lines: {}", cfg.max_lines);
    println!("  max_line_len: {}", cfg.max_line_len);
    println!("  show_footer: {}", cfg.show_footer);
    println!("  ls_max_depth: {}", cfg.ls_max_depth);
    println!("  ls_max_entries: {}", cfg.ls_max_entries);
    println!("  ls_skip: {:?}", cfg.ls_skip);
}

fn create_config(global: bool) {
    let path = if global {
        let dir = dirs::config_dir()
            .expect("could not determine config directory")
            .join("cx");
        std::fs::create_dir_all(&dir).expect("could not create config dir");
        dir.join("config.toml")
    } else {
        std::path::PathBuf::from(".cx.toml")
    };

    if path.exists() {
        println!("[cx] config already exists: {}", path.display());
        return;
    }

    std::fs::write(&path, Config::default_toml()).expect("could not write config");
    println!("[cx] created {}", path.display());
}
