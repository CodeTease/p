// Mkdir command

use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::{Result, Context};
use std::fs;
use std::io::{Read, Write};
use crate::pas::commands::builtins::common::resolve_path;

pub struct MkdirCommand;
impl Executable for MkdirCommand {
    fn execute(&self, args: &[String], ctx: &mut ShellContext, _stdin: Option<Box<dyn Read + Send>>, _stdout: Option<Box<dyn Write + Send>>, _stderr: Option<Box<dyn Write + Send>>) -> Result<i32> {
        let mut parents = false;
        let mut paths = Vec::new();

        // Skip command name
        for arg in args.iter().skip(1) {
            if arg == "-p" {
                parents = true;
            } else if arg.starts_with('-') {
                // Ignore other flags
            } else {
                paths.push(arg);
            }
        }

        for path_str in paths {
            let p = resolve_path(ctx, path_str);
            if parents {
                fs::create_dir_all(&p).with_context(|| format!("Failed to create directory (with parents): {}", path_str))?;
            } else {
                fs::create_dir(&p).with_context(|| format!("Failed to create directory: {}", path_str))?;
            }
        }
        Ok(0)
    }
}