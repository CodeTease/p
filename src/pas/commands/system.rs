// System command
use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::{Result, Context};
use std::process::{Command, Stdio};
use std::io::{Read, Write};
use std::thread;

pub struct SystemCommand;

impl Executable for SystemCommand {
    fn execute(
        &self, 
        args: &[String], 
        ctx: &mut ShellContext, 
        stdin: Option<Box<dyn Read + Send>>, 
        stdout: Option<Box<dyn Write + Send>>,
        stderr: Option<Box<dyn Write + Send>>
    ) -> Result<i32> {
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

        // Handle Stdin
        if stdin.is_some() {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::inherit());
        }

        // Handle Stdout
        if stdout.is_some() {
            cmd.stdout(Stdio::piped());
        } else {
            cmd.stdout(Stdio::inherit());
        }

        // Handle Stderr
        if stderr.is_some() {
            cmd.stderr(Stdio::piped());
        } else {
            cmd.stderr(Stdio::inherit());
        }

        let mut child = cmd.spawn().with_context(|| format!("Failed to execute command: {}", program))?;

        // Spawn thread for Stdin
        if let Some(mut source) = stdin {
             if let Some(mut child_in) = child.stdin.take() {
                 thread::spawn(move || {
                     std::io::copy(&mut source, &mut child_in).ok();
                 });
             }
        }

        // Spawn thread for Stdout
        let stdout_thread = if let Some(mut dest) = stdout {
            if let Some(mut child_out) = child.stdout.take() {
                 Some(thread::spawn(move || {
                     std::io::copy(&mut child_out, &mut dest).ok();
                 }))
            } else {
                None
            }
        } else {
            None
        };

        // Spawn thread for Stderr
        let stderr_thread = if let Some(mut dest) = stderr {
            if let Some(mut child_err) = child.stderr.take() {
                 Some(thread::spawn(move || {
                     std::io::copy(&mut child_err, &mut dest).ok();
                 }))
            } else {
                None
            }
        } else {
            None
        };

        let status = child.wait()?;
        
        // Wait for stdout thread to finish copying (ensure all output is flushed)
        if let Some(handle) = stdout_thread {
            handle.join().ok(); 
        }

        if let Some(handle) = stderr_thread {
            handle.join().ok();
        }

        Ok(status.code().unwrap_or(1))
    }
}
