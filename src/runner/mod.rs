pub mod task;
pub mod cache;

use anyhow::{Result, bail};
use colored::*;
use std::collections::HashSet;
use rayon::prelude::*;
use crate::config::PavidiConfig;
use crate::utils::{detect_shell, expand_command, run_shell_command};
use self::task::RunnerTask;
use self::cache::is_up_to_date;
use log::{info, error, debug};

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
    let (mut cmds, deps, parallel_deps, sources, outputs) = match task {
        RunnerTask::Single(cmd) => (vec![cmd.clone()], vec![], false, None, None),
        RunnerTask::List(cmds) => (cmds.clone(), vec![], false, None, None),
        RunnerTask::Full { cmds, deps, parallel, sources, outputs, .. } => 
            (cmds.clone(), deps.clone(), *parallel, sources.clone(), outputs.clone()),
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

    // 2. Check Conditional Execution (Cache Check)
    if let (Some(srcs), Some(outs)) = (sources, outputs) {
        // We only check cache if NOT in dry-run, OR we can check it but should we skip?
        // Usually dry-run should show what WOULD happen. If it's cached, it wouldn't run.
        // So checking cache is correct even in dry-run.
        if is_up_to_date(&srcs, &outs)? {
            if !capture_output {
                info!("{} Task '{}' is up-to-date. Skipping.", "✨".green(), task_name.bold());
            }
            call_stack.pop(task_name);
            return Ok(());
        }
    }

    // 3. Execute Main Commands
    if !cmds.is_empty() {
        if !capture_output {
            info!("{} Running task: {}", "⚡".yellow(), task_name.bold());
        }

        // Phase 3: Optimize Core Logic - detect shell
        let shell_cmd = detect_shell(config.project.as_ref().and_then(|p| p.shell.as_ref()));

        for cmd in &mut cmds {
            // Apply Argument Expansion ($1, $2...)
            let final_cmd = expand_command(cmd, extra_args);

            if dry_run {
                println!("{} [DRY-RUN] Executing: {}", "::".yellow(), final_cmd);
                continue;
            }

            if !capture_output {
                info!("{} Executing: {}", "::".blue(), final_cmd);
            }

            if let Err(e) = run_shell_command(&final_cmd, &config.env, capture_output, task_name, &shell_cmd) {
                bail!("❌ Task '{}' failed at: '{}' -> {}", task_name, final_cmd, e);
            }
        }
    }
    
    call_stack.pop(task_name);
    Ok(())
}
