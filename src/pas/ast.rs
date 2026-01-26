#[derive(Debug, Clone, PartialEq)]
pub enum ArgPart {
    Literal(String),
    Variable(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg(pub Vec<ArgPart>);

#[derive(Debug, Clone, PartialEq)]
pub enum CommandExpr {
    // Simple command: "echo hello"
    Simple {
        program: Arg,
        args: Vec<Arg>,
    },
    // Pipeline: "ls | grep target"
    Pipe {
        left: Box<CommandExpr>,
        right: Box<CommandExpr>,
    },
    // Redirection: "echo logs > file.txt"
    Redirect {
        cmd: Box<CommandExpr>,
        target: Arg,
        mode: RedirectMode, // Create, Append, Input
    },
    // Logic AND: "cargo build && cargo run"
    And(Box<CommandExpr>, Box<CommandExpr>),
    // Logic OR: "cargo test || echo failed"
    Or(Box<CommandExpr>, Box<CommandExpr>),
    // Assignment: "A=10"
    Assignment {
        key: String,
        value: Arg,
    },
    // Subshell: "( cd /tmp; ls )"
    Subshell(Box<CommandExpr>),
    // If: "if true; then echo yes; else echo no; fi"
    If {
        cond: Box<CommandExpr>,
        then_branch: Box<CommandExpr>,
        else_branch: Option<Box<CommandExpr>>,
    },
    // While: "while true; do echo loop; done"
    While {
        cond: Box<CommandExpr>,
        body: Box<CommandExpr>,
    },
    // Sequence: "cmd1; cmd2"
    Sequence(Box<CommandExpr>, Box<CommandExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectMode {
    Overwrite, // >
    Append,    // >>
    Input,     // <
}
