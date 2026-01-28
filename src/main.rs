mod cli;
mod config;
mod runner;
mod handlers;
mod utils;
pub mod pas;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use handlers::{shell, task, clean, jump, init, env, list, info};

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::D { path } => shell::handle_dir_jump(path),
        Commands::Shell => shell::handle_repl(),
        Commands::List => list::handle_list(),
        Commands::R { task, dry_run, args } => task::handle_runner_entry(task, args, dry_run),
        Commands::C => clean::handle_clean(),
        Commands::J { path } => jump::handle_jump(path),
        Commands::I { shell } => init::handle_init(&shell),
        Commands::E => env::handle_env(),
        Commands::Info => info::handle_info(),
    }
}
