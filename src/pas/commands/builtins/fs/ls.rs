// Ls command

use std::fs;
use std::io::{Read, Write};
use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::{Result, Context};
use crate::pas::commands::builtins::common::resolve_path;

pub struct LsCommand;
impl Executable for LsCommand {
    fn execute(
        &self,
        args: &[String],
        ctx: &mut ShellContext,
        _stdin: Option<Box<dyn Read + Send>>,
        stdout: Option<Box<dyn Write + Send>>,
        _stderr: Option<Box<dyn Write + Send>>,
    ) -> Result<i32> {
        let path_str = if args.len() > 1 {
            &args[1]
        } else {
            "."
        };

        let path = resolve_path(ctx, path_str);
        let entries = fs::read_dir(&path)
            .with_context(|| format!("Failed to read directory: {}", path_str))?;

        let mut output = String::new();
        for entry in entries {
            let entry = entry?;
            let file_name = entry.file_name();
            output.push_str(&format!("{}\n", file_name.to_string_lossy()));
        }

        if let Some(mut out) = stdout {
            write!(out, "{}", output)?;
        } else {
            print!("{}", output);
        }

        Ok(0)
    }
}