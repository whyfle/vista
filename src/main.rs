pub mod distro;
pub mod packages;
pub mod config;
pub mod cache;
pub mod database;
pub mod github;
pub mod flathub;
pub mod security;
pub mod downloader;
pub mod installers;
pub mod resolver;
pub mod cli;

use clap::Parser;
use colored::Colorize;

fn main() {
    let cli = cli::Cli::parse();
    if let Err(e) = cli::run(cli) {
        eprintln!("\n  {} {}", "✗".red().bold(), e.to_string().red());
        // Print cause chain if verbose?
        let mut source = e.source();
        while let Some(cause) = source {
            eprintln!("    Caused by: {}", cause);
            source = cause.source();
        }
        std::process::exit(1);
    }
}
