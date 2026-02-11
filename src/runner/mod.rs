pub mod task;
pub mod cache;
pub mod portable;
pub mod handler;
pub mod common;

use anyhow::{Result, bail};
use colored::*;
use std::collections::HashSet;
use std::time::Duration;
use rayon::prelude::*;
use crate::config::PavidiConfig;
use crate::utils::{detect_shell, expand_command, run_shell_command, CaptureMode};
use crate::logger::write_log;
use self::task::RunnerTask;
use self::cache::{is_up_to_date, save_cache};
use self::portable::run_portable_command;
use log::{info, error};
use std::time::Instant;
use std::thread;

pub struct CallStack {
    stack: HashSet<String>,
}

impl CallStack {
    pub fn new() -> Self {
        Self {
            stack: HashSet::new(),
        }
    }

    pub fn push(&mut self, task_name: &str) -> Result<()> {
        if self.stack.contains(task_name) {
            bail!("🔄 Circular dependency detected: {}", task_name);
        }
        self.stack.insert(task_name.to_string());
        Ok(())
    }

    pub fn pop(&mut self, task_name: &str) {
        self.stack.remove(task_name);
    }

    pub fn clone_stack(&self) -> Self {
        Self {
            stack: self.stack.clone(),
        }
    }
}

pub fn recursive_runner(
    task_name: &str, 
    config: &PavidiConfig, 
    call_stack: &mut CallStack,
    extra_args: &[String],
    capture_output: bool, // true = buffer output (for parallel), false = inherit
    dry_run: bool
) -> Result<()> {
    call_stack.push(task_name)?;

    let runner_section = config.runner.as_ref().unwrap();
    let task = runner_section.get(task_name).expect("Task check passed before");

    // Destructure task config
    let (mut cmds, deps, parallel_deps, run_if, skip_if, sources, outputs, windows, linux, macos, ignore_failure, timeout_sec, retry, retry_delay) = match task {
        RunnerTask::Single(cmd) => (vec![cmd.clone()], vec![], false, None, None, None, None, None, None, None, false, None, None, None),
        RunnerTask::List(cmds) => (cmds.clone(), vec![], false, None, None, None, None, None, None, None, false, None, None, None),
        RunnerTask::Full { cmds, deps, parallel, run_if, skip_if, sources, outputs, windows, linux, macos, ignore_failure, timeout, retry, retry_delay, .. } => 
            (cmds.clone(), deps.clone(), *parallel, run_if.clone(), skip_if.clone(), sources.clone(), outputs.clone(), windows.clone(), linux.clone(), macos.clone(), *ignore_failure, *timeout, *retry, *retry_delay),
    };

    // 1. Run Dependencies
    if !deps.is_empty() {
        if parallel_deps {
            if !capture_output {
                info!("{} Running dependencies in parallel: {:?}...", "🚀".cyan(), deps);
            }
            
            // Snapshot the stack to avoid capturing &mut CallStack in the closure
            let stack_snapshot = call_stack.clone_stack();

            // Rayon parallel iterator
            let errors: Vec<String> = deps
                .par_iter()
                .map(|dep_name| {
                    let mut local_stack = stack_snapshot.clone_stack();
 
                    // Parallel deps MUST capture output to prevent mixed logs
                    recursive_runner(dep_name, config, &mut local_stack, &[], true, dry_run)
                        .map_err(|e| format!("Dep '{}' failed: {}", dep_name, e))
                })
                .filter_map(|res| res.err())
                .collect();

            if !errors.is_empty() {
                for e in &errors { error!("{} {}", "❌".red(), e); }
                bail!("Dependency execution failed.");
            }
        } else {
            if !capture_output {
                info!("{} Running dependencies sequentially...", "🔗".blue());
            }
            for dep in deps {
                recursive_runner(&dep, config, call_stack, &[], capture_output, dry_run)?;
            }
        }
    }

    // 2. Logic Gates (Conditional Execution)
    // Detect shell (needed for condition checks)
    let shell_pref = config.project.as_ref().and_then(|p| p.shell.as_ref())
        .or(config.module.as_ref().and_then(|m| m.shell.as_ref()));
    let shell_cmd = detect_shell(shell_pref);

    // skip_if
    if let Some(raw_cmd) = skip_if {
        let cmd = expand_command(&raw_cmd, extra_args, &config.env);
        // Silent execution
        let (code, _) = run_shell_command(&cmd, &config.env, CaptureMode::Buffer, task_name, &shell_cmd, None)?;
        if code == 0 {
            if !capture_output {
                info!("{} Skipping task '{}' because 'skip_if' condition met.", "⏭️".yellow(), task_name.bold());
            }
            call_stack.pop(task_name);
            return Ok(());
        }
    }

    // run_if
    if let Some(raw_cmd) = run_if {
        let cmd = expand_command(&raw_cmd, extra_args, &config.env);
        // Silent execution
        let (code, _) = run_shell_command(&cmd, &config.env, CaptureMode::Buffer, task_name, &shell_cmd, None)?;
        if code != 0 {
            if !capture_output {
                info!("{} Skipping task '{}' because 'run_if' condition failed.", "⏭️".yellow(), task_name.bold());
            }
            call_stack.pop(task_name);
            return Ok(());
        }
    }

    // 3. Check Conditional Execution (Cache Check)
    if let (Some(srcs), Some(outs)) = (&sources, &outputs) {
        if is_up_to_date(task_name, srcs, outs, &config.env)? {
            if !capture_output {
                info!("{} Task '{}' is up-to-date. Skipping.", "✨".green(), task_name.bold());
            }
            call_stack.pop(task_name);
            return Ok(());
        }
    }

    // 4. Execute Main Commands

    // OS Detection & Command Selection
    let os = std::env::consts::OS;
    let os_cmds = match os {
        "windows" => windows.as_ref(),
        "linux" => linux.as_ref(),
        "macos" => macos.as_ref(),
        _ => None,
    };

    if let Some(c) = os_cmds {
        cmds = c.clone();
    } 

    let has_os_config = windows.is_some() || linux.is_some() || macos.is_some();
    if cmds.is_empty() && has_os_config {
         bail!("No commands defined for this OS ({})", os);
    }

    if !cmds.is_empty() {
        if !capture_output {
            info!("{} Running task: {}", "⚡".yellow(), task_name.bold());
        }

        // Log configuration
        let (log_strategy, _) = if let Some(p) = &config.project {
            (p.log_strategy, p.log_plain)
        } else if let Some(m) = &config.module {
            (m.log_strategy, m.log_plain)
        } else {
            (None, None)
        };
        let log_enabled = log_strategy.unwrap_or(crate::config::LogStrategy::None) != crate::config::LogStrategy::None;

        let capture_mode = if capture_output {
            CaptureMode::Buffer
        } else {
            if log_enabled {
                CaptureMode::Tee
            } else {
                CaptureMode::Inherit
            }
        };
        
        // Note: shell_cmd was already detected above, reusing it.

        let timeout_duration = match timeout_sec {
            Some(0) => None,
            Some(s) => Some(Duration::from_secs(s)),
            None => Some(Duration::from_secs(1800)),
        };

        let max_retries = retry.unwrap_or(0);
        let retry_delay_duration = Duration::from_secs(retry_delay.unwrap_or(0));

        for cmd in &mut cmds {
            // Apply Argument Expansion ($1, $2...) and Env Var Interpolation
            let final_cmd = expand_command(cmd, extra_args, &config.env);

            if dry_run {
                println!("{} [DRY-RUN] Executing: {}", "::".yellow(), final_cmd);
                continue;
            }

            if !capture_output {
                info!("{} Executing: {}", "::".blue(), final_cmd);
            }

            let mut attempt = 0;
            
            loop {
                let start_time = Instant::now();
                let mut captured_output = String::new();
                let mut exit_code = 0;
                let mut execution_failed = false;
                let mut execution_error = String::new();

                // Fallback to legacy portable/shell command
                if final_cmd.trim_start().starts_with("p:") {
                        if let Err(e) = run_portable_command(&final_cmd) {
                            execution_failed = true;
                            execution_error = e.to_string();
                            exit_code = 1;
                        }
                } else {
                    let result = run_shell_command(&final_cmd, &config.env, capture_mode, task_name, &shell_cmd, timeout_duration);
                    
                    match result {
                        Ok((code, output)) => {
                            captured_output = output;
                            exit_code = code;
                            if code != 0 {
                                execution_failed = true;
                            }
                        },
                        Err(e) => {
                            execution_failed = true;
                            execution_error = e.to_string();
                            exit_code = 1;
                        }
                    }
                }
                
                if !execution_failed {
                    // Success
                    if log_enabled {
                         if let Ok(Some(path)) = write_log(task_name, &final_cmd, &captured_output, config, start_time.elapsed(), exit_code, &config.env) {
                             info!("{} Log saved: {}", "📝".dimmed(), path.display());
                         }
                    }
                    break;
                } else {
                    // Failure
                    if log_enabled {
                        let log_content = if !execution_error.is_empty() {
                            format!("Execution Error: {}", execution_error)
                        } else {
                            captured_output.clone()
                        };
                         let _ = write_log(task_name, &final_cmd, &log_content, config, start_time.elapsed(), exit_code, &config.env);
                    }

                    if attempt < max_retries {
                        attempt += 1;
                        if !capture_output {
                            info!("{} Command failed. Retrying ({}/{}) in {}s...", "🔄".yellow(), attempt, max_retries, retry_delay_duration.as_secs());
                        }
                        thread::sleep(retry_delay_duration);
                        continue;
                    } else {
                        // All retries failed
                        if ignore_failure {
                             if !execution_error.is_empty() {
                                log::warn!("{} Command failed but ignored: {}", "⚠️".yellow(), execution_error);
                             } else {
                                log::warn!("{} Command failed but ignored (code {})", "⚠️".yellow(), exit_code);
                             }
                             break;
                        } else {
                             if !execution_error.is_empty() {
                                bail!("❌ Task '{}' failed at: '{}' -> {}", task_name, final_cmd, execution_error);
                             } else {
                                bail!("❌ Task '{}' failed at: '{}' -> Exit code {}", task_name, final_cmd, exit_code);
                             }
                        }
                    }
                }
            } // end loop
        } // end for cmd

        // Success: Update cache if sources AND outputs defined (otherwise we never check it anyway)
        if let (Some(srcs), Some(_)) = (&sources, &outputs) {
             save_cache(task_name, srcs, &config.env)?;
        }
    }
    
    call_stack.pop(task_name);
    Ok(())
}
