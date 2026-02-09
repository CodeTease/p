use anyhow::Result;
use colored::*;
use std::env;
use crate::config::{load_config, Metadata};

pub fn handle_info() -> Result<()> {
    let current_dir = env::current_dir()?;
    let config = load_config(&current_dir)?;

    let metadata: Option<&Metadata> = if let Some(p) = &config.project {
        Some(&p.metadata)
    } else if let Some(m) = &config.module {
        Some(&m.metadata)
    } else {
        None
    };

    if let Some(meta) = metadata {
        println!("{}", "Project Information".bold().underline());
        
        if let Some(name) = &meta.name {
            println!("{}: {}", "Name".cyan(), name);
        }
        if let Some(version) = &meta.version {
            println!("{}: {}", "Version".cyan(), version);
        }
        if let Some(desc) = &meta.description {
            println!("{}: {}", "Description".cyan(), desc);
        }
        if let Some(authors) = &meta.authors {
            if !authors.is_empty() {
                println!("{}: {}", "Authors".cyan(), authors.join(", "));
            }
        }
    } else {
        println!("{}", "No project/module metadata found.".yellow());
    }

    Ok(())
}
