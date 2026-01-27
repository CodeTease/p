pub mod context;
pub mod commands;
pub mod parser;
pub mod ast;
pub mod executor;

use context::ShellContext;
use executor::execute_expr;
use anyhow::Result;

#[cfg(test)]
mod tests;

pub fn run_command_line(cmd_str: &str, ctx: &mut ShellContext) -> Result<i32> {
    let expr = parser::parse_command_line(cmd_str, ctx)?;
    execute_expr(expr, ctx, None, None, None)
}
