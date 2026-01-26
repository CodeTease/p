use anyhow::{Result, Context, bail};
use std::fs;
use std::path::Path;

pub fn run_portable_command(cmd_str: &str) -> Result<()> {
    let args = shell_words::split(cmd_str).context("Failed to parse portable command arguments")?;
    if args.is_empty() {
        return Ok(());
    }

    let command = &args[0];
    match command.as_str() {
        "p:rm" => handle_rm(&args[1..]),
        "p:mkdir" => handle_mkdir(&args[1..]),
        "p:cp" => handle_cp(&args[1..]),
        _ => bail!("Unknown portable command: {}", command),
    }
}

fn handle_rm(args: &[String]) -> Result<()> {
    let mut recursive = false;
    let mut force = false;
    let mut paths = Vec::new();

    for arg in args {
        if arg.starts_with('-') {
            if arg.contains('r') || arg.contains('R') { recursive = true; }
            if arg.contains('f') { force = true; }
        } else {
            paths.push(arg);
        }
    }

    for path in paths {
        let p = Path::new(path);
        if !p.exists() {
            if !force {
                bail!("File not found: {}", path);
            }
            continue;
        }

        if p.is_dir() {
            if recursive {
                fs::remove_dir_all(p).with_context(|| format!("Failed to remove directory: {}", path))?;
            } else {
                bail!("Cannot remove directory '{}' without -r", path);
            }
        } else {
            fs::remove_file(p).with_context(|| format!("Failed to remove file: {}", path))?;
        }
    }
    Ok(())
}

fn handle_mkdir(args: &[String]) -> Result<()> {
    let mut parents = false;
    let mut paths = Vec::new();

    for arg in args {
        if arg == "-p" {
            parents = true;
        } else if arg.starts_with('-') {
            // Ignore other flags
        } else {
            paths.push(arg);
        }
    }

    for path in paths {
        if parents {
            fs::create_dir_all(path).with_context(|| format!("Failed to create directory (with parents): {}", path))?;
        } else {
            fs::create_dir(path).with_context(|| format!("Failed to create directory: {}", path))?;
        }
    }
    Ok(())
}

fn handle_cp(args: &[String]) -> Result<()> {
    let mut recursive = false;
    let mut paths = Vec::new();

    for arg in args {
        if arg == "-r" || arg == "-R" || arg == "--recursive" {
            recursive = true;
        } else {
            paths.push(arg);
        }
    }

    if paths.len() < 2 {
        bail!("cp requires at least source and destination");
    }

    let dest = paths.pop().unwrap(); // Last one is dest
    let sources = paths;

    let dest_path = Path::new(&dest);
    let dest_is_dir = dest_path.is_dir();

    if sources.len() > 1 && !dest_is_dir {
        bail!("Target '{}' is not a directory", dest);
    }

    for src in sources {
        let src_path = Path::new(&src);
        if !src_path.exists() {
            bail!("Source not found: {}", src);
        }

        let target = if dest_is_dir {
            dest_path.join(src_path.file_name().ok_or_else(|| anyhow::anyhow!("Invalid source filename"))?)
        } else {
            dest_path.to_path_buf()
        };

        if src_path.is_dir() {
            if recursive {
                copy_dir_recursive(src_path, &target)?;
            } else {
                bail!("Omitting directory '{}' (use -r to copy)", src);
            }
        } else {
            fs::copy(src_path, &target).with_context(|| format!("Failed to copy {} to {}", src, target.display()))?;
        }
    }

    Ok(())
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
