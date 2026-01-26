// Cd command

use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::{Result, bail};
use std::io::{Read, Write};
use crate::pas::commands::builtins::common::resolve_path;

pub struct CdCommand;
impl Executable for CdCommand {
    fn execute(&self, args: &[String], ctx: &mut ShellContext, _stdin: Option<Box<dyn Read + Send>>, _stdout: Option<Box<dyn Write + Send>>) -> Result<i32> {
        // args[0] is "cd". args[1] is path.
        let path_str = if args.len() < 2 {
            // Default to HOME or root?
             ctx.env.get("HOME").map(|s| s.as_str()).unwrap_or("/")
        } else {
            &args[1]
        };

        let new_path = resolve_path(ctx, path_str);
        if new_path.exists() && new_path.is_dir() {
            // Canonicalize to remove .. and .
            if let Ok(canon) = new_path.canonicalize() {
                ctx.cwd = canon;
            } else {
                ctx.cwd = new_path; // Fallback
            }
            Ok(0)
        } else {
            bail!("cd: no such file or directory: {}", path_str);
        }
    }
}