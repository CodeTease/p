// Cat command 

use crate::pas::commands::Executable;
use crate::pas::context::ShellContext;
use anyhow::Result;
use std::fs::File;
use std::io::{Read, Write, BufReader};

pub struct CatCommand;
impl Executable for CatCommand {
    fn execute(
        &self,
        args: &[String],
        _ctx: &mut ShellContext,
        _stdin: Option<Box<dyn Read + Send>>,
        stdout: Option<Box<dyn Write + Send>>,
        _stderr: Option<Box<dyn Write + Send>>,
    ) -> Result<i32> {
        let mut out: Box<dyn Write + Send> = match stdout {
            Some(s) => s,
            None => Box::new(std::io::stdout()),
        };

        if args.len() < 2 {
            writeln!(out, "Usage: cat <file1> <file2> ...")?;
            return Ok(1);
        }

        for filename in &args[1..] {
            let file = File::open(filename)?;
            let mut reader = BufReader::new(file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;
            out.write_all(&buffer)?;
        }

        Ok(0)
    }
}