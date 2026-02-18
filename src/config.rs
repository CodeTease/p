use anyhow::{Context, Result, bail};
use colored::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use crate::runner::task::RunnerTask;
use regex::Regex;
use crate::utils::{run_shell_command, CaptureMode, detect_shell};

#[derive(Debug, Deserialize)]
pub struct PavidiConfig {
    pub project: Option<ProjectConfig>,
    pub module: Option<ModuleConfig>,
    pub capability: Option<CapabilityConfig>,
    #[serde(default)] 
    pub env: HashMap<String, String>,
    pub runner: Option<HashMap<String, RunnerTask>>,

    #[serde(skip)]
    pub env_provenance: HashMap<String, Vec<(String, String)>>,
    #[serde(skip)]
    pub extensions_applied: Vec<(String, Metadata)>,
    #[serde(skip)]
    pub original_metadata: Option<Metadata>,
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
    pub secret_patterns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ModuleConfig {
    #[serde(flatten)]
    pub metadata: Metadata,
    pub shell: Option<String>,
    pub log_strategy: Option<LogStrategy>,
    pub log_plain: Option<bool>,
    pub secret_patterns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CapabilityConfig {
    pub allow_paths: Option<Vec<String>>,
}

fn merge_configurations(base: &mut PavidiConfig, extension: PavidiConfig) {
    // Merge Env (Overwrite)
    base.env.extend(extension.env);

    // Merge Runner Tasks (Overwrite)
    if let Some(ext_runner) = extension.runner {
        let base_runner = base.runner.get_or_insert_with(HashMap::new);
        base_runner.extend(ext_runner);
    }

    // Merge Capability (Allow Paths) - Append unique paths
    if let Some(ext_cap) = extension.capability {
        if let Some(ext_paths) = ext_cap.allow_paths {
            let base_cap = base.capability.get_or_insert(CapabilityConfig { allow_paths: Some(vec![]) });
            let base_paths = base_cap.allow_paths.get_or_insert(vec![]);
            for p in ext_paths {
                if !base_paths.contains(&p) {
                    base_paths.push(p);
                }
            }
        }
    }

    // Merge Project Config (Settings only)
    if let Some(ext_proj) = extension.project {
        if let Some(base_proj) = &mut base.project {
            if let Some(s) = ext_proj.shell { base_proj.shell = Some(s); }
            if let Some(l) = ext_proj.log_strategy { base_proj.log_strategy = Some(l); }
            if let Some(p) = ext_proj.log_plain { base_proj.log_plain = Some(p); }
            
            // Append secret patterns
            if let Some(ext_patterns) = ext_proj.secret_patterns {
                let base_patterns = base_proj.secret_patterns.get_or_insert(vec![]);
                base_patterns.extend(ext_patterns);
            }
        }
    }

    // Merge Module Config (Settings only)
    if let Some(ext_mod) = extension.module {
        if let Some(base_mod) = &mut base.module {
            if let Some(s) = ext_mod.shell { base_mod.shell = Some(s); }
            if let Some(l) = ext_mod.log_strategy { base_mod.log_strategy = Some(l); }
            if let Some(p) = ext_mod.log_plain { base_mod.log_plain = Some(p); }

            // Append secret patterns
            if let Some(ext_patterns) = ext_mod.secret_patterns {
                let base_patterns = base_mod.secret_patterns.get_or_insert(vec![]);
                base_patterns.extend(ext_patterns);
            }
        }
    }
}

pub fn load_config(dir: &Path) -> Result<PavidiConfig> {
    let config_path = dir.join("p.toml");
    if !config_path.exists() {
        bail!("❌ Critical: 'p.toml' not found in {:?}.", dir);
    }
    let content = fs::read_to_string(&config_path).context("Failed to read p.toml")?;
    
    // 1. Parse p.toml (Base Layer)
    let mut config: PavidiConfig = toml::from_str(&content).context("Failed to parse p.toml")?;

    // Initialize provenance tracking
    config.env_provenance = HashMap::new();
    for (k, v) in &config.env {
        config.env_provenance.insert(k.clone(), vec![("p.toml".to_string(), v.clone())]);
    }

    // Capture original metadata
    if let Some(p) = &config.project {
        config.original_metadata = Some(p.metadata.clone());
    } else if let Some(m) = &config.module {
        config.original_metadata = Some(m.metadata.clone());
    }
    
    config.extensions_applied = Vec::new();

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

    // 1.5 Load Extensions (p.*.toml)
    let pattern = dir.join("p.*.toml");
    let pattern_str = pattern.to_str().ok_or_else(|| anyhow::anyhow!("Invalid path pattern"))?;
    
    let mut extension_files: Vec<PathBuf> = glob::glob(pattern_str)?
        .filter_map(Result::ok)
        .collect();
    
    // Sort alphabetically to ensure deterministic order
    extension_files.sort();

    for ext_path in extension_files {
        eprintln!("{} Loading extension config: {}", "➕".blue(), ext_path.file_name().unwrap().to_string_lossy());
        let ext_content = fs::read_to_string(&ext_path).context("Failed to read extension config")?;
        let mut ext_config: PavidiConfig = toml::from_str(&ext_content).context("Failed to parse extension config")?;

        let ext_name = ext_path.file_name().unwrap().to_string_lossy().to_string();

        // Capture extension metadata
        let meta = if let Some(p) = &ext_config.project {
            p.metadata.clone()
        } else if let Some(m) = &ext_config.module {
            m.metadata.clone()
        } else {
            Metadata { name: None, version: None, authors: None, description: None }
        };
        config.extensions_applied.push((ext_name.clone(), meta));

        // Update provenance for vars in extension
        for (k, v) in &ext_config.env {
            config.env_provenance.entry(k.clone()).or_default().push((ext_name.clone(), v.clone()));
        }

        // Resolve relative paths in extension capability BEFORE merging
        if let Some(caps) = &mut ext_config.capability {
             if let Some(paths) = &mut caps.allow_paths {
                let resolved: Vec<String> = paths.iter().map(|p| {
                    let path = Path::new(p);
                    if path.is_absolute() {
                        p.clone()
                    } else {
                        // Resolve relative to the directory
                        dir.join(p).to_string_lossy().into_owned()
                    }
                }).collect();
                *paths = resolved;
            }
        }

        merge_configurations(&mut config, ext_config);
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
            
            // Track provenance
            config.env_provenance.entry(key.clone()).or_default().push((env_filename.clone(), val.clone()));
            
            // .env overrides p.toml
            config.env.insert(key, val);
        }
    }

    // 3. Dynamic Env Var Resolution
    let shell_pref = config.project.as_ref().and_then(|p| p.shell.as_ref())
        .or(config.module.as_ref().and_then(|m| m.shell.as_ref()));
    let shell = detect_shell(shell_pref);
    
    let re = Regex::new(r"^\$\((.*)\)$").unwrap();
    let mut updates = HashMap::new();

    for (k, v) in &config.env {
        if let Some(caps) = re.captures(v) {
            let cmd = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if !cmd.trim().is_empty() {
                // Execute command
                let (code, output) = run_shell_command(
                    cmd, 
                    &config.env, 
                    CaptureMode::Buffer,
                    &format!("env:{}", k),
                    &shell,
                    None 
                )?;
                
                if code != 0 {
                    bail!("❌ Failed to resolve dynamic environment variable '{}': Command '{}' failed with exit code {}.", k, cmd, code);
                }
                
                updates.insert(k.clone(), output.trim().to_string());
            }
        }
    }
    
    // Update provenance for dynamic vars
    for (k, v) in &updates {
        config.env_provenance.entry(k.clone()).or_default().push(("dynamic".to_string(), v.clone()));
    }
    
    config.env.extend(updates);

    Ok(config)
}
