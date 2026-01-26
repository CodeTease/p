use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use crate::pas::commands::Executable;

#[derive(Clone)]
pub struct ShellContext {
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
    pub exit_code: i32,
    pub registry: Arc<HashMap<String, Box<dyn Executable + Send + Sync>>>,
}

impl ShellContext {
    pub fn new() -> Self {
        let env: HashMap<String, String> = std::env::vars().collect();
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut ctx = Self {
            cwd,
            env,
            exit_code: 0,
            registry: Arc::new(HashMap::new()),
        };
        crate::pas::commands::builtins::register_all_builtins(&mut ctx);
        ctx
    }

    pub fn register_command(&mut self, name: &str, command: Box<dyn Executable + Send + Sync>) {
        if let Some(map) = Arc::get_mut(&mut self.registry) {
            map.insert(name.to_string(), command);
        } else {
            // This should not happen during initialization phase
            panic!("Cannot register command: Registry is shared");
        }
    }

    pub fn clone_for_parallel(&self) -> Self {
        Self {
            cwd: self.cwd.clone(),
            env: self.env.clone(),
            exit_code: self.exit_code,
            registry: self.registry.clone(),
        }
    }
}
