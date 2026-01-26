pub mod builtin;
pub mod system;
pub mod adapter;

use crate::pas::context::ShellContext;
use anyhow::Result;
use std::io::{Read, Write};

pub trait Executable: Send + Sync {
    fn execute(
        &self, 
        args: &[String], 
        ctx: &mut ShellContext, 
        stdin: Option<Box<dyn Read + Send>>, 
        stdout: Option<Box<dyn Write + Send>>
    ) -> Result<i32>;
}
