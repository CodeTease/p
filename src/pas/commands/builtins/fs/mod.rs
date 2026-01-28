use crate::pas::context::ShellContext;
use anyhow::{Result, bail};
use std::path::Path;
use std::fs;

pub mod cp;
pub mod mkdir;
pub mod rm;
pub mod ls;
pub mod mv;

pub fn check_path_access(target: &Path, ctx: &ShellContext) -> Result<()> {
    if let Some(caps) = &ctx.capabilities {
        if let Some(allowed_strs) = &caps.allow_paths {
             // 1. Resolve target to absolute path to remove ambiguity
             let abs_target = if target.is_absolute() {
                 target.to_path_buf()
             } else {
                 ctx.cwd.join(target)
             };
             
             // 2. Canonicalize target (handle .. symlinks)
             // If target does not exist, try to canonicalize parent.
             let canonical_target = match fs::canonicalize(&abs_target) {
                 Ok(p) => p,
                 Err(_) => {
                     // If path doesn't exist, try parent
                     if let Some(parent) = abs_target.parent() {
                         if let Ok(canon_parent) = fs::canonicalize(parent) {
                             canon_parent.join(abs_target.file_name().unwrap_or_default())
                         } else {
                             abs_target.clone()
                         }
                     } else {
                         abs_target.clone()
                     }
                 }
             };

             // 3. Check against allowed paths
             // NOTE: We assume allowed_strs are either absolute or we match them as prefix components.
             // If allow_paths = ["src"], and we access "/abs/to/proj/src/file", it won't match "src".
             // We really need resolved allowed paths. 
             // Ideally, Config loading should resolve these relative to project root.
             // For now, we attempt to match strictly. 
             
             let mut denied = true;
             for allowed in allowed_strs {
                 let allowed_path = Path::new(allowed);
                 // Check if canonical_target starts with allowed_path
                 if canonical_target.starts_with(allowed_path) {
                     denied = false;
                     break;
                 }
                 
                 // If allowed is relative, and we are running inside the directory...
                 // This is tricky without knowing project root.
                 // We'll fallback to simple string/path matching.
             }
             
             if denied {
                 bail!("🚫 Security: Access to '{}' is denied by allow_paths.", target.display());
             }
        }
    }
    Ok(())
}
