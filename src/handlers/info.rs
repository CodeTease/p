use anyhow::Result;
use std::env;
use colored::*;
use crate::config::load_config;

pub fn handle_info() -> Result<()> {
    let current_dir = env::current_dir()?;
    // We ignore error here? No, load_config returns Result.
    // But maybe we want to show info even if partial? 
    // load_config handles missing file by erroring.
    let config = load_config(&current_dir)?;

    println!();
    if let Some(project) = config.project {
        println!("{}", "📦 PROJECT SCOPE".green().bold());
        println!("{}", "================".green());
        print_metadata(&project.metadata);
    } else if let Some(module) = config.module {
        println!("{}", "🧩 MODULE SCOPE".cyan().bold());
        println!("{}", "===============".cyan());
        print_metadata(&module.metadata);
    } else {
        // This case should be caught by load_config validation technically if we enforced presence.
        // But structs are Option.
        println!("{}", "⚠️  No [project] or [module] definition found.".yellow());
    }
    println!();

    Ok(())
}

fn print_metadata(meta: &crate::config::Metadata) {
    if let Some(name) = &meta.name {
        println!("   Name:        {}", name.bold());
    }
    if let Some(ver) = &meta.version {
        println!("   Version:     {}", ver);
    }
    if let Some(desc) = &meta.description {
        println!("   Description: {}", desc.italic());
    }
    if let Some(authors) = &meta.authors {
        let joined = authors.join(", ");
        println!("   Authors:     {}", joined);
    }
}
