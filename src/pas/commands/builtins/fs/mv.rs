// Mv command

use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::{Result, Context, bail};
use std::fs;
use std::io::Write;
use std::path::Path;

pub struct MvCommand;
impl Executable for MvCommand {
    fn execute(
        &self,
        args: &[String],
        _ctx: &mut ShellContext,
        _stdin: Option<Box<dyn std::io::Read + Send>>,
        _stdout: Option<Box<dyn std::io::Write + Send>>,
    ) -> Result<i32> {
        if args.len() < 3 {
            writeln!(std::io::stderr(), "Usage: mv <source1> <source2> ... <destination>")?;
            return Ok(1);
        }

        let dest = args.last().unwrap();
        let dest_path = Path::new(dest);
        let dest_is_dir = dest_path.is_dir();

        let sources = &args[1..args.len() - 1];
        if sources.len() > 1 && !dest_is_dir {
            bail!("Target '{}' is not a directory", dest);
        }

        for src in sources {
            let src_path = Path::new(src);
            if !src_path.exists() {
                bail!("Source not found: {}", src);
            }

            let target = if dest_is_dir {
                dest_path.join(src_path.file_name().ok_or_else(|| anyhow::anyhow!("Invalid source filename"))?)
            } else {
                dest_path.to_path_buf()
            };

            fs::rename(src_path, &target).with_context(|| format!("Failed to move {} to {}", src, target.display()))?;
        }

        Ok(0)
    }
}