use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::{Result, Context};
use std::io::{Read, Write};
use std::fs;
use crate::pas::parser::parse_command_line;
use crate::pas::executor::execute_expr;

pub struct SourceCommand;

impl Executable for SourceCommand {
    fn execute(
        &self, 
        args: &[String], 
        ctx: &mut ShellContext, 
        stdin: Option<Box<dyn Read + Send>>, 
        stdout: Option<Box<dyn Write + Send>>,
        stderr: Option<Box<dyn Write + Send>>,
    ) -> Result<i32> {
        if args.len() < 2 {
            // No file specified
            return Ok(1);
        }
        let filepath = &args[1];
        let content = fs::read_to_string(filepath)
            .with_context(|| format!("Failed to read file: {}", filepath))?;

        match parse_command_line(&content, ctx) {
            Ok(expr) => {
                execute_expr(expr, ctx, stdin, stdout, stderr)
            },
            Err(e) => {
                // We should probably log this properly
                if let Some(mut err) = stderr {
                    writeln!(err, "Source error: {}", e).ok();
                } else {
                    eprintln!("Source error: {}", e);
                }
                Ok(1)
            }
        }
    }
}
