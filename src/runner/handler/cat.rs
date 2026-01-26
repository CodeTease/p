// Cat portable handler

use anyhow::{Result, Context};
use std::fs;
use std::path::Path;

pub fn handle_cat(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        println!("Usage: cat <file1> <file2> ...");
        return Ok(());
    }

    for filename in &args[1..] {
        let path = Path::new(filename);
        if !path.exists() {
            println!("cat: {}: No such file", filename);
            continue;
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", filename))?;
        print!("{}", content);
    }

    Ok(())
}