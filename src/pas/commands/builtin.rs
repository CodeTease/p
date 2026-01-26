// Builtin commands
use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::{Result, Context, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::{Read, Write};

fn resolve_path(ctx: &ShellContext, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        ctx.cwd.join(p)
    }
}

pub struct RmCommand;
impl Executable for RmCommand {
    fn execute(&self, args: &[String], ctx: &mut ShellContext, _stdin: Option<Box<dyn Read + Send>>, _stdout: Option<Box<dyn Write + Send>>) -> Result<i32> {
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

pub struct MkdirCommand;
impl Executable for MkdirCommand {
    fn execute(&self, args: &[String], ctx: &mut ShellContext, _stdin: Option<Box<dyn Read + Send>>, _stdout: Option<Box<dyn Write + Send>>) -> Result<i32> {
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

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

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
