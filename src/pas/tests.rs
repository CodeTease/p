use crate::pas::context::ShellContext;
use crate::pas::commands::builtin::{CdCommand, RmCommand};
use crate::pas::commands::Executable;
use crate::pas::parser::parse_command_line;
use crate::pas::ast::CommandExpr;
use std::fs;

#[test]
fn test_parser_basic() {
    let ctx = ShellContext::new();
    let expr = parse_command_line("echo hello world", &ctx).unwrap();
    if let CommandExpr::Simple { program, args } = expr {
        assert_eq!(program, "echo");
        assert_eq!(args, vec!["hello", "world"]);
    } else {
        panic!("Expected Simple command");
    }
}

#[test]
fn test_parser_env() {
    let mut ctx = ShellContext::new();
    ctx.env.insert("VAR".to_string(), "value".to_string());
    
    let expr = parse_command_line("echo $VAR", &ctx).unwrap();
    if let CommandExpr::Simple { args, .. } = expr {
         assert_eq!(args, vec!["value"]);
    } else {
        panic!("Expected Simple command");
    }
}

#[test]
fn test_cd_builtin() {
    let mut ctx = ShellContext::new();
    let cd = CdCommand;
    cd.execute(&["cd".to_string(), "..".to_string()], &mut ctx, None, None).unwrap();
}

#[test]
fn test_rm_builtin() {
    let mut ctx = ShellContext::new();
    let test_file = ctx.cwd.join("test_file_rm.txt");
    fs::write(&test_file, "content").unwrap();
    
    let rm = RmCommand;
    rm.execute(&["rm".to_string(), "test_file_rm.txt".to_string()], &mut ctx, None, None).unwrap();
    
    assert!(!test_file.exists());
}

#[test]
fn test_system_command_fallback() {
    let mut ctx = ShellContext::new();
    let res = crate::pas::run_command_line("echo system_test", &mut ctx);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0);
}

#[test]
fn test_redirect_output() {
    let mut ctx = ShellContext::new();
    let out_file = ctx.cwd.join("test_redirect.txt");
    if out_file.exists() { fs::remove_file(&out_file).unwrap(); }
    
    let cmd = format!("echo hello > {}", out_file.to_string_lossy());
    crate::pas::run_command_line(&cmd, &mut ctx).unwrap();
    
    assert!(out_file.exists());
    let content = fs::read_to_string(&out_file).unwrap();
    // echo usually outputs newline
    assert!(content.trim() == "hello");
    fs::remove_file(out_file).unwrap();
}

#[test]
fn test_logic_and() {
    let mut ctx = ShellContext::new();
    let out_file = ctx.cwd.join("test_and.txt");
    if out_file.exists() { fs::remove_file(&out_file).unwrap(); }

    // First command succeeds, second runs
    // Note: echo usually returns 0
    let cmd = format!("echo 1 && echo 2 > {}", out_file.to_string_lossy());
    crate::pas::run_command_line(&cmd, &mut ctx).unwrap();
    
    assert!(out_file.exists());
    fs::remove_file(out_file).unwrap();
}

#[test]
fn test_pipe_simple() {
    // Note: cat might not be available on Windows?
    // Use `more`? Or `findstr`? 
    // Usually `sort` is available on both.
    // `echo "b\na" | sort`
    
    // For universal test, maybe `echo hello | cat` (Unix) or `type` (Windows)?
    // But `echo` is builtin (via system/fallback).
    
    let mut ctx = ShellContext::new();
    let out_file = ctx.cwd.join("test_pipe.txt");
    
    // Using a command that exists. `echo` exists.
    // Piping echo to something.
    // On Linux/Mac: `echo hello | rev > file`.
    // On Windows: `echo hello | sort` works?
    
    // Let's rely on basic commands.
    // If I cannot guarantee OS commands, this test might be flaky.
    // But this is unit test on local env.
    
    if cfg!(unix) {
        let cmd = format!("echo hello | cat > {}", out_file.to_string_lossy());
        crate::pas::run_command_line(&cmd, &mut ctx).unwrap();
        let content = fs::read_to_string(&out_file).unwrap();
        assert!(content.contains("hello"));
    }
    
    if out_file.exists() { fs::remove_file(out_file).unwrap(); }
}
