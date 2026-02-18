mod cli;
mod config;
mod runner;
mod handlers;
mod utils;
mod logger;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use handlers::{task, env, list, info};

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    if cli.list {
        list::handle_list()
    } else if cli.info {
        info::handle_info()
    } else if cli.env {
        env::handle_env(&cli)
    } else {
        let task_name = cli.task.unwrap_or_else(|| "default".to_string());
        task::handle_runner_entry(task_name, cli.args, cli.dry_run)
    }
}
