mod cli;
mod config;
mod runner;
mod handlers;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use handlers::{shell, task, clean, jump, init, env};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::D { path } => shell::handle_dir_jump(path),
        Commands::R { task, args } => task::handle_runner_entry(task, args),
        Commands::C => clean::handle_clean(),
        Commands::J { path } => jump::handle_jump(path),
        Commands::I { shell } => init::handle_init(&shell),
        Commands::E => env::handle_env(),
    }
}
