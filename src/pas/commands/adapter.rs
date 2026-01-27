// TaskRunnerAdapter
use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use crate::config::PavidiConfig;
use crate::runner::{recursive_runner, CallStack};
use anyhow::Result;
use std::sync::Arc;
use std::io::{Read, Write};

pub struct TaskRunnerAdapter {
    pub task_name: String,
    pub config: Arc<PavidiConfig>,
}

impl Executable for TaskRunnerAdapter {
    fn execute(&self, args: &[String], ctx: &mut ShellContext, _stdin: Option<Box<dyn Read + Send>>, _stdout: Option<Box<dyn Write + Send>>, _stderr: Option<Box<dyn Write + Send>>) -> Result<i32> {
        let extra_args = args.iter().skip(1).cloned().collect::<Vec<_>>();
        let mut call_stack = CallStack::new();

        // Calls recursive_runner with the context.
        // We assume recursive_runner has been updated to accept &mut ShellContext.
        recursive_runner(
            &self.task_name, 
            &self.config, 
            &mut call_stack, 
            &extra_args, 
            false, 
            false,
            Some(ctx)
        )?;
        
        Ok(0)
    }
}
