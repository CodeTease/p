use anyhow::{Result, Context, bail};
use crate::runner::handler::cp::handle_cp;
use crate::runner::handler::mkdir::handle_mkdir;
use crate::runner::handler::rm::handle_rm;
use crate::runner::handler::ls::handle_ls;
use crate::runner::handler::mv::handle_mv;
use crate::runner::handler::cat::handle_cat;

pub fn run_portable_command(cmd_str: &str) -> Result<()> {
    let args = shell_words::split(cmd_str).context("Failed to parse portable command arguments")?;
    if args.is_empty() {
        return Ok(());
    }

    let command = &args[0];
    match command.as_str() {
        "p:rm" => handle_rm(&args[1..]),
        "p:mkdir" => handle_mkdir(&args[1..]),
        "p:cp" => handle_cp(&args[1..]),
        "p:ls" => handle_ls(&args[1..]),
        "p:mv" => handle_mv(&args[1..]),
        "p:cat" => handle_cat(&args[1..]),
        _ => bail!("Unknown portable command: {}", command),
    }
}
