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
use crate::pas::context::ShellContext;
use crate::pas::run_command_line;
use crate::logger::write_log;
use self::task::RunnerTask;
use self::cache::{is_up_to_date, save_cache};
use self::portable::run_portable_command;
use log::{info, error};
use std::time::Instant;
use std::io::Write;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct PasTeeWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
    inner: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
}

impl PasTeeWriter {
    fn new(buffer: Arc<Mutex<Vec<u8>>>, inner: Option<Box<dyn Write + Send>>) -> Self {
        Self {
            buffer,
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

impl Write for PasTeeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.lock().unwrap().extend_from_slice(buf);
        if let Some(inner) = self.inner.lock().unwrap().as_mut() {
            inner.write(buf)
        } else {
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(inner) = self.inner.lock().unwrap().as_mut() {
            inner.flush()
        } else {
            Ok(())
        }
    }
}

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

        // Optimize Core Logic - detect shell
        let shell_pref = config.project.as_ref().and_then(|p| p.shell.as_ref())
            .or(config.module.as_ref().and_then(|m| m.shell.as_ref()));
        let shell_cmd = detect_shell(shell_pref);
        
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

            let start_time = Instant::now();
            let mut captured_output = String::new();
            let mut exit_code = 0;

            // Execute using PAS if context is available
            if let Some(ctx) = &mut context {
                let res = if capture_mode != CaptureMode::Inherit {
                    let buf = Arc::new(Mutex::new(Vec::new()));
                    // Create writers. If Tee, print to Stdout/Stderr. If Buffer, None.
                    let (out_writer, err_writer) = if capture_mode == CaptureMode::Tee {
                        (Some(Box::new(PasTeeWriter::new(buf.clone(), Some(Box::new(std::io::stdout())))) as Box<dyn Write + Send>),
                         Some(Box::new(PasTeeWriter::new(buf.clone(), Some(Box::new(std::io::stderr())))) as Box<dyn Write + Send>))
                    } else {
                        // Buffer mode: Capture only.
                        (Some(Box::new(PasTeeWriter::new(buf.clone(), None)) as Box<dyn Write + Send>),
                         Some(Box::new(PasTeeWriter::new(buf.clone(), None)) as Box<dyn Write + Send>))
                    };

                    let r = run_command_line(&final_cmd, ctx, out_writer, err_writer);
                    
                    let output_bytes = buf.lock().unwrap();
                    captured_output = String::from_utf8_lossy(&output_bytes).to_string();
                    
                    if capture_mode == CaptureMode::Buffer && !captured_output.trim().is_empty() {
                        info!("[{}] output:\n{}", task_name.cyan(), captured_output.trim());
                    }
                    r
                } else {
                    run_command_line(&final_cmd, ctx, None, None)
                };

                match res {
                    Ok(0) => exit_code = 0,
                    Ok(code) => {
                        exit_code = code;
                        if log_enabled {
                            let _ = write_log(task_name, &final_cmd, &captured_output, config, start_time.elapsed(), exit_code, &config.env);
                        }
                        if ignore_failure {
                            log::warn!("{} Command failed with exit code {} but ignored", "⚠️".yellow(), code);
                        } else {
                            bail!("❌ Task '{}' failed at: '{}' -> Exit code {}", task_name, final_cmd, code);
                        }
                    },
                    Err(e) => {
                        // PAS execution error (not command exit code)
                        if log_enabled {
                            let _ = write_log(task_name, &final_cmd, &format!("Internal Error: {}\nPartial Output:\n{}", e, captured_output), config, start_time.elapsed(), 1, &config.env);
                        }
                        if ignore_failure {
                            log::warn!("{} Command failed but ignored: {}", "⚠️".yellow(), e);
                            continue;
                        }
                        bail!("❌ Task '{}' failed at: '{}' -> {}", task_name, final_cmd, e);
                    }
                }
            } else {
                // Fallback to legacy portable/shell command
                if final_cmd.trim_start().starts_with("p:") {
                     if let Err(e) = run_portable_command(&final_cmd) {
                        if ignore_failure {
                            log::warn!("{} Command failed but ignored: {}", "⚠️".yellow(), e);
                            continue;
                        }
                        bail!("❌ Task '{}' failed at: '{}' -> {}", task_name, final_cmd, e);
                     }
                } else {
                    let result = run_shell_command(&final_cmd, &config.env, capture_mode, task_name, &shell_cmd, timeout_duration);
                    
                    match result {
                        Ok((code, output)) => {
                            captured_output = output;
                            exit_code = code;
                            if code != 0 {
                                if log_enabled {
                                    let _ = write_log(task_name, &final_cmd, &captured_output, config, start_time.elapsed(), code, &config.env);
                                }
                                if ignore_failure {
                                    log::warn!("{} Command failed but ignored (code {})", "⚠️".yellow(), code);
                                } else {
                                    bail!("❌ Task '{}' failed at: '{}' -> Exit code {}", task_name, final_cmd, code);
                                }
                            }
                        },
                        Err(e) => {
                             // Execution error (timeout, etc)
                            if log_enabled {
                                let _ = write_log(task_name, &final_cmd, &format!("Execution Error: {}", e), config, start_time.elapsed(), 1, &config.env);
                            }
                            if ignore_failure {
                                log::warn!("{} Command failed but ignored: {}", "⚠️".yellow(), e);
                                continue;
                            }
                            bail!("❌ Task '{}' failed at: '{}' -> {}", task_name, final_cmd, e);
                        }
                    }
                }
            }

            if log_enabled {
                 if let Ok(Some(path)) = write_log(task_name, &final_cmd, &captured_output, config, start_time.elapsed(), exit_code, &config.env) {
                     info!("{} Log saved: {}", "📝".dimmed(), path.display());
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
