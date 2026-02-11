// Mv portable handler

use anyhow::{Result, Context, bail};
use std::fs;
use std::path::Path;
use crate::runner::common::expand_globs;

pub fn handle_mv(args: &[String]) -> Result<()> {
    let expanded_args = expand_globs(args);

    let mut paths = Vec::new();
    // We ignore flags for now, but filter them out to avoid treating them as paths
    for arg in &expanded_args {
        if !arg.starts_with('-') {
            paths.push(arg);
        }
    }

    if paths.len() < 2 {
        bail!("mv command requires at least source and destination");
    }

    let dest = paths.pop().unwrap();
    let sources = paths;

    let dest_path = Path::new(dest);
    let dest_is_dir = dest_path.is_dir();

    if sources.len() > 1 && !dest_is_dir {
        bail!("Target '{}' is not a directory", dest);
    }

    for src in sources {
        let src_path = Path::new(src);
        if !src_path.exists() {
             bail!("Source not found: {}", src);
        }

        let target = if dest_is_dir {
            dest_path.join(src_path.file_name().ok_or_else(|| anyhow::anyhow!("Invalid source filename"))?)
        } else {
            dest_path.to_path_buf()
        };

        fs::rename(src_path, &target).with_context(|| format!("Failed to move from {:?} to {:?}", src_path, target))?;
    }

    Ok(())
}
