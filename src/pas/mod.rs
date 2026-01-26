pub mod context;
pub mod commands;
pub mod parser;

use context::ShellContext;
use commands::system::SystemCommand;
use commands::Executable;
use anyhow::Result;

#[cfg(test)]
mod tests;

pub fn run_command_line(cmd_str: &str, ctx: &mut ShellContext) -> Result<i32> {
    let args = parser::parse_command(cmd_str, ctx)?;
    if args.is_empty() {
        return Ok(0);
    }
    
    let cmd_name = &args[0];
    
    // Look up in registry
    // Registry is Arc, so we can access it.
    // Note: We need to clone the Box or Reference to execute?
    // Map stores Box<dyn Executable>. We can get reference.
    // Executable::execute takes &self.
    
    // We can't hold reference to registry (in ctx) while mutating ctx passed to execute.
    // ctx.registry borrow vs ctx mutable borrow.
    // This is a classic Rust borrow checker issue.
    // `ctx.registry` is a field of `ctx`.
    // `cmd.execute(args, ctx)` takes `&self` (from registry) and `&mut ctx`.
    // If `cmd` borrows from `ctx.registry`, and we pass `&mut ctx`, we have aliasing.
    
    // Solution:
    // 1. Clone the command? `Box<dyn Executable>` is not Clone.
    // 2. Registry is `Arc<HashMap...>`.
    //    We can clone the Arc!
    //    `let registry = ctx.registry.clone();`
    //    `let cmd = registry.get(cmd_name);`
    //    Now `cmd` borrows from `registry` (local Arc), not `ctx`.
    //    Then we can pass `&mut ctx` to execute.
    //    This works because `registry` is disjoint from `ctx` (mostly, except `ctx` holds another Arc).
    
    let registry = ctx.registry.clone();
    
    if let Some(cmd) = registry.get(cmd_name) {
        cmd.execute(&args, ctx)
    } else {
        // Fallback to SystemCommand
        let sys_cmd = SystemCommand;
        sys_cmd.execute(&args, ctx)
    }
}
