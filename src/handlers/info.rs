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
        
        let original = config.original_metadata.as_ref();

        // Helper to print diff
        let print_diff = |label: &str, current: &Option<String>, orig_val_opt: Option<&String>| {
            if let Some(curr_val) = current {
                 print!("{}: {}", label.cyan(), curr_val);
                 if let Some(orig_val) = orig_val_opt {
                     if curr_val != orig_val {
                         print!(" {} (was {})", "(modified)".yellow().italic(), orig_val.dimmed());
                     }
                 } else if original.is_some() {
                     print!(" {} (new)", "(added)".green().italic());
                 }
                 println!("");
            }
        };

        print_diff("Name", &meta.name, original.and_then(|m| m.name.as_ref()));
        print_diff("Version", &meta.version, original.and_then(|m| m.version.as_ref()));
        print_diff("Description", &meta.description, original.and_then(|m| m.description.as_ref()));
        
        if let Some(authors) = &meta.authors {
            if !authors.is_empty() {
                print!("{}: {}", "Authors".cyan(), authors.join(", "));
                 // Check if modified
                 let orig_authors = original.and_then(|m| m.authors.as_ref());
                 if let Some(orig) = orig_authors {
                     if authors != orig {
                         print!(" {}", "(modified)".yellow().italic());
                     }
                 } else if original.is_some() {
                     print!(" {}", "(new)".green().italic());
                 }
                 println!("");
            }
        }
    } else {
        println!("{}", "No project/module metadata found.".yellow());
    }

    println!("\n{}", "Extensions Applied".bold().underline());
    if !config.extensions_applied.is_empty() {
        for (name, meta) in &config.extensions_applied {
             print!("- {}", name.green());
             if let Some(ver) = &meta.version {
                 print!(" (v{})", ver);
             }
             if let Some(desc) = &meta.description {
                 print!(": {}", desc.dimmed());
             }
             println!("");
        }
    } else {
        println!("{}", "  (none)".dimmed());
    }

    Ok(())
}
