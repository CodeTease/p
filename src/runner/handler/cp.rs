// Cp portable handler

use anyhow::{Result, Context, bail};
use std::fs;
use std::path::Path;
use crate::runner::common::copy_dir_recursive;
use crate::runner::common::expand_globs;

pub fn handle_cp(args: &[String]) -> Result<()> {
    let expanded_args = expand_globs(args);

    let mut recursive = false;
    let mut paths = Vec::new();

    for arg in &expanded_args {
        if arg == "-r" || arg == "-R" || arg == "--recursive" {
            recursive = true;
        } else {
            paths.push(arg);
        }
    }

    if paths.len() < 2 {
        bail!("cp requires at least source and destination");
    }

    let dest = paths.pop().unwrap(); // Last one is dest
    let sources = paths;

    let dest_path = Path::new(&dest);
    let dest_is_dir = dest_path.is_dir();

    if sources.len() > 1 && !dest_is_dir {
        bail!("Target '{}' is not a directory", dest);
    }

    for src in sources {
        let src_path = Path::new(src); // No need for &src here as src is String (actually &String if from &expanded_args, wait)
        if !src_path.exists() {
            bail!("Source not found: {}", src);
        }

        let target = if dest_is_dir {
            dest_path.join(src_path.file_name().ok_or_else(|| anyhow::anyhow!("Invalid source filename"))?)
        } else {
            dest_path.to_path_buf()
        };

        if src_path.is_dir() {
            if recursive {
                copy_dir_recursive(src_path, &target)?;
            } else {
                bail!("Omitting directory '{}' (use -r to copy)", src);
            }
        } else {
            fs::copy(src_path, &target).with_context(|| format!("Failed to copy {} to {}", src, target.display()))?;
        }
    }

    Ok(())
}
