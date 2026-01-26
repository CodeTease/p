pub mod fs;
pub mod env;
pub mod io;
pub mod common; // Private helpers

use crate::pas::context::ShellContext;

/* Commands note with '//' means they are too simple or too popular/classic
   to need portable version (p:). So only one version is registered.
   Specifically, 'echo' is only registered as normal command, not p:echo,
   because it's the built-in command that every shell has.
*/

/// Helper to register all built-in commands at once
pub fn register_all_builtins(ctx: &mut ShellContext) {
    // FS commands
    ctx.register_command("rm", Box::new(fs::rm::RmCommand));
    ctx.register_command("p:rm", Box::new(fs::rm::RmCommand));
    ctx.register_command("mkdir", Box::new(fs::mkdir::MkdirCommand));
    ctx.register_command("p:mkdir", Box::new(fs::mkdir::MkdirCommand));
    ctx.register_command("cp", Box::new(fs::cp::CpCommand));
    ctx.register_command("p:cp", Box::new(fs::cp::CpCommand));
    ctx.register_command("ls", Box::new(fs::ls::LsCommand)); 
    ctx.register_command("p:ls", Box::new(fs::ls::LsCommand));
    ctx.register_command("mv", Box::new(fs::mv::MvCommand));
    ctx.register_command("p:mv", Box::new(fs::mv::MvCommand));

    // Env/Navigation
    ctx.register_command("cd", Box::new(env::cd::CdCommand)); // `cd` in CMD (without args) is like `pwd`? I'll look into it later
    ctx.register_command("exit", Box::new(env::exit::ExitCommand)); //

    // IO
    ctx.register_command("echo", Box::new(io::echo::EchoCommand)); //
    ctx.register_command("cat", Box::new(io::cat::CatCommand)); 
    ctx.register_command("p:cat", Box::new(io::cat::CatCommand)); 
}