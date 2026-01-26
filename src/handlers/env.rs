use anyhow::Result;
use colored::*;
use std::env;
use crate::config::load_config;

pub fn handle_env() -> Result<()> {
    let current_dir = env::current_dir()?;
    // Load config which merges p.toml and .env
    let config = load_config(&current_dir)?;

    println!("{} Merged Environment Variables:", "🔍".cyan());
    
    // Sort keys for better readability
    let mut keys: Vec<&String> = config.env.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(val) = config.env.get(key) {
            println!("{} = {}", key.bold(), val);
        }
    }

    Ok(())
}
