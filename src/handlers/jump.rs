use anyhow::{Context, Result, bail};
use std::fs;
use std::env;
use std::path::PathBuf;

pub fn handle_jump(target_path: PathBuf) -> Result<()> {
    if !target_path.exists() {
         bail!("Path does not exist: {:?}", target_path);
    }
    
    let abs_path = fs::canonicalize(&target_path)
        .context("Failed to resolve path")?;

    // Output for external tools (like shell aliases) to capture the path
    if let Ok(output_file) = env::var("PAVIDI_OUTPUT") {
        fs::write(output_file, abs_path.to_string_lossy().as_bytes())
            .context("Failed to write jump path")?;
    } else {
        println!("{}", abs_path.to_string_lossy());
    }

    Ok(())
}
