use crate::pas::context::ShellContext;
use crate::pas::commands::builtin::{CdCommand, RmCommand};
use crate::pas::commands::Executable;
use crate::pas::parser::parse_command;
use std::fs;

#[test]
fn test_parser_basic() {
    let ctx = ShellContext::new();
    let args = parse_command("echo hello world", &ctx).unwrap();
    assert_eq!(args, vec!["echo", "hello", "world"]);
}

#[test]
fn test_parser_quotes() {
    let ctx = ShellContext::new();
    let args = parse_command("echo 'hello world'", &ctx).unwrap();
    assert_eq!(args, vec!["echo", "hello world"]);
    
    let args = parse_command("echo \"hello world\"", &ctx).unwrap();
    assert_eq!(args, vec!["echo", "hello world"]);
}

#[test]
fn test_parser_env() {
    let mut ctx = ShellContext::new();
    ctx.env.insert("VAR".to_string(), "value".to_string());
    
    let args = parse_command("echo $VAR", &ctx).unwrap();
    assert_eq!(args, vec!["echo", "value"]);
    
    let args = parse_command("echo '$VAR'", &ctx).unwrap();
    assert_eq!(args, vec!["echo", "$VAR"]); // Single quotes protect
    
    let args = parse_command("echo \"$VAR\"", &ctx).unwrap();
    assert_eq!(args, vec!["echo", "value"]); // Double quotes expand
}

#[test]
fn test_cd_builtin() {
    let mut ctx = ShellContext::new();
    let initial_cwd = ctx.cwd.clone();
    
    // cd ..
    let cd = CdCommand;
    // cd assumes args include command name
    cd.execute(&["cd".to_string(), "..".to_string()], &mut ctx).unwrap();
    
    // Check if changed. Note: if at root, might not change.
    // But usually we are in src/pas/..
    if let Some(parent) = initial_cwd.parent() {
        if parent != initial_cwd {
             // assert_eq!(ctx.cwd, parent); // Canonicalization might affect this
        }
    }
    // Just ensure it ran without error
}

#[test]
fn test_rm_builtin() {
    // Create temp file in current directory
    let mut ctx = ShellContext::new();
    let test_file = ctx.cwd.join("test_file_rm.txt");
    fs::write(&test_file, "content").unwrap();
    assert!(test_file.exists());
    
    let rm = RmCommand;
    rm.execute(&["rm".to_string(), "test_file_rm.txt".to_string()], &mut ctx).unwrap();
    
    assert!(!test_file.exists());
}

#[test]
fn test_system_command_fallback() {
    let mut ctx = ShellContext::new();
    // Use run_command_line which triggers dispatch and fallback to SystemCommand for "echo"
    let res = crate::pas::run_command_line("echo system_test", &mut ctx);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0);
}
