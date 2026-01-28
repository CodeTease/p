// Rm command

use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::{Result, Context, bail};
use std::fs;
use std::io::{Read, Write};
use crate::pas::commands::builtins::common::resolve_path;
use super::check_path_access;

pub struct RmCommand;
impl Executable for RmCommand {
    fn execute(&self, args: &[String], ctx: &mut ShellContext, _stdin: Option<Box<dyn Read + Send>>, _stdout: Option<Box<dyn Write + Send>>, _stderr: Option<Box<dyn Write + Send>>) -> Result<i32> {
        let mut recursive = false;
        let mut force = false;
        let mut paths = Vec::new();

        // Skip command name (args[0])
        for arg in args.iter().skip(1) {
            if arg.starts_with('-') {
                if arg.contains('r') || arg.contains('R') { recursive = true; }
                if arg.contains('f') { force = true; }
            } else {
                paths.push(arg);
            }
        }

        for path_str in paths {
            let p = resolve_path(ctx, path_str);
            check_path_access(&p, ctx)?;
            if !p.exists() {
                if !force {
                    bail!("File not found: {}", path_str);
                }
                continue;
            }

            if p.is_dir() {
                if recursive {
                    fs::remove_dir_all(&p).with_context(|| format!("Failed to remove directory: {}", path_str))?;
                } else {
                    bail!("Cannot remove directory '{}' without -r", path_str);
                }
            } else {
                fs::remove_file(&p).with_context(|| format!("Failed to remove file: {}", path_str))?;
            }
        }
        Ok(0)
    }
}