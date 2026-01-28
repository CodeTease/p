use anyhow::Result;
use colored::*;
use crate::config::load_config;
use crate::runner::task::RunnerTask;

use std::env;

pub fn handle_list() -> Result<()> {
    let current_dir = env::current_dir()?;
    let config = load_config(&current_dir)?;
    
    if let Some(p) = &config.project {
        let name = p.metadata.name.as_deref().unwrap_or("Unnamed Project");
        println!("{} {} {}", "📦".green(), name.bold(), "(Project)".dimmed());
    } else if let Some(m) = &config.module {
        let name = m.metadata.name.as_deref().unwrap_or("Unnamed Module");
        println!("{} {} {}", "🧩".cyan(), name.bold(), "(Module)".dimmed());
    }
    println!();

    if let Some(runner_tasks) = config.runner {
        println!("{}", "Available Tasks:".bold().underline());
        
        let mut max_len = 0;
        let mut tasks: Vec<(&String, Option<&String>)> = Vec::new();

        for (name, task) in &runner_tasks {
            if name.len() > max_len {
                max_len = name.len();
            }
            
            let desc = match task {
                RunnerTask::Full { description, .. } => description.as_ref(),
                _ => None,
            };
            tasks.push((name, desc));
        }
        
        // Sort for consistent output
        tasks.sort_by(|a, b| a.0.cmp(b.0));

        for (name, desc) in tasks {
            let padding = " ".repeat(max_len - name.len() + 2);
            let empty_string = String::new();
            let description = desc.unwrap_or(&empty_string);
            println!("  {}{}{}", name.cyan(), padding, description.italic());
        }
    } else {
        println!("No tasks defined in configuration.");
    }

    Ok(())
}
