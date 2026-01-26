// Rm portable handler

use anyhow::{Result, Context, bail};
use std::fs;
use std::path::Path;

pub fn handle_rm(args: &[String]) -> Result<()> {
    let mut recursive = false;
    let mut force = false;
    let mut paths = Vec::new();

    for arg in args {
        if arg.starts_with('-') {
            if arg.contains('r') || arg.contains('R') { recursive = true; }
            if arg.contains('f') { force = true; }
        } else {
            paths.push(arg);
        }
    }

    for path in paths {
        let p = Path::new(path);
        if !p.exists() {
            if !force {
                bail!("File not found: {}", path);
            }
            continue;
        }

        if p.is_dir() {
            if recursive {
                fs::remove_dir_all(p).with_context(|| format!("Failed to remove directory: {}", path))?;
            } else {
                bail!("Cannot remove directory '{}' without -r", path);
            }
        } else {
            fs::remove_file(p).with_context(|| format!("Failed to remove file: {}", path))?;
        }
    }
    Ok(())
}