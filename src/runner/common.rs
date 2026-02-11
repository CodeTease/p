use anyhow::Result;
use std::fs;
use std::path::Path;
use glob::glob;

pub fn expand_globs(args: &[String]) -> Vec<String> {
    let mut expanded_args = Vec::new();

    for arg in args {
        // Skip flags
        if arg.starts_with('-') {
            expanded_args.push(arg.clone());
            continue;
        }

        // Check for glob characters
        if arg.contains('*') || arg.contains('?') || arg.contains('[') {
             match glob(arg) {
                Ok(paths) => {
                    let mut matched_paths = Vec::new();
                    for entry in paths {
                        if let Ok(path) = entry {
                            matched_paths.push(path.to_string_lossy().to_string());
                        }
                    }
                    
                    if matched_paths.is_empty() {
                         // No matches found, keep original argument (bash behavior)
                         expanded_args.push(arg.clone());
                    } else {
                        // Sort to ensure deterministic behavior (like shell expansion)
                        matched_paths.sort();
                        expanded_args.extend(matched_paths);
                    }
                },
                Err(_) => {
                    // Invalid pattern, keep original argument
                    expanded_args.push(arg.clone());
                }
            }
        } else {
            expanded_args.push(arg.clone());
        }
    }
    expanded_args
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_expand_globs() {
        // Setup
        let _ = File::create("test_glob_a.tmp");
        let _ = File::create("test_glob_b.tmp");

        let args = vec!["test_glob_*.tmp".to_string()];
        let expanded = expand_globs(&args);

        // Teardown
        let _ = fs::remove_file("test_glob_a.tmp");
        let _ = fs::remove_file("test_glob_b.tmp");

        assert_eq!(expanded.len(), 2);
        assert!(expanded.contains(&"test_glob_a.tmp".to_string()));
        assert!(expanded.contains(&"test_glob_b.tmp".to_string()));
    }

    #[test]
    fn test_expand_globs_no_match() {
        let args = vec!["*.nomatch".to_string()];
        let expanded = expand_globs(&args);
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0], "*.nomatch");
    }
}
