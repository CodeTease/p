use anyhow::{Context, Result, bail};
use colored::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::env;
use crate::runner::task::RunnerTask;

#[derive(Debug, Deserialize)]
pub struct PavidiConfig {
    pub project: Option<ProjectConfig>,
    #[serde(default)] 
    pub env: HashMap<String, String>,
    pub runner: Option<HashMap<String, RunnerTask>>,
    pub clean: Option<CleanConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: Option<String>,
    pub shell: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CleanConfig {
    pub targets: Vec<String>,
}

pub fn load_config(dir: &Path) -> Result<PavidiConfig> {
    let config_path = dir.join("p.toml");
    if !config_path.exists() {
        bail!("❌ Critical: 'p.toml' not found in {:?}.", dir);
    }
    let content = fs::read_to_string(&config_path).context("Failed to read p.toml")?;
    
    // 1. Parse p.toml (Base Layer)
    let mut config: PavidiConfig = toml::from_str(&content).context("Failed to parse p.toml")?;

    // 2. Load .env using dotenvy (Override Layer)
    // Determines filename: .env or .env.prod based on P_ENV
    let env_filename = env::var("P_ENV")
        .map(|v| format!(".env.{}", v))
        .unwrap_or_else(|_| ".env".to_string());
    
    let env_path = dir.join(&env_filename);

    if env_path.exists() {
        eprintln!("{} Loading environment from: {}", "🌿".green(), env_filename.bold());
        
        // We use from_path_iter to get the vars as a Map, NOT setting them globally yet.
        // This keeps the separation clean until execution.
        for item in dotenvy::from_path_iter(&env_path)? {
            let (key, val) = item?;
            // .env overrides p.toml
            config.env.insert(key, val);
        }
    }

    Ok(config)
}
