use anyhow::{Context, Result, bail};
use std::env;
use std::sync::Arc;
use crate::config::load_config;
use crate::runner::{recursive_runner, CallStack};
use crate::pas::context::ShellContext;
use crate::pas::commands::builtin::{RmCommand, MkdirCommand, CpCommand, CdCommand};
use crate::pas::commands::adapter::TaskRunnerAdapter;

pub fn handle_runner_entry(task_name: String, extra_args: Vec<String>, dry_run: bool) -> Result<()> {
    let current_dir = env::current_dir()?;
    let config = load_config(&current_dir)?; 
    
    // Wrap config in Arc for TaskRunnerAdapter
    let config_arc = Arc::new(config);

    let runner_section = config_arc.runner.as_ref().context("No [runner] section defined in config")?;
    if !runner_section.contains_key(&task_name) {
        bail!("Task '{}' not found", task_name);
    }

    let mut call_stack = CallStack::new();

    // Initialize Shell Context
    let mut ctx = ShellContext::new();
    
    // Register builtins
    ctx.register_command("rm", Box::new(RmCommand));
    ctx.register_command("p:rm", Box::new(RmCommand));
    ctx.register_command("mkdir", Box::new(MkdirCommand));
    ctx.register_command("p:mkdir", Box::new(MkdirCommand));
    ctx.register_command("cp", Box::new(CpCommand));
    ctx.register_command("p:cp", Box::new(CpCommand));
    ctx.register_command("cd", Box::new(CdCommand));

    // Register tasks
    for (name, _) in runner_section {
        let adapter = TaskRunnerAdapter {
            task_name: name.clone(),
            config: config_arc.clone(),
        };
        ctx.register_command(name, Box::new(adapter));
    }
    
    // Root task is allowed to print directly to stdout/stderr (capture = false)
    recursive_runner(&task_name, &config_arc, &mut call_stack, &extra_args, false, dry_run, Some(&mut ctx))
}
