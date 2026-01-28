use anyhow::{Context, Result, bail};
use colored::*;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::env;
use log::{info, error};
use wait_timeout::ChildExt;
use std::time::Duration;
use std::io::{BufReader, BufRead};
use regex::Regex;
use std::thread;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CaptureMode {
    Inherit,
    Buffer,
    Tee,
}

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
    mode: CaptureMode,
    task_label: &str,
    shell_cmd: &str,
    timeout: Option<Duration>
) -> Result<(i32, String)> {
    let flag = if shell_cmd.contains("cmd") && !shell_cmd.contains("sh") { 
        "/C" 
    } else { 
        "-c" 
    };

    let mut command = Command::new(shell_cmd);
    command.arg(flag)
           .arg(cmd_str)
           .envs(env_vars)
           .stdin(Stdio::inherit()); 

    match mode {
        CaptureMode::Inherit => {
            command.stdout(Stdio::inherit());
            command.stderr(Stdio::inherit());
        },
        CaptureMode::Buffer | CaptureMode::Tee => {
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
        }
    }

    let mut child = command.spawn().context("Failed to spawn shell process")?;
    
    // For logging (merged)
    let captured_log = Arc::new(Mutex::new(String::new()));
    
    // For Buffer mode printing (separated)
    let captured_stdout = if mode == CaptureMode::Buffer { Some(Arc::new(Mutex::new(String::new()))) } else { None };
    let captured_stderr = if mode == CaptureMode::Buffer { Some(Arc::new(Mutex::new(String::new()))) } else { None };

    let mut threads = vec![];

    if mode != CaptureMode::Inherit {
        if let Some(stdout) = child.stdout.take() {
            let log_clone = captured_log.clone();
            let buf_clone = captured_stdout.clone();
            let mode_clone = mode;
            threads.push(thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        if mode_clone == CaptureMode::Tee {
                            println!("{}", l);
                        }
                        
                        let mut g_log = log_clone.lock().unwrap();
                        g_log.push_str(&l);
                        g_log.push('\n');

                        if let Some(buf) = &buf_clone {
                            let mut g_buf = buf.lock().unwrap();
                            g_buf.push_str(&l);
                            g_buf.push('\n');
                        }
                    }
                }
            }));
        }
        
        if let Some(stderr) = child.stderr.take() {
            let log_clone = captured_log.clone();
            let buf_clone = captured_stderr.clone();
            let mode_clone = mode;
            threads.push(thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        if mode_clone == CaptureMode::Tee {
                            eprintln!("{}", l);
                        }

                        let mut g_log = log_clone.lock().unwrap();
                        g_log.push_str(&l);
                        g_log.push('\n');

                        if let Some(buf) = &buf_clone {
                            let mut g_buf = buf.lock().unwrap();
                            g_buf.push_str(&l);
                            g_buf.push('\n');
                        }
                    }
                }
            }));
        }
    }

    let status = match timeout {
        Some(t) => {
            match child.wait_timeout(t).context("Failed to wait on child")? {
                Some(status) => status,
                None => {
                    let _ = child.kill();
                    child.wait().context("Failed to wait on killed child")?;
                    bail!("Execution timed out after {:?}", t);
                }
            }
        },
        None => child.wait().context("Failed to wait on child")?,
    };

    // Wait for readers to finish
    for t in threads {
        let _ = t.join();
    }

    let final_log = if mode != CaptureMode::Inherit {
        let log = captured_log.lock().unwrap().clone();

        if mode == CaptureMode::Buffer {
             if let Some(stdout_buf) = captured_stdout {
                 let s = stdout_buf.lock().unwrap();
                 if !s.trim().is_empty() {
                     info!("[{}] {}", task_label.cyan(), s.trim());
                 }
             }
             if let Some(stderr_buf) = captured_stderr {
                 let s = stderr_buf.lock().unwrap();
                 if !s.trim().is_empty() {
                     error!("[{}] {}", task_label.red(), s.trim());
                 }
             }
        }
        log
    } else {
        String::new()
    };

    let code = status.code().unwrap_or(1);
    
    if !status.success() {
         return Ok((code, final_log));
    }

    Ok((0, final_log))
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
            "powershell".to_string() 
        } else if which::which("pwsh").is_ok() {
            "pwsh".to_string() 
        } else {
            "cmd".to_string() 
        }
    } else {
        "sh".to_string()
    }
}
