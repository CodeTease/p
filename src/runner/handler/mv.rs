// Mv portable handler

use anyhow::{Result, Context};
use std::fs;
use std::path::Path;

pub fn handle_mv(args: &[String]) -> Result<()> {
    if args.len() != 2 {
        anyhow::bail!("mv command requires exactly 2 arguments: source and destination");
    }

    let src = Path::new(&args[0]);
    let dst = Path::new(&args[1]);

    fs::rename(src, dst).with_context(|| format!("Failed to move from {:?} to {:?}", src, dst))?;

    Ok(())
}