// Mkdir portable handler

use anyhow::{Result, Context};
use std::fs;

pub fn handle_mkdir(args: &[String]) -> Result<()> {
    let mut parents = false;
    let mut paths = Vec::new();

    for arg in args {
        if arg == "-p" {
            parents = true;
        } else if arg.starts_with('-') {
            // Ignore other flags
        } else {
            paths.push(arg);
        }
    }

    for path in paths {
        if parents {
            fs::create_dir_all(path).with_context(|| format!("Failed to create directory (with parents): {}", path))?;
        } else {
            fs::create_dir(path).with_context(|| format!("Failed to create directory: {}", path))?;
        }
    }
    Ok(())
}