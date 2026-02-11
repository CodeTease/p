// Ls portable handler

use anyhow::{Result, Context};
use std::fs;
use std::path::Path;
use crate::runner::common::expand_globs;

pub fn handle_ls(args: &[String]) -> Result<()> {
    let mut expanded_args = expand_globs(args);

    if expanded_args.is_empty() {
        expanded_args.push(".".to_string());
    }

    let show_header = expanded_args.len() > 1;

    for path_str in expanded_args {
        let path = Path::new(&path_str);
        if !path.exists() {
             println!("ls: {}: No such file or directory", path_str);
             continue;
        }

        if path.is_dir() {
            if show_header {
                println!("{}:", path_str);
            }
            
            let mut entries_vec = Vec::new();
            let read_dir = fs::read_dir(path).with_context(|| format!("Failed to read directory: {}", path_str))?;
            
            for entry in read_dir {
                entries_vec.push(entry?.file_name());
            }
            
            // Sort for consistent output
            entries_vec.sort();

            for name in entries_vec {
                println!("{}", name.to_string_lossy());
            }
        } else {
            println!("{}", path_str);
        }
    }
    Ok(())
}
