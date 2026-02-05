# Pavidi (P)

> **PREVIEW STAGE**  
> This software is currently in preview. Features and APIs are subject to change. Use with caution.

Pavidi (or simply **P**) is a minimalist task runner and shell environment built in Rust. It aims to provide a consistent execution layer across different operating systems.

## Components

- **Pavidi (Core):** The project-aware task runner that manages configuration, dependencies, and execution flow.

## Installation

To build and install from source:

```sh
cargo install --path .
```

## Configuration (`p.toml`)

Project configuration is defined in a `p.toml` file at the root of your project.

```toml
[project]
name = "my-awesome-app"
version = "0.1.0"

[env]
RUST_LOG = "info"
app_port = "8080"

# Simple task definition
[runner]
clean = "rm -rf target/"
format = ["cargo fmt", "cargo clippy"]

# Full task definition with dependencies and metadata
[runner.build]
cmds = ["cargo build --release"]
description = "Builds the project for release"

[runner.test]
cmds = ["cargo test"]
deps = ["build"]
ignore_failure = false
```

## Usage

P uses short, mnemonic commands for efficiency.

- **Run a task:**
  ```sh
  p r <task_name>
  # Example: p r build
  ```

- **List available tasks:**
  ```sh
  p ls
  ```

- **Show project info:**
  ```sh
  p info
  ```

- **Clean artifacts:**
  ```sh
  p c
  ```
