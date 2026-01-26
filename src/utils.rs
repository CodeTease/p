use anyhow::{Context, Result, bail};
use colored::*;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::env;
use log::{info, error};
use wait_timeout::ChildExt;
use std::time::Duration;
use std::io::Read;
use regex::Regex;

/// Replaces $1, $2... with corresponding args.
/// Then replaces ${VAR} or $VAR with values from env_vars.
/// Fallback for args: If no placeholders found, append args to the end.
pub fn expand_command(cmd_template: &str, args: &[String], env_vars: &HashMap<String, String>) -> String {
    let mut expanded = cmd_template.to_string();
    
    // 1. Argument Substitution ($1, $2...)
    if !args.is_empty() {
        let mut replaced_args = false;
        for (i, arg) in args.iter().enumerate() {
            let placeholder = format!("${}", i + 1);
            if expanded.contains(&placeholder) {
                expanded = expanded.replace(&placeholder, arg);
                replaced_args = true;
            }
        }

        // Backward Compatibility: Append if no placeholders used
        if !replaced_args {
            expanded.push_str(" ");
            expanded.push_str(&args.join(" "));
        }
    }

    // 2. Env Var Interpolation (${VAR} or $VAR)
    // Matches ${VAR} or $VAR (variable name must start with letter/underscore)
    // This avoids matching $1, $2 which are handled above (and usually don't match [a-zA-Z_])
    let re = Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}|\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    
    expanded = re.replace_all(&expanded, |caps: &regex::Captures| {
        let key = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str()).unwrap_or("");
        match env_vars.get(key) {
            Some(val) => val.to_string(),
            None => caps.get(0).unwrap().as_str().to_string(), // Keep original if not found
        }
    }).to_string();
    
    expanded
}

pub fn run_shell_command(
    cmd_str: &str, 
    env_vars: &HashMap<String, String>, 
    capture: bool,
    task_label: &str,
    shell_cmd: &str,
    timeout: Option<Duration>
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
    } else {
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());
    }

    let mut child = command.spawn().context("Failed to spawn shell process")?;

    let status = match timeout {
        Some(t) => {
            match child.wait_timeout(t).context("Failed to wait on child")? {
                Some(status) => status,
                None => {
                    // Timeout occurred, kill the child
                    let _ = child.kill();
                    child.wait().context("Failed to wait on killed child")?;
                    bail!("Execution timed out after {:?}", t);
                }
            }
        },
        None => child.wait().context("Failed to wait on child")?,
    };

    if capture {
        if let Some(mut stdout_pipe) = child.stdout.take() {
            let mut stdout = String::new();
            stdout_pipe.read_to_string(&mut stdout).unwrap_or_default();
            if !stdout.trim().is_empty() {
                info!("[{}] {}", task_label.cyan(), stdout.trim());
            }
        }
        
        if let Some(mut stderr_pipe) = child.stderr.take() {
            let mut stderr = String::new();
            stderr_pipe.read_to_string(&mut stderr).unwrap_or_default();
            if !stderr.trim().is_empty() {
                error!("[{}] {}", task_label.red(), stderr.trim());
            }
        }
    }

    if !status.success() {
        bail!("Exit code: {:?}", status.code());
    }

    Ok(())
}

pub fn detect_shell(config_shell: Option<&String>) -> String {
    if let Some(s) = config_shell {
        return s.clone();
    }
    
    if let Ok(s) = env::var("SHELL") {
        return s;
    }

    if cfg!(windows) {
        if which::which("powershell").is_ok() {
            "powershell".to_string() // Built-in PS 5. Always available so we prefer it
        } else if which::which("pwsh").is_ok() {
            "pwsh".to_string() // PS 6+ Core. Available if user installed it
        } else {
            "cmd".to_string() // Final fallback. CMD also always available
        }
    } else {
        "sh".to_string()
    }
}
