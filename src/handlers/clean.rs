use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::env;
use crate::config::load_config;

pub fn handle_clean() -> Result<()> {
    let current_dir = env::current_dir()?;
    let config = load_config(&current_dir)?;
    let clean_section = config.clean.context("No [clean] section defined in config")?;

    println!("{} Cleaning targets...", "🧹".red());
    for pattern in clean_section.targets {
        let full_pattern = format!("{}/{}", current_dir.to_string_lossy(), pattern);
        for entry in glob::glob(&full_pattern)? {
            if let Ok(path) = entry {
                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                    println!("   Deleted dir: {:?}", path.file_name().unwrap());
                } else {
                    fs::remove_file(&path)?;
                    println!("   Deleted file: {:?}", path.file_name().unwrap());
                }
            }
        }
    }
    Ok(())
}
