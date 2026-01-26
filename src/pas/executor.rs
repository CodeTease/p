use crate::pas::ast::{CommandExpr, RedirectMode};
use crate::pas::context::ShellContext;
use crate::pas::commands::system::SystemCommand;
use crate::pas::commands::Executable;
use anyhow::{Result, Context};
use std::io::{Read, Write};
use std::fs::OpenOptions;
use std::thread;
use os_pipe::pipe;

pub fn execute_expr(
    expr: CommandExpr, 
    ctx: &mut ShellContext, 
    stdin: Option<Box<dyn Read + Send>>, 
    stdout: Option<Box<dyn Write + Send>>
) -> Result<i32> {
    match expr {
        CommandExpr::Simple { program, args } => {
            let mut full_args = vec![program.clone()];
            full_args.extend(args);
            
            let registry = ctx.registry.clone();
            let exit_code = if let Some(cmd) = registry.get(&program) {
                cmd.execute(&full_args, ctx, stdin, stdout)?
            } else {
                let sys_cmd = SystemCommand;
                sys_cmd.execute(&full_args, ctx, stdin, stdout)?
            };
            
            // Update last exit code
            ctx.exit_code = exit_code;
            Ok(exit_code)
        },
        CommandExpr::Pipe { left, right } => {
            let (reader, writer) = pipe().context("Failed to create pipe")?;
            
            // Left runs in separate thread with cloned context (subshell behavior)
            let mut ctx_left = ctx.clone_for_parallel();
            let left_thread = thread::spawn(move || {
                execute_expr(*left, &mut ctx_left, stdin, Some(Box::new(writer)))
            });
            
            // Right runs in current thread
            let right_res = execute_expr(*right, ctx, Some(Box::new(reader)), stdout);
            
            let _ = left_thread.join().unwrap(); // Wait for left to finish
            
            right_res
        },
        CommandExpr::Redirect { cmd, target, mode } => {
            let mut open_opts = OpenOptions::new();
            match mode {
                RedirectMode::Overwrite => { open_opts.write(true).create(true).truncate(true); },
                RedirectMode::Append => { open_opts.write(true).create(true).append(true); },
                RedirectMode::Input => { open_opts.read(true); },
            };
            
            let file = open_opts.open(&target).with_context(|| format!("Failed to open file: {}", target))?;
            
            if mode == RedirectMode::Input {
                execute_expr(*cmd, ctx, Some(Box::new(file)), stdout)
            } else {
                execute_expr(*cmd, ctx, stdin, Some(Box::new(file)))
            }
        },
        CommandExpr::And(left, right) => {
            handle_sequence(*left, Some(*right), ctx, stdin, stdout, true)
        },
        CommandExpr::Or(left, right) => {
            handle_sequence(*left, Some(*right), ctx, stdin, stdout, false)
        }
    }
}

fn handle_sequence(
    left: CommandExpr,
    right: Option<CommandExpr>,
    ctx: &mut ShellContext,
    stdin: Option<Box<dyn Read + Send>>,
    stdout: Option<Box<dyn Write + Send>>,
    is_and: bool
) -> Result<i32> {
    // If stdout is redirected (Some), we need to bridge it so both commands can write to it.
    if let Some(out) = stdout {
        let (mut reader, writer) = pipe().context("Failed to create bridge pipe")?;
        
        let mut out_sink = out;
        let bridge_thread = thread::spawn(move || {
            std::io::copy(&mut reader, &mut out_sink).ok();
        });
        
        // We clone writer for Left.
        // Right gets ownership of 'writer' later.
        // BUT wait, if we give 'writer' to Right, and Left is done, Left drops its writer.
        // Pipe stays open until Right is done.
        
        let w1 = writer.try_clone().context("Failed to clone pipe writer")?;
        let w2 = writer; 
        
        // Run Left
        let left_res = execute_expr(left, ctx, stdin, Some(Box::new(w1)))?;
        
        let proceed = if is_and { left_res == 0 } else { left_res != 0 };
        
        let final_res = if proceed {
             if let Some(r) = right {
                 // Right gets w2
                 execute_expr(r, ctx, None, Some(Box::new(w2)))?
             } else {
                 left_res
             }
        } else {
            // Drop w2 to close pipe
            drop(w2);
            left_res
        };
        
        bridge_thread.join().unwrap();
        Ok(final_res)
    } else {
        // Inherit case
        let left_res = execute_expr(left, ctx, stdin, None)?;
        
        let proceed = if is_and { left_res == 0 } else { left_res != 0 };
        
        if proceed {
            if let Some(r) = right {
                 execute_expr(r, ctx, None, None)
            } else {
                Ok(left_res)
            }
        } else {
            Ok(left_res)
        }
    }
}
