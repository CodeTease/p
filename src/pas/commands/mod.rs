pub mod builtin;
pub mod system;
pub mod adapter;

use crate::pas::context::ShellContext;
use anyhow::Result;

pub trait Executable: Send + Sync {
    fn execute(&self, args: &[String], ctx: &mut ShellContext) -> Result<i32>;
}
