use anyhow::{Context, Result, bail};
use std::env;
use std::sync::Arc;
use crate::config::load_config;
use crate::runner::{recursive_runner, CallStack};

pub fn handle_runner_entry(task_name: String, extra_args: Vec<String>, dry_run: bool, trace: bool) -> Result<()> {
    let current_dir = env::current_dir()?;
    let config = load_config(&current_dir)?; 
    
    // Wrap config in Arc for TaskRunnerAdapter
    let config_arc = Arc::new(config);

    let runner_section = config_arc.runner.as_ref().context("No [runner] section defined in config")?;
    if !runner_section.contains_key(&task_name) {
        bail!("Task '{}' not found", task_name);
    }

    let mut call_stack = CallStack::new();

    // Root task is allowed to print directly to stdout/stderr (capture = false)
    recursive_runner(&task_name, &config_arc, &mut call_stack, &extra_args, false, dry_run, trace, 0)
}
