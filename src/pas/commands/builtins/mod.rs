pub mod fs;
pub mod env;
pub mod io;
pub mod common; // Private helpers

use crate::pas::context::ShellContext;

/// Helper to register all built-in commands at once
pub fn register_all_builtins(ctx: &mut ShellContext) {
    // FS commands
    ctx.register_command("rm", Box::new(fs::rm::RmCommand));
    ctx.register_command("p:rm", Box::new(fs::rm::RmCommand));
    ctx.register_command("mkdir", Box::new(fs::mkdir::MkdirCommand));
    ctx.register_command("p:mkdir", Box::new(fs::mkdir::MkdirCommand));
    ctx.register_command("cp", Box::new(fs::cp::CpCommand));
    ctx.register_command("p:cp", Box::new(fs::cp::CpCommand));

    // Env/Navigation
    ctx.register_command("cd", Box::new(env::cd::CdCommand));

    // IO
    ctx.register_command("echo", Box::new(io::echo::EchoCommand));
}