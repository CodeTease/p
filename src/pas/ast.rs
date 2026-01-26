#[derive(Debug, Clone, PartialEq)]
pub enum CommandExpr {
    // Simple command: "echo hello"
    Simple {
        program: String,
        args: Vec<String>,
    },
    // Pipeline: "ls | grep target"
    Pipe {
        left: Box<CommandExpr>,
        right: Box<CommandExpr>,
    },
    // Redirection: "echo logs > file.txt"
    Redirect {
        cmd: Box<CommandExpr>,
        target: String,
        mode: RedirectMode, // Create, Append, Input
    },
    // Logic AND: "cargo build && cargo run"
    And(Box<CommandExpr>, Box<CommandExpr>),
    // Logic OR: "cargo test || echo failed"
    Or(Box<CommandExpr>, Box<CommandExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectMode {
    Overwrite, // >
    Append,    // >>
    Input,     // <
}
