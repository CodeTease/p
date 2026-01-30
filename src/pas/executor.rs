use crate::pas::ast::{CommandExpr, RedirectMode, Arg, ArgPart};
use crate::pas::context::ShellContext;
use crate::pas::commands::system::SystemCommand;
use crate::pas::commands::Executable;
use anyhow::{Result, Context};
use std::io::{Read, Write};
use std::fs::OpenOptions;
use std::thread;
use os_pipe::pipe;
use std::path::MAIN_SEPARATOR;
use std::sync::{Arc, Mutex};

// SharedWriter allows cloning a writer handle (by sharing the underlying writer via Arc+Mutex)
#[derive(Clone)]
struct SharedWriter(Arc<Mutex<Box<dyn Write + Send>>>);

impl SharedWriter {
    fn new(w: Box<dyn Write + Send>) -> Self {
        Self(Arc::new(Mutex::new(w)))
    }
}

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

pub fn execute_expr(
    expr: CommandExpr, 
    ctx: &mut ShellContext, 
    stdin: Option<Box<dyn Read + Send>>, 
    stdout: Option<Box<dyn Write + Send>>,
    stderr: Option<Box<dyn Write + Send>>,
) -> Result<i32> {
    // Wrap stderr if present to allow sharing across sequence/logic branches
    let stderr_shared = stderr.map(SharedWriter::new);

    let get_stderr = || -> Option<Box<dyn Write + Send>> {
        stderr_shared.as_ref().map(|s| Box::new(s.clone()) as Box<dyn Write + Send>)
    };

    match expr {
        CommandExpr::Simple { program, args } => {
            let prog_str = expand_arg(&program, ctx);
            let mut full_args = vec![prog_str.clone()];
            
            for arg in args {
                let arg_str = expand_arg(&arg, ctx);
                let has_wildcard = arg_str.contains('*') || arg_str.contains('?') || arg_str.contains('[');
                if has_wildcard {
                    let mut found = false;
                    if let Ok(paths) = glob::glob(&arg_str) {
                        for entry in paths {
                            if let Ok(path) = entry {
                                full_args.push(path.to_string_lossy().into_owned());
                                found = true;
                            }
                        }
                    }
                    if !found {
                        full_args.push(arg_str);
                    }
                } else {
                    full_args.push(arg_str);
                }
            }
            
            let registry = ctx.registry.clone();
            let exit_code = if let Some(cmd) = registry.get(&prog_str) {
                cmd.execute(&full_args, ctx, stdin, stdout, get_stderr())?
            } else {
                let sys_cmd = SystemCommand;
                sys_cmd.execute(&full_args, ctx, stdin, stdout, get_stderr())?
            };
            
            ctx.exit_code = exit_code;
            Ok(exit_code)
        },
        CommandExpr::Pipe { left, right } => {
            let (reader, writer) = pipe().context("Failed to create pipe")?;
            let mut ctx_left = ctx.clone_for_parallel();
            let err_left = get_stderr();
            let left_thread = thread::spawn(move || {
                execute_expr(*left, &mut ctx_left, stdin, Some(Box::new(writer)), err_left)
            });
            let right_res = execute_expr(*right, ctx, Some(Box::new(reader)), stdout, get_stderr());
            let _ = left_thread.join().unwrap();
            right_res
        },
        CommandExpr::Redirect { cmd, target, mode, source_fd } => {
            if let RedirectMode::MergeStderrToStdout = mode {
                 // 2>&1 case. Redirect stderr to where stdout is going.
                 if let Some(out) = stdout {
                     let shared = SharedWriter::new(out);
                     let out_clone = Box::new(shared.clone());
                     let err_clone = Box::new(shared.clone());
                     // If we are redirecting 2>&1, we ignore the current stderr (get_stderr result)
                     // and replace it with stdout's handle.
                     // But wait, what if we have `3>&1`? We only support 2>&1 via MergeStderrToStdout variant implies.
                     execute_expr(*cmd, ctx, stdin, Some(out_clone), Some(err_clone))
                 } else {
                     // If no stdout is captured, inherit both?
                     // Or force both to inherit.
                     execute_expr(*cmd, ctx, stdin, None, None)
                 }
            } else {
                let target_str = expand_arg(&target, ctx);
                let mut open_opts = OpenOptions::new();
                match mode {
                    RedirectMode::Overwrite => { open_opts.write(true).create(true).truncate(true); },
                    RedirectMode::Append => { open_opts.write(true).create(true).append(true); },
                    RedirectMode::Input => { open_opts.read(true); },
                    _ => unreachable!(),
                };
                let file = open_opts.open(&target_str).with_context(|| format!("Failed to open file: {}", target_str))?;
                
                if mode == RedirectMode::Input {
                    execute_expr(*cmd, ctx, Some(Box::new(file)), stdout, get_stderr())
                } else {
                    // Output redirection
                    let file_box = Box::new(file);
                    if source_fd == 2 {
                        execute_expr(*cmd, ctx, stdin, stdout, Some(file_box))
                    } else {
                        // Default to stdout (1)
                        execute_expr(*cmd, ctx, stdin, Some(file_box), get_stderr())
                    }
                }
            }
        },
        CommandExpr::And(left, right) => {
            handle_sequence(*left, Some(*right), ctx, stdin, stdout, get_stderr(), SequenceMode::And)
        },
        CommandExpr::Or(left, right) => {
            handle_sequence(*left, Some(*right), ctx, stdin, stdout, get_stderr(), SequenceMode::Or)
        },
        CommandExpr::Sequence(left, right) => {
            handle_sequence(*left, Some(*right), ctx, stdin, stdout, get_stderr(), SequenceMode::Always)
        },
        CommandExpr::Assignment { key, value } => {
            let val_str = expand_arg(&value, ctx);
            ctx.env.insert(key, val_str);
            Ok(0)
        },
        CommandExpr::Subshell(cmd) => {
            let mut sub_ctx = ctx.clone_for_parallel();
            execute_expr(*cmd, &mut sub_ctx, stdin, stdout, get_stderr())
        },
        CommandExpr::If { cond, then_branch, else_branch } => {
            let cond_res = execute_expr(*cond, ctx, stdin, None, get_stderr())?;
            if cond_res == 0 {
                execute_expr(*then_branch, ctx, None, stdout, get_stderr())
            } else if let Some(else_block) = else_branch {
                execute_expr(*else_block, ctx, None, stdout, get_stderr())
            } else {
                Ok(0)
            }
        },
        CommandExpr::While { cond, body } => {
            loop {
                // We clone the Box<CommandExpr>. 
                let cond_val = *cond.clone();
                let res = execute_expr(cond_val, ctx, None, None, get_stderr())?;
                if res == 0 {
                    let body_val = *body.clone();
                    execute_expr(body_val, ctx, None, None, get_stderr())?;
                } else {
                    break;
                }
            }
            Ok(0)
        }
    }
}

fn expand_arg(arg: &Arg, ctx: &ShellContext) -> String {
    let mut res = String::new();
    let mut iter = arg.0.iter();

    if let Some(first) = iter.next() {
        match first {
            ArgPart::Literal(s) => {
                if s == "~" || s.starts_with("~/") {
                    if let Some(home) = ctx.env.get("HOME") {
                        res.push_str(home);
                        res.push_str(&s[1..]);
                    } else {
                        res.push_str(s);
                    }
                } else {
                    res.push_str(s);
                }
            }
            ArgPart::Variable(name) => {
                if name == "?" {
                    res.push_str(&ctx.exit_code.to_string());
                } else if let Some(val) = ctx.env.get(name) {
                    res.push_str(val);
                }
            }
        }
    }

    for part in iter {
        match part {
            ArgPart::Literal(s) => res.push_str(s),
            ArgPart::Variable(name) => {
                if name == "?" {
                    res.push_str(&ctx.exit_code.to_string());
                } else if let Some(val) = ctx.env.get(name) {
                    res.push_str(val);
                }
            }
        }
    }
    // Windows normalization?
    if cfg!(windows) && res.contains('/') {
        res = res.replace('/', &MAIN_SEPARATOR.to_string());
    }
    res
}

#[derive(PartialEq)]
enum SequenceMode {
    And,
    Or,
    Always,
}

fn handle_sequence(
    left: CommandExpr,
    right: Option<CommandExpr>,
    ctx: &mut ShellContext,
    stdin: Option<Box<dyn Read + Send>>,
    stdout: Option<Box<dyn Write + Send>>,
    stderr: Option<Box<dyn Write + Send>>,
    mode: SequenceMode
) -> Result<i32> {
    let stderr_shared = stderr.map(SharedWriter::new);
    let get_stderr = || -> Option<Box<dyn Write + Send>> {
        stderr_shared.as_ref().map(|s| Box::new(s.clone()) as Box<dyn Write + Send>)
    };

    if let Some(out) = stdout {
        let (mut reader, writer) = pipe().context("Failed to create bridge pipe")?;
        let mut out_sink = out;
        let bridge_thread = thread::spawn(move || {
            std::io::copy(&mut reader, &mut out_sink).ok();
        });
        let w1 = writer.try_clone().context("Failed to clone pipe writer")?;
        let w2 = writer; 
        
        let left_res = execute_expr(left, ctx, stdin, Some(Box::new(w1)), get_stderr())?;
        
        let proceed = match mode {
            SequenceMode::And => left_res == 0,
            SequenceMode::Or => left_res != 0,
            SequenceMode::Always => true,
        };
        
        let final_res = if proceed {
             if let Some(r) = right {
                 execute_expr(r, ctx, None, Some(Box::new(w2)), get_stderr())?
             } else {
                 left_res
             }
        } else {
            drop(w2);
            left_res
        };
        bridge_thread.join().unwrap();
        Ok(final_res)
    } else {
        let left_res = execute_expr(left, ctx, stdin, None, get_stderr())?;
        let proceed = match mode {
            SequenceMode::And => left_res == 0,
            SequenceMode::Or => left_res != 0,
            SequenceMode::Always => true,
        };
        if proceed {
            if let Some(r) = right {
                 execute_expr(r, ctx, None, None, get_stderr())
            } else {
                Ok(left_res)
            }
        } else {
            Ok(left_res)
        }
    }
}
