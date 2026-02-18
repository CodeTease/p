use anyhow::Result;
use colored::*;
use std::env;
use std::collections::HashSet;
use crate::config::load_config;
use crate::cli::Cli;

pub fn handle_env(cli: &Cli) -> Result<()> {
    let current_dir = env::current_dir()?;
    // Load config which merges p.toml and .env
    let config = load_config(&current_dir)?;

    if cli.trace {
        println!("{} Environment Variable Trace:", "🔍".cyan());
        
        let mut keys: Vec<&String> = config.env_provenance.keys().collect();
        keys.sort();

        for key in keys {
            let history = &config.env_provenance[key];
            println!("{}:", key.bold());
            for (idx, (source, val)) in history.iter().enumerate() {
                let prefix = if idx == history.len() - 1 { "└──".green() } else { "├──".blue() };
                println!("  {} {} = {} ({})", prefix, source, val, if idx == history.len() - 1 { "active".green() } else { "overridden".red().dimmed() });
            }
        }
    } else {
        println!("{} Environment Variables (Layered):", "🔍".cyan());
        
        // Identify all unique sources involved, preserving order if possible
        let mut ordered_sources = Vec::new();
        let mut seen_sources = HashSet::new();

        // 1. p.toml (always first if present)
        if config.env_provenance.values().any(|h| h.iter().any(|(s, _)| s == "p.toml")) {
            ordered_sources.push("p.toml".to_string());
            seen_sources.insert("p.toml".to_string());
        }

        // 2. Extensions (in applied order)
        for (ext_name, _) in &config.extensions_applied {
            if seen_sources.insert(ext_name.clone()) {
                ordered_sources.push(ext_name.clone());
            }
        }

        // 3. Other sources (.env, dynamic) found in provenance
        // We'll collect them and append them. Typically .env is last.
        // But we want to preserve some logical order.
        let mut other_sources = Vec::new();
        for history in config.env_provenance.values() {
            for (source, _) in history {
                if !seen_sources.contains(source) {
                    if seen_sources.insert(source.clone()) {
                        other_sources.push(source.clone());
                    }
                }
            }
        }
        // sort other sources alphabetically or just append? usually .env should be last.
        other_sources.sort(); 
        ordered_sources.extend(other_sources);

        for source in ordered_sources {
            println!("\n[{}]", source.yellow().bold());
            
            // Find vars defined/modified in this source
            let mut vars_in_source = Vec::new();
            
            for (key, history) in &config.env_provenance {
                // Find the index of this source in the history
                if let Some(pos) = history.iter().position(|(s, _)| s == &source) {
                    let val = &history[pos].1;
                    let is_active = pos == history.len() - 1;
                    vars_in_source.push((key, val, is_active));
                }
            }
            
            vars_in_source.sort_by_key(|k| k.0);
            
            if vars_in_source.is_empty() {
                println!("  (none)");
            }

            for (key, val, is_active) in vars_in_source {
                if is_active {
                     println!("  {} = {}", key.bold(), val);
                } else {
                     // Show as overridden
                     println!("  {} = {} {}", key.dimmed().strikethrough(), val.dimmed().strikethrough(), "(overridden)".red().italic());
                }
            }
        }
    }

    Ok(())
}
