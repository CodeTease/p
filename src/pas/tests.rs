use crate::pas::context::ShellContext;
use crate::pas::commands::builtins::env::cd::CdCommand;
use crate::pas::commands::builtins::fs::rm::RmCommand;
use crate::pas::commands::Executable;
use crate::pas::parser::parse_command_line;
use crate::pas::ast::{CommandExpr, Arg, ArgPart};
use std::fs;

fn lit(s: &str) -> Arg {
    Arg(vec![ArgPart::Literal(s.to_string())])
}

fn var(s: &str) -> Arg {
    Arg(vec![ArgPart::Variable(s.to_string())])
}

#[test]
fn test_parser_basic() {
    let ctx = ShellContext::new();
    let expr = parse_command_line("echo hello world", &ctx).unwrap();
    if let CommandExpr::Simple { program, args } = expr {
        assert_eq!(program, lit("echo"));
        assert_eq!(args, vec![lit("hello"), lit("world")]);
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
        // Now it returns Variable part, not expanded value!
         assert_eq!(args[0], var("VAR"));
    } else {
        panic!("Expected Simple command");
    }
}

#[test]
fn test_cd_builtin() {
    let mut ctx = ShellContext::new();
    let cd = CdCommand;
    // execute now expects expanded args (Vec<String>) because executor calls it after expansion
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
    assert!(content.trim() == "hello");
    fs::remove_file(out_file).unwrap();
}

#[test]
fn test_logic_and() {
    let mut ctx = ShellContext::new();
    let out_file = ctx.cwd.join("test_and.txt");
    if out_file.exists() { fs::remove_file(&out_file).unwrap(); }

    let cmd = format!("echo 1 && echo 2 > {}", out_file.to_string_lossy());
    crate::pas::run_command_line(&cmd, &mut ctx).unwrap();
    
    assert!(out_file.exists());
    fs::remove_file(out_file).unwrap();
}

#[test]
fn test_pipe_simple() {
    let mut ctx = ShellContext::new();
    let out_file = ctx.cwd.join("test_pipe.txt");
    
    if cfg!(unix) {
        let cmd = format!("echo hello | grep hello > {}", out_file.to_string_lossy());
        crate::pas::run_command_line(&cmd, &mut ctx).unwrap();
        let content = fs::read_to_string(&out_file).unwrap();
        assert!(content.contains("hello"));
    }
    
    if out_file.exists() { fs::remove_file(out_file).unwrap(); }
}

#[test]
fn test_variable_assignment() {
    let mut ctx = ShellContext::new();
    crate::pas::run_command_line("A=10", &mut ctx).unwrap();
    assert_eq!(ctx.env.get("A").unwrap(), "10");
}

#[test]
fn test_variable_expansion_delayed() {
    let mut ctx = ShellContext::new();
    // This previously failed with static expansion
    crate::pas::run_command_line("A=10; echo $A", &mut ctx).unwrap();
    // We can't easily check stdout here but we verified assignment works.
    // We can use a side effect.
    crate::pas::run_command_line("A=file_delayed.txt; echo content > $A", &mut ctx).unwrap();
    assert!(ctx.cwd.join("file_delayed.txt").exists());
    fs::remove_file("file_delayed.txt").unwrap();
}

#[test]
fn test_if_else() {
    let mut ctx = ShellContext::new();
    crate::pas::run_command_line("if true; then A=yes; else A=no; fi", &mut ctx).unwrap();
    assert_eq!(ctx.env.get("A").unwrap(), "yes");
    
    crate::pas::run_command_line("if false; then B=yes; else B=no; fi", &mut ctx).unwrap();
    assert_eq!(ctx.env.get("B").unwrap(), "no");
}

#[test]
fn test_while_loop() {
    let mut ctx = ShellContext::new();
    if cfg!(unix) {
        // Now this should work because $A is expanded at runtime
        crate::pas::run_command_line("A=0; while test $A -ne 1; do A=1; done", &mut ctx).unwrap();
        assert_eq!(ctx.env.get("A").unwrap(), "1");
    }
}

#[test]
fn test_subshell() {
    let mut ctx = ShellContext::new();
    ctx.env.insert("OUTER".to_string(), "original".to_string());
    
    crate::pas::run_command_line("(OUTER=changed; INNER=created)", &mut ctx).unwrap();
    
    // Parent env should NOT change
    assert_eq!(ctx.env.get("OUTER").unwrap(), "original");
    assert!(ctx.env.get("INNER").is_none());
}

#[test]
fn test_sequence() {
    let mut ctx = ShellContext::new();
    crate::pas::run_command_line("A=1; A=2", &mut ctx).unwrap();
    assert_eq!(ctx.env.get("A").unwrap(), "2");
}
