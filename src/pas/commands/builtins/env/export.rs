use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::Result;
use std::io::{Read, Write};

pub struct ExportCommand;

impl Executable for ExportCommand {
    fn execute(
        &self, 
        args: &[String], 
        ctx: &mut ShellContext, 
        _stdin: Option<Box<dyn Read + Send>>, 
        _stdout: Option<Box<dyn Write + Send>>,
        _stderr: Option<Box<dyn Write + Send>>,
    ) -> Result<i32> {
        // args[0] is "export"
        for arg in &args[1..] {
            if let Some(idx) = arg.find('=') {
                let key = arg[..idx].to_string();
                let value = arg[idx+1..].to_string();
                ctx.env.insert(key, value);
            }
        }
        Ok(0)
    }
}
