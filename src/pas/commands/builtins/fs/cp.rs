// Cp command

use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::{Result, Context, bail};
use std::fs;
use std::io::{Read, Write};
use crate::pas::commands::builtins::common::{resolve_path, copy_dir_recursive};

pub struct CpCommand;
impl Executable for CpCommand {
    fn execute(&self, args: &[String], ctx: &mut ShellContext, _stdin: Option<Box<dyn Read + Send>>, _stdout: Option<Box<dyn Write + Send>>) -> Result<i32> {
        let mut recursive = false;
        let mut paths = Vec::new();

        // Skip command name
        for arg in args.iter().skip(1) {
            if arg == "-r" || arg == "-R" || arg == "--recursive" {
                recursive = true;
            } else {
                paths.push(arg);
            }
        }

        if paths.len() < 2 {
            bail!("cp requires at least source and destination");
        }

        let dest_str = paths.pop().unwrap();
        let sources = paths;

        let dest_path = resolve_path(ctx, &dest_str);
        let dest_is_dir = dest_path.is_dir();

        if sources.len() > 1 && !dest_is_dir {
             bail!("Target '{}' is not a directory", dest_str);
        }

        for src_str in sources {
            let src_path = resolve_path(ctx, src_str);
            if !src_path.exists() {
                bail!("Source not found: {}", src_str);
            }

            let target = if dest_is_dir {
                dest_path.join(src_path.file_name().ok_or_else(|| anyhow::anyhow!("Invalid source filename"))?)
            } else {
                dest_path.clone()
            };

            if src_path.is_dir() {
                if recursive {
                    copy_dir_recursive(&src_path, &target)?;
                } else {
                    bail!("Omitting directory '{}' (use -r to copy)", src_str);
                }
            } else {
                fs::copy(&src_path, &target).with_context(|| format!("Failed to copy {} to {}", src_str, target.display()))?;
            }
        }

        Ok(0)
    }
}