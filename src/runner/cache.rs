use anyhow::Result;
use std::fs;
use std::time::SystemTime;

/// Check if outputs are newer than sources based on modification time (mtime).
pub fn is_up_to_date(sources: &[String], outputs: &[String]) -> Result<bool> {
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

    if !has_src || !has_out {
        return Ok(false);
    }

    Ok(latest_src < oldest_out)
}
