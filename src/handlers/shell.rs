use anyhow::{Context, Result, bail};
use colored::*;
use std::fs;
use std::path::{PathBuf};
use std::process::{Command, Stdio};
use std::env;
use crate::config::load_config;
use crate::utils::detect_shell;

pub fn handle_dir_jump(target_path: PathBuf) -> Result<()> {
    if !target_path.exists() || !target_path.is_dir() {
        bail!("Target directory does not exist: {:?}", target_path);
    }

    let abs_path = fs::canonicalize(&target_path)?;
    // Now config includes merged envs from p.toml and .env
    let config = load_config(&abs_path)?;

    // Detect shell preference or fallback to system default
    let shell_cmd = detect_shell(config.project.as_ref().and_then(|p| p.shell.as_ref()));

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
