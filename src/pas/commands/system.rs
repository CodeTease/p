// System command
use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::{Result, Context};
use std::process::Command;

pub struct SystemCommand;

impl Executable for SystemCommand {
    fn execute(&self, args: &[String], ctx: &mut ShellContext) -> Result<i32> {
        if args.is_empty() {
            return Ok(0);
        }
        let program = &args[0];
        let cmd_args = &args[1..];

        let mut cmd = Command::new(program);
        cmd.current_dir(&ctx.cwd);
        
        // Shadowing: we use the context's environment as the source of truth
        cmd.env_clear();
        cmd.envs(&ctx.env);
        
        cmd.args(cmd_args);

        // Spawn process and wait
        let status = cmd.status().with_context(|| format!("Failed to execute command: {}", program))?;
        
        Ok(status.code().unwrap_or(1))
    }
}
