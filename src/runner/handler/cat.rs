// Cat portable handler

use anyhow::{Result, Context};
use std::fs;
use std::io;
use std::path::Path;
use crate::runner::common::expand_globs;

pub fn handle_cat(args: &[String]) -> Result<()> {
    let expanded_args = expand_globs(args);

    if expanded_args.is_empty() {
        println!("Usage: cat <file1> <file2> ...");
        return Ok(());
    }

    for filename in &expanded_args {
        let path = Path::new(filename);
        if !path.exists() {
            println!("cat: {}: No such file", filename);
            continue;
        }
        
        if path.is_dir() {
            println!("cat: {}: Is a directory", filename);
            continue;
        }

        let mut file = fs::File::open(path)
            .with_context(|| format!("Failed to open file: {}", filename))?;
        io::copy(&mut file, &mut io::stdout())
            .with_context(|| format!("Failed to read file: {}", filename))?;
    }

    Ok(())
}
