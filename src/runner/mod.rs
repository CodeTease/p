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
use crate::utils::{detect_shell, expand_command, run_shell_command};
use crate::pas::context::ShellContext;
use crate::pas::run_command_line;
use self::task::RunnerTask;
use self::cache::{is_up_to_date, save_cache};
use self::portable::run_portable_command;
use log::{info, error};

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
    dry_run: bool,
    mut context: Option<&mut ShellContext>
) -> Result<()> {
    call_stack.push(task_name)?;

    let runner_section = config.runner.as_ref().unwrap();
    let task = runner_section.get(task_name).expect("Task check passed before");

    // Destructure task config
    let (mut cmds, deps, parallel_deps, sources, outputs, windows, linux, macos, ignore_failure, timeout_sec) = match task {
        RunnerTask::Single(cmd) => (vec![cmd.clone()], vec![], false, None, None, None, None, None, false, None),
        RunnerTask::List(cmds) => (cmds.clone(), vec![], false, None, None, None, None, None, false, None),
        RunnerTask::Full { cmds, deps, parallel, sources, outputs, windows, linux, macos, ignore_failure, timeout, .. } => 
            (cmds.clone(), deps.clone(), *parallel, sources.clone(), outputs.clone(), windows.clone(), linux.clone(), macos.clone(), *ignore_failure, *timeout),
    };

    // 1. Run Dependencies
    if !deps.is_empty() {
        if parallel_deps {
            if !capture_output {
                info!("{} Running dependencies in parallel: {:?}...", "🚀".cyan(), deps);
            }
            
            // Snapshot the stack to avoid capturing &mut CallStack in the closure
            let stack_snapshot = call_stack.clone_stack();
            // Snapshot context for parallel execution
            let context_snapshot = context.as_ref().map(|c| (**c).clone());

            // Rayon parallel iterator
            let errors: Vec<String> = deps
                .par_iter()
                .map(|dep_name| {
                    let mut local_stack = stack_snapshot.clone_stack();
                    // Clone context for this thread
                    let mut local_ctx_val = context_snapshot.clone();
                    let local_ctx = local_ctx_val.as_mut();
 
                    // Parallel deps MUST capture output to prevent mixed logs
                    recursive_runner(dep_name, config, &mut local_stack, &[], true, dry_run, local_ctx)
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
                recursive_runner(&dep, config, call_stack, &[], capture_output, dry_run, context.as_deref_mut())?;
            }
        }
    }

    // 2. Check Conditional Execution (Cache Check)
    if let (Some(srcs), Some(outs)) = (&sources, &outputs) {
        if is_up_to_date(task_name, srcs, outs)? {
            if !capture_output {
                info!("{} Task '{}' is up-to-date. Skipping.", "✨".green(), task_name.bold());
            }
            call_stack.pop(task_name);
            return Ok(());
        }
    }

    // 3. Execute Main Commands

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

        // Optimize Core Logic - detect shell
        let shell_cmd = detect_shell(config.project.as_ref().and_then(|p| p.shell.as_ref()));
        
        let timeout_duration = match timeout_sec {
            Some(0) => None,
            Some(s) => Some(Duration::from_secs(s)),
            None => Some(Duration::from_secs(1800)),
        };

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

            // Execute using PAS if context is available
            if let Some(ctx) = &mut context {
                match run_command_line(&final_cmd, ctx) {
                    Ok(0) => {}, // Success
                    Ok(code) => {
                        if ignore_failure {
                            log::warn!("{} Command failed with exit code {} but ignored", "⚠️".yellow(), code);
                            continue;
                        }
                        bail!("❌ Task '{}' failed at: '{}' -> Exit code {}", task_name, final_cmd, code);
                    },
                    Err(e) => {
                        if ignore_failure {
                            log::warn!("{} Command failed but ignored: {}", "⚠️".yellow(), e);
                            continue;
                        }
                        bail!("❌ Task '{}' failed at: '{}' -> {}", task_name, final_cmd, e);
                    }
                }
            } else {
                // Fallback to legacy portable/shell command
                let result = if final_cmd.trim_start().starts_with("p:") {
                    run_portable_command(&final_cmd)
                } else {
                    run_shell_command(&final_cmd, &config.env, capture_output, task_name, &shell_cmd, timeout_duration)
                };

                if let Err(e) = result {
                    if ignore_failure {
                        log::warn!("{} Command failed but ignored: {}", "⚠️".yellow(), e);
                        continue;
                    }
                    bail!("❌ Task '{}' failed at: '{}' -> {}", task_name, final_cmd, e);
                }
            }
        }

        // Success: Update cache if sources AND outputs defined (otherwise we never check it anyway)
        if let (Some(srcs), Some(_)) = (&sources, &outputs) {
             save_cache(task_name, srcs)?;
        }
    }
    
    call_stack.pop(task_name);
    Ok(())
}
