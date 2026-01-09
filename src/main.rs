use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use colored::*;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::env;
use std::time::SystemTime;
use rayon::prelude::*;

// --- CLI Structure ---

#[derive(Parser)]
#[command(name = "p", version, about = "Pavidi: Minimalist Project Runner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Jump to a project directory and enter its shell environment
    D { path: PathBuf },
    
    /// Run a task defined in .p.toml
    R { 
        task: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
    
    /// Clean artifacts defined in .p.toml
    C,
}

// --- Config Structure ---

#[derive(Debug, Deserialize)]
struct PavidiConfig {
    project: Option<ProjectConfig>,
    #[serde(default)] 
    env: HashMap<String, String>,
    runner: Option<HashMap<String, RunnerTask>>,
    clean: Option<CleanConfig>,
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    name: Option<String>,
    shell: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CleanConfig {
    targets: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum RunnerTask {
    /// Simple string command
    Single(String),
    /// List of sequential commands
    List(Vec<String>),
    /// Full configuration with dependencies and caching
    Full {
        #[serde(default)]
        cmds: Vec<String>,
        #[serde(default)]
        deps: Vec<String>,
        #[serde(default)]
        parallel: bool,
        // Conditional Execution
        sources: Option<Vec<String>>,
        outputs: Option<Vec<String>>,
    },
}

// --- Main Execution ---

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::D { path } => handle_dir_jump(path),
        Commands::R { task, args } => handle_runner_entry(task, args),
        Commands::C => handle_clean(),
    }
}

// --- Handlers ---

fn load_config(dir: &Path) -> Result<PavidiConfig> {
    let config_path = dir.join(".p.toml");
    if !config_path.exists() {
        bail!("❌ Critical: '.p.toml' not found in {:?}.", dir);
    }
    let content = fs::read_to_string(&config_path).context("Failed to read .p.toml")?;
    toml::from_str(&content).context("Failed to parse .p.toml")
}

fn handle_dir_jump(target_path: PathBuf) -> Result<()> {
    if !target_path.exists() || !target_path.is_dir() {
        bail!("Target directory does not exist: {:?}", target_path);
    }

    let config = load_config(&target_path)?;
    let abs_path = fs::canonicalize(&target_path)?;

    // Detect shell preference or fallback to system default
    let shell_cmd = config.project
        .and_then(|p| p.shell)
        .or_else(|| env::var("SHELL").ok())
        .unwrap_or_else(|| if cfg!(windows) { "cmd".to_string() } else { "sh".to_string() });

    eprintln!("{} Entering environment at: {}", "⤵️".cyan(), abs_path.display());
    
    let mut command = Command::new(&shell_cmd);
    command.current_dir(&abs_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .envs(&config.env);

    let status = command.status()
        .context(format!("Failed to spawn shell: {}", shell_cmd))?;

    if !status.success() {
        eprintln!("{} Shell exited with non-zero code.", "⚠️".yellow());
    }

    // Output for external tools (like shell aliases) to capture the path
    if let Ok(output_file) = env::var("PAVIDI_OUTPUT") {
        fs::write(output_file, abs_path.to_string_lossy().as_bytes())
            .context("Failed to write jump path")?;
    } else {
        println!("{}", abs_path.to_string_lossy());
    }

    Ok(())
}

fn handle_runner_entry(task_name: String, extra_args: Vec<String>) -> Result<()> {
    let current_dir = env::current_dir()?;
    let config = load_config(&current_dir)?;
    
    let runner_section = config.runner.as_ref().context("No [runner] section defined in config")?;
    if !runner_section.contains_key(&task_name) {
        bail!("Task '{}' not found", task_name);
    }

    let mut call_stack = HashSet::new();
    
    // Root task is allowed to print directly to stdout/stderr (capture = false)
    recursive_runner(&task_name, &config, &mut call_stack, &extra_args, false)
}

fn recursive_runner(
    task_name: &str, 
    config: &PavidiConfig, 
    call_stack: &mut HashSet<String>,
    extra_args: &[String],
    capture_output: bool // true = buffer output (for parallel), false = inherit
) -> Result<()> {
    if call_stack.contains(task_name) {
        bail!("🔄 Circular dependency detected: {}", task_name);
    }
    call_stack.insert(task_name.to_string());

    let runner_section = config.runner.as_ref().unwrap();
    let task = runner_section.get(task_name).expect("Task check passed before");

    // Destructure task config
    let (mut cmds, deps, parallel_deps, sources, outputs) = match task {
        RunnerTask::Single(cmd) => (vec![cmd.clone()], vec![], false, None, None),
        RunnerTask::List(cmds) => (cmds.clone(), vec![], false, None, None),
        RunnerTask::Full { cmds, deps, parallel, sources, outputs } => 
            (cmds.clone(), deps.clone(), *parallel, sources.clone(), outputs.clone()),
    };

    // 1. Run Dependencies
    // Note: Dependencies run BEFORE checking "up-to-date" for the current task,
    // because dependencies might update the source files of the current task.
    if !deps.is_empty() {
        if parallel_deps {
            if !capture_output {
                println!("{} Running dependencies in parallel: {:?}...", "🚀".cyan(), deps);
            }
            
            // Rayon parallel iterator
            let errors: Vec<String> = deps
                .par_iter()
                .map(|dep_name| {
                    let mut local_stack = call_stack.clone(); 
                    // Parallel deps MUST capture output to prevent mixed logs
                    recursive_runner(dep_name, config, &mut local_stack, &[], true)
                        .map_err(|e| format!("Dep '{}' failed: {}", dep_name, e))
                })
                .filter_map(|res| res.err())
                .collect();

            if !errors.is_empty() {
                for e in &errors { eprintln!("{} {}", "❌".red(), e); }
                bail!("Dependency execution failed.");
            }
        } else {
            if !capture_output {
                println!("{} Running dependencies sequentially...", "🔗".blue());
            }
            for dep in deps {
                // Sequential deps inherit capture mode
                recursive_runner(&dep, config, call_stack, &[], capture_output)?;
            }
        }
    }

    // 2. Check Conditional Execution (Cache Check)
    if let (Some(srcs), Some(outs)) = (sources, outputs) {
        if is_up_to_date(&srcs, &outs)? {
            if !capture_output {
                println!("{} Task '{}' is up-to-date. Skipping.", "✨".green(), task_name.bold());
            }
            call_stack.remove(task_name);
            return Ok(());
        }
    }

    // 3. Execute Main Commands
    if !cmds.is_empty() {
        if !capture_output {
            println!("{} Running task: {}", "⚡".yellow(), task_name.bold());
        }

        for cmd in &mut cmds {
            // Apply Argument Expansion ($1, $2...)
            let final_cmd = expand_command(cmd, extra_args);

            if !capture_output {
                println!("{} Executing: {}", "::".blue(), final_cmd);
            }

            if let Err(e) = run_shell_command(&final_cmd, &config.env, capture_output, task_name) {
                bail!("❌ Task '{}' failed at: '{}' -> {}", task_name, final_cmd, e);
            }
        }
    }
    
    call_stack.remove(task_name);
    Ok(())
}

/// Check if outputs are newer than sources based on modification time (mtime).
fn is_up_to_date(sources: &[String], outputs: &[String]) -> Result<bool> {
    let mut latest_src = SystemTime::UNIX_EPOCH;
    let mut oldest_out = SystemTime::now(); // Start with "now" and find something older
    
    let mut has_src = false;
    let mut has_out = false;

    // Find latest source mtime
    for pattern in sources {
        for entry in glob::glob(pattern)? {
            let path = entry?;
            let metadata = fs::metadata(&path)?;
            let mtime = metadata.modified()?;
            if mtime > latest_src {
                latest_src = mtime;
            }
            has_src = true;
        }
    }

    // Find oldest output mtime
    for pattern in outputs {
        for entry in glob::glob(pattern)? {
            let path = entry?;
            let metadata = fs::metadata(&path)?;
            let mtime = metadata.modified()?;
            if mtime < oldest_out {
                oldest_out = mtime;
            }
            has_out = true;
        }
    }

    // If source or output files are missing, we cannot skip.
    if !has_src || !has_out {
        return Ok(false);
    }

    // If latest source is OLDER than oldest output -> Up to date
    Ok(latest_src < oldest_out)
}

/// Replaces $1, $2... with corresponding args.
/// Fallback: If no placeholders found, append args to the end.
fn expand_command(cmd_template: &str, args: &[String]) -> String {
    if args.is_empty() {
        return cmd_template.to_string();
    }

    let mut expanded = cmd_template.to_string();
    let mut replaced = false;

    for (i, arg) in args.iter().enumerate() {
        let placeholder = format!("${}", i + 1);
        if expanded.contains(&placeholder) {
            expanded = expanded.replace(&placeholder, arg);
            replaced = true;
        }
    }

    // Backward Compatibility: Append if no placeholders used
    if !replaced {
        expanded.push_str(" ");
        expanded.push_str(&args.join(" "));
    }
    
    expanded
}

fn run_shell_command(
    cmd_str: &str, 
    env_vars: &HashMap<String, String>, 
    capture: bool,
    task_label: &str
) -> Result<()> {
    #[cfg(target_os = "windows")]
    let (shell, flag) = ("cmd", "/C");
    #[cfg(not(target_os = "windows"))]
    let (shell, flag) = ("sh", "-c");

    let mut command = Command::new(shell);
    command.arg(flag)
           .arg(cmd_str)
           .envs(env_vars)
           .stdin(Stdio::inherit()); // Inherit stdin allows user interaction (unless deeply parallel)

    if capture {
        // Parallel/Quiet Mode: Capture stdout/stderr
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.output().context("Failed to spawn shell process (captured)")?;

        // Atomic Print: Print everything at once when done
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.trim().is_empty() {
            println!("[{}] {}", task_label.cyan(), stdout.trim());
        }
        if !stderr.trim().is_empty() {
            eprintln!("[{}] {}", task_label.red(), stderr.trim());
        }

        if !output.status.success() {
            bail!("Exit code: {:?}", output.status.code());
        }
    } else {
        // Interactive Mode: Direct output stream
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        let status = command.status().context("Failed to spawn shell process")?;
        if !status.success() {
            bail!("Exit code: {:?}", status.code());
        }
    }

    Ok(())
}

fn handle_clean() -> Result<()> {
    let current_dir = env::current_dir()?;
    let config = load_config(&current_dir)?;
    let clean_section = config.clean.context("No [clean] section defined in config")?;

    println!("{} Cleaning targets...", "🧹".red());
    for pattern in clean_section.targets {
        let full_pattern = format!("{}/{}", current_dir.to_string_lossy(), pattern);
        for entry in glob::glob(&full_pattern)? {
            if let Ok(path) = entry {
                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                    println!("   Deleted dir: {:?}", path.file_name().unwrap());
                } else {
                    fs::remove_file(&path)?;
                    println!("   Deleted file: {:?}", path.file_name().unwrap());
                }
            }
        }
    }
    Ok(())
}