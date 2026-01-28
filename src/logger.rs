use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::Local;
use regex::Regex;
use crate::config::{PavidiConfig, LogStrategy};
use std::time::Duration;
use blake3::Hasher;

pub fn strip_ansi(content: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(content, "").to_string()
}

pub fn write_log(
    task_name: &str,
    cmd_str: &str,
    content: &str,
    config: &PavidiConfig,
    duration: Duration,
    exit_code: i32,
    env_vars: &HashMap<String, String>
) -> Result<Option<PathBuf>> {
    // 1. Determine Strategy
    let (strategy, log_plain) = if let Some(p) = &config.project {
        (p.log_strategy, p.log_plain.unwrap_or(true))
    } else if let Some(m) = &config.module {
        (m.log_strategy, m.log_plain.unwrap_or(true))
    } else {
        (None, true)
    };

    let strategy = strategy.unwrap_or(LogStrategy::None);

    match strategy {
        LogStrategy::None => return Ok(None),
        LogStrategy::ErrorOnly => {
            if exit_code == 0 {
                return Ok(None);
            }
        },
        LogStrategy::Always => {},
    }

    // 2. Generate Path
    let now = Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H%M%S").to_string();
    
    // Short Hash
    let mut hasher = Hasher::new();
    hasher.update(task_name.as_bytes());
    hasher.update(time_str.as_bytes());
    let hash_full = hasher.finalize().to_hex().to_string();
    let short_hash = &hash_full[0..6];

    let filename = format!("{}_{}_{}.log", time_str, task_name.replace("/", "_"), short_hash);
    let log_dir = Path::new(".p").join("logs").join(date_str).join(exit_code.to_string());
    
    fs::create_dir_all(&log_dir).context("Failed to create log directory")?;
    let log_path = log_dir.join(filename);

    // 3. Format Content
    let mut file_content = String::new();
    
    // Header
    file_content.push_str("=== PAVIDI EXECUTION LOG ===\n");
    file_content.push_str(&format!("Task: {}\n", task_name));
    file_content.push_str(&format!("Command: {}\n", cmd_str));
    file_content.push_str(&format!("Time: {}\n", now.to_rfc3339()));
    file_content.push_str("=== ENVIRONMENT SNAPSHOT ===\n");
    
    // Filter sensitive envs
    let mut sorted_keys: Vec<_> = env_vars.keys().collect();
    sorted_keys.sort();
    
    for k in sorted_keys {
        let v = &env_vars[k];
        let k_upper = k.to_uppercase();
        if k_upper.contains("KEY") || k_upper.contains("TOKEN") || k_upper.contains("PASS") || k_upper.contains("SECRET") {
             file_content.push_str(&format!("{} = [REDACTED]\n", k));
        } else {
             file_content.push_str(&format!("{} = {}\n", k, v));
        }
    }
    file_content.push_str("============================\n\n");

    // Body
    let body = if log_plain {
        strip_ansi(content)
    } else {
        content.to_string()
    };
    file_content.push_str(&body);
    if !body.ends_with('\n') {
        file_content.push('\n');
    }

    // Footer
    file_content.push_str("\n============================\n");
    file_content.push_str(&format!("Exit Code: {}\n", exit_code));
    file_content.push_str(&format!("Duration: {} ms\n", duration.as_millis()));
    file_content.push_str(&format!("End Time: {}\n", Local::now().to_rfc3339()));
    file_content.push_str("============================\n");

    fs::write(&log_path, file_content).context("Failed to write log file")?;

    Ok(Some(log_path))
}
