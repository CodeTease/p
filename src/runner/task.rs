use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum RunnerTask {
    /// Simple string command
    Single(String),
    /// List of sequential commands
    List(Vec<String>),
    /// Full configuration with dependencies and caching
    Full {
        #[serde(default)]
        cmds: Vec<String>,
        #[serde(default)]
        deps: Vec<String>,
        #[serde(default)]
        parallel: bool,
        // Description for listing
        #[serde(default)]
        description: Option<String>,
        
        // Conditional Execution
        run_if: Option<String>,
        skip_if: Option<String>,
        sources: Option<Vec<String>>,
        outputs: Option<Vec<String>>,

        // OS-specific commands
        windows: Option<Vec<String>>,
        linux: Option<Vec<String>>,
        macos: Option<Vec<String>>,

        // Error Handling
        #[serde(default)]
        ignore_failure: bool,

        // Timeout (seconds)
        #[serde(default)]
        timeout: Option<u64>,
    },
}
