use clap::Parser;

#[derive(Parser)]
#[command(name = "p", version, about = "Pavidi: Minimalist Project Runner")]
pub struct Cli {
    /// List all available tasks
    #[arg(short, long)]
    pub list: bool,

    /// Inspect environment variables
    #[arg(short, long)]
    pub env: bool,

    /// Show project/module metadata
    #[arg(short = 'i', long = "info")]
    pub info: bool,

    /// Run in dry-run mode (print commands without executing)
    #[arg(short = 'd', long = "dry-run")]
    pub dry_run: bool,

    /// The task to run (defaults to "default")
    #[arg(name = "TASK")]
    pub task: Option<String>,

    /// Arguments to pass to the task
    #[arg(last = true)]
    pub args: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
