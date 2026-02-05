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
    pub module: Option<ModuleConfig>,
    pub capability: Option<CapabilityConfig>,
    #[serde(default)] 
    pub env: HashMap<String, String>,
    pub runner: Option<HashMap<String, RunnerTask>>,
    pub clean: Option<CleanConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Metadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub authors: Option<Vec<String>>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum LogStrategy {
    Always,
    ErrorOnly,
    None,
}

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    #[serde(flatten)]
    pub metadata: Metadata,
    pub shell: Option<String>,
    pub log_strategy: Option<LogStrategy>,
    pub log_plain: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ModuleConfig {
    #[serde(flatten)]
    pub metadata: Metadata,
    pub shell: Option<String>,
    pub log_strategy: Option<LogStrategy>,
    pub log_plain: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CapabilityConfig {
    pub allow_paths: Option<Vec<String>>,
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

    // Resolve relative paths in capabilities
    if let Some(caps) = &mut config.capability {
        if let Some(paths) = &mut caps.allow_paths {
            let resolved: Vec<String> = paths.iter().map(|p| {
                let path = Path::new(p);
                if path.is_absolute() {
                    p.clone()
                } else {
                    dir.join(p).to_string_lossy().into_owned()
                }
            }).collect();
            *paths = resolved;
        }
    }

    // Validation: Exclusive Project vs Module
    if config.project.is_some() && config.module.is_some() {
        bail!("❌ Configuration Error: 'p.toml' cannot contain both [project] and [module] sections. Please use only one.");
    }

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
