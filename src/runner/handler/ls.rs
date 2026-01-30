// Ls portable handler

use anyhow::{Result, Context};
use std::fs;
use std::path::Path;

pub fn handle_ls(args: &[String]) -> Result<()> {
    let path = if args.is_empty() {
        "."
    } else {
        &args[0]
    };

    let entries = fs::read_dir(Path::new(path))
        .with_context(|| format!("Failed to read directory: {}", path))?;

    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name();
        println!("{}", file_name.to_string_lossy());
    }

    Ok(())
}