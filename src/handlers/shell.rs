use anyhow::{Context, Result, bail};
use colored::*;
use std::fs;
use std::path::{PathBuf};
use std::process::{Command, Stdio};
use std::env;
use std::io::{self, Write};
use crate::config::load_config;
use crate::utils::detect_shell;
use crate::pas;

pub fn handle_repl() -> Result<()> {
    // Signal handling: catch Ctrl+C to prevent shell exit
    ctrlc::set_handler(move || {
        print!("\n> ");
        io::stdout().flush().ok();
    }).context("Error setting Ctrl-C handler")?;

    // Load Config (Fail Closed)
    let current_dir = env::current_dir()?;
    let config_res = load_config(&current_dir);
    
    let config = match config_res {
        Ok(c) => Some(c),
        Err(e) => {
            if current_dir.join("p.toml").exists() {
                eprintln!("{} Configuration Error: {}", "❌".red(), e);
                bail!("Aborting shell session because p.toml exists but cannot be loaded. Fix the configuration to ensure security rules are applied.");
            }
            None
        }
    };

    let capabilities = config.as_ref().and_then(|c| c.capability.clone());

    let mut ctx = pas::context::ShellContext::new(capabilities);

    // Startup Profile
    if let Some(cfg) = &config {
        if let Some(pas_cfg) = &cfg.pas {
            if let Some(profile) = &pas_cfg.profile {
                 if let Some(startup) = &profile.startup {
                     println!("{}", "Initializing environment...".dimmed());
                     for cmd in startup {
                         match pas::run_command_line(cmd, &mut ctx) {
                             Ok(_) => {},
                             Err(e) => eprintln!("{} Startup command failed: {}", "⚠️".yellow(), e),
                         }
                     }
                 }
            }
        }
    }

    println!("Welcome to PaShell. Type 'exit' to quit.");

    loop {
        print!("> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            break; // EOF
        }
        
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        
        if input == "exit" {
            break;
        }
        
        // Run
        match pas::run_command_line(input, &mut ctx) {
            Ok(_) => {}, // Exit code stored in ctx
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    Ok(())
}

pub fn handle_dir_jump(target_path: PathBuf) -> Result<()> {
    if !target_path.exists() || !target_path.is_dir() {
        bail!("Target directory does not exist: {:?}", target_path);
    }

    let abs_path = fs::canonicalize(&target_path)?;
    // Now config includes merged envs from p.toml and .env
    let config = load_config(&abs_path)?;

    // Detect shell preference or fallback to system default
    let shell_pref = config.project.as_ref().and_then(|p| p.shell.as_ref())
        .or(config.module.as_ref().and_then(|m| m.shell.as_ref()));
    let shell_cmd = detect_shell(shell_pref);

    eprintln!("{} Entering environment at: {}", "⤵️".cyan(), abs_path.display());
    
    let mut command = Command::new(&shell_cmd);
    command.current_dir(&abs_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .envs(&config.env); // Inject merged envs

    let status = command.status()
        .context(format!("Failed to spawn shell: {}", shell_cmd))?;

    if !status.success() {
        eprintln!("{} Shell exited with non-zero code.", "⚠️".yellow());
    }

    // Output for external tools (like shell aliases) to capture the path
    if let Ok(output_file) = env::var("PAVIDI_OUTPUT") {
        fs::write(output_file, abs_path.to_string_lossy().as_bytes())
            .context("Failed to write jump path")?;
    } else {
        println!("{}", abs_path.to_string_lossy());
    }

    Ok(())
}
