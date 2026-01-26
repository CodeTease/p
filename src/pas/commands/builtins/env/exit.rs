// Exit command

use crate::pas::commands::Executable;
use crate::pas::context::ShellContext; 
use anyhow::Result;
use std::io::{Read, Write};

pub struct ExitCommand;
impl Executable for ExitCommand {
    fn execute(
        &self,
        args: &[String],
        _ctx: &mut ShellContext,
        _stdin: Option<Box<dyn Read + Send>>,
        mut _stdout: Option<Box<dyn Write + Send>>,
    ) -> Result<i32> {
        let exit_code = if args.len() > 1 {
            args[1].parse::<i32>().unwrap_or(0)
        } else {
            0
        };
        std::process::exit(exit_code);
    }
}