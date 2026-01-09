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
        sources: Option<Vec<String>>,
        outputs: Option<Vec<String>>,
    },
}
