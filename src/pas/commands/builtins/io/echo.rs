// Echo command

use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::Result;
use std::io::{Read, Write};

pub struct EchoCommand;

impl Executable for EchoCommand {
    fn execute(
        &self,
        args: &[String],
        _ctx: &mut ShellContext,
        _stdin: Option<Box<dyn Read + Send>>,
        stdout: Option<Box<dyn Write + Send>>,
    ) -> Result<i32> {
        // Skip "echo" in args[0]
        let output = args[1..].join(" ");
        
        if let Some(mut out) = stdout {
            writeln!(out, "{}", output)?;
        } else {
            println!("{}", output);
        }
        
        Ok(0)
    }
}