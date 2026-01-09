use anyhow::{Context, Result, bail};
use colored::*;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::env;

/// Replaces $1, $2... with corresponding args.
/// Fallback: If no placeholders found, append args to the end.
pub fn expand_command(cmd_template: &str, args: &[String]) -> String {
    if args.is_empty() {
        return cmd_template.to_string();
    }

    let mut expanded = cmd_template.to_string();
    let mut replaced = false;

    for (i, arg) in args.iter().enumerate() {
        let placeholder = format!("${}", i + 1);
        if expanded.contains(&placeholder) {
            expanded = expanded.replace(&placeholder, arg);
            replaced = true;
        }
    }

    // Backward Compatibility: Append if no placeholders used
    if !replaced {
        expanded.push_str(" ");
        expanded.push_str(&args.join(" "));
    }
    
    expanded
}

pub fn run_shell_command(
    cmd_str: &str, 
    env_vars: &HashMap<String, String>, 
    capture: bool,
    task_label: &str,
    shell_cmd: &str
) -> Result<()> {
    // Determine flag based on shell
    // Simple heuristic: "cmd" or "cmd.exe" uses /C, others use -c
    // This allows for powershell -c, bash -c, zsh -c, fish -c
    let flag = if shell_cmd.contains("cmd") && !shell_cmd.contains("sh") { 
        "/C" 
    } else { 
        "-c" 
    };

    // shell_cmd might contain arguments (though unlikely from config currently, 
    // config usually just gives the bin path).
    // detect_shell returns a String, we treat it as the command.
    
    let mut command = Command::new(shell_cmd);
    command.arg(flag)
           .arg(cmd_str)
           .envs(env_vars) // Use merged envs
           .stdin(Stdio::inherit()); 

    if capture {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.output().context("Failed to spawn shell process (captured)")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.trim().is_empty() {
            println!("[{}] {}", task_label.cyan(), stdout.trim());
        }
        if !stderr.trim().is_empty() {
            eprintln!("[{}] {}", task_label.red(), stderr.trim());
        }

        if !output.status.success() {
            bail!("Exit code: {:?}", output.status.code());
        }
    } else {
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        let status = command.status().context("Failed to spawn shell process")?;
        if !status.success() {
            bail!("Exit code: {:?}", status.code());
        }
    }

    Ok(())
}

pub fn detect_shell(config_shell: Option<&String>) -> String {
    config_shell
        .cloned()
        .or_else(|| env::var("SHELL").ok())
        .unwrap_or_else(|| if cfg!(windows) { "cmd".to_string() } else { "sh".to_string() })
}
