use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "p", version, about = "Pavidi: Minimalist Project Runner")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Enter a project's shell environment (Sub-shell session)
    D { path: PathBuf },
    
    /// Run a task defined in .p.toml
    R { 
        task: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
    
    /// Clean artifacts defined in .p.toml
    C,

    /// Jump to a directory (Resolve path for shell hook)
    J { path: PathBuf },

    /// Initialize shell hooks
    I { 
        #[arg(default_value = "zsh")]
        shell: String 
    },

    /// Inspect environment variables
    E,
}
