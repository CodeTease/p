# Pavidi (P) - The Minimalist, Powerful Task Runner

**Pavidi** (executable as `p`) is a modern, cross-platform task runner built in Rust. It aims to provide a consistent execution layer across different operating systems, handling dependencies, environment variables, and parallel execution with ease.

[**📚 Read the Full Documentation**](docs/index.md)

## 🚀 Key Features

- **Cross-Platform Compatibility**: Write tasks once, run anywhere (Windows, Linux, macOS).
- **Portable Commands**: Built-in cross-platform commands like `p:rm`, `p:cp`, `p:mkdir` ensuring your scripts work on any OS.
- **Dependency Management**: Define task dependencies and execute them sequentially or in parallel.
- **Smart Caching**: Skip tasks if inputs haven't changed (hashing based on source files and environment variables).
- **Environment Management**: First-class support for `.env` files, dynamic variable resolution, and rigorous environment provenance tracking.
- **Secret Redaction**: Automatically mask sensitive information in logs.
- **Configuration Hierarchy**: Modular configuration with `p.toml` and extension files (`p.*.toml`).
- **Parallel Execution**: Leverage multi-core processors for independent tasks.

## 📦 Installation

To build and install from source:

```bash
cargo install --path .
```

Ensure `~/.cargo/bin` is in your `PATH`.

## ⚡ Quick Start

Create a `p.toml` file in your project root:

```toml
[project]
name = "my-awesome-project"
version = "0.1.0"

[env]
RUST_LOG = "info"
PORT = "8080"

[runner.build]
cmds = ["cargo build --release"]
description = "Build the project"

[runner.test]
cmds = ["cargo test"]
deps = ["build"]
ignore_failure = false

[runner.clean]
cmds = ["p:rm -rf target/"]
description = "Clean build artifacts"
```

Run a task:
```bash
p build
```

## 📖 Configuration Reference (`p.toml`)

Pavidi uses `p.toml` as its primary configuration file.

### Project Metadata

Define project-wide settings under `[project]`.

```toml
[project]
name = "my-project"
version = "1.0.0"
authors = ["Alice <alice@example.com>"]
description = "A sample project"
shell = "bash" # Optional: Force a specific shell (defaults to system default)
log_strategy = "always" # Options: "always", "error-only", "none"
log_plain = false # Disable colored logs if true
secret_patterns = ["API_KEY_.*"] # Regex patterns to redact in logs
```

### Environment Variables (`[env]`)

Define environment variables that are available to all tasks.

```toml
[env]
DATABASE_URL = "postgres://localhost:5432/mydb"
API_KEY = "secret-123"
# Dynamic variables (executed at runtime)
GIT_HASH = "$(git rev-parse --short HEAD)"
```

- **.env Files**: Pavidi automatically loads `.env` files. If `P_ENV` is set (e.g., `P_ENV=prod`), it looks for `.env.prod`.
- **Precedence**: `.env` files override `p.toml` variables.

### Task Definitions (`[runner]`)

Tasks are defined in the `[runner]` section.

#### Simple Command
```toml
[runner]
lint = "cargo clippy"
format = ["cargo fmt", "prettier --write ."]
```

#### Full Task Configuration
For more control, use a table:

```toml
[runner.deploy]
cmds = ["./deploy.sh"]
deps = ["build", "test"] # Tasks to run before this one
parallel = false # Run dependencies in parallel? (default: false)
description = "Deploy the application"

# Conditional Execution
run_if = "test -f dist/app.bin" # Run only if command succeeds (exit code 0)
skip_if = "git diff --quiet"    # Skip if command succeeds

# Smart Caching (Skip if inputs/outputs are up-to-date)
sources = ["src/**/*.rs", "Cargo.toml"]
outputs = ["target/release/app"]

# OS-Specific Overrides
windows = ["powershell ./deploy.ps1"]
linux = ["./deploy.sh"]
macos = ["./deploy.sh"]

# Error Handling
ignore_failure = false # Fail if command fails? (default: false)
retry = 3             # Number of retries
retry_delay = 5       # Seconds between retries
timeout = 600         # Timeout in seconds

# Cleanup
finally = ["p:rm tmp_file"] # Always runs after task (even on failure)
```

## 🛠 Portable Commands

Pavidi includes built-in commands to ensure cross-platform compatibility without relying on system shells.

- `p:rm [files/dirs...]`: Remove files or directories (supports `-r` for recursive, `-f` for force).
- `p:cp [src] [dest]`: Copy files or directories (supports `-r` for recursive).
- `p:mkdir [dirs...]`: Create directories (supports `-p` implicitly).
- `p:ls [dirs...]`: List files.
- `p:mv [src] [dest]`: Move/Rename files.
- `p:cat [files...]`: Concatenate and print files.

Example:
```toml
clean = "p:rm -rf target/ dist/"
```

## 💻 CLI Usage

```bash
p [TASK] [ARGS...]
```

- **Run a task**: `p build`
- **Pass arguments**: `p run -- --port 9000` (arguments after `--` are passed to the task)
- **List tasks**: `p -l` or `p --list`
- **Show Info**: `p -i` or `p --info` (shows loaded config and extensions)
- **Inspect Env**: `p --env` (shows resolved environment variables)
- **Trace Env**: `p -e --trace` (shows where each variable came from)
- **Dry Run**: `p --dry-run` (print commands without executing)

## 🧩 Advanced Features

### Configuration Extensions
You can split configuration into multiple files using the naming convention `p.*.toml`. These are loaded alphabetically and merged into the main configuration. This is useful for:
- User-specific overrides (`p.local.toml` - typically gitignored).
- Modular configurations for large projects.

### Smart Caching
Pavidi computes a BLAKE3 hash of the files matched by `sources` and the environment variables. It compares this against a stored hash. If the hash matches AND the files in `outputs` exist, the task is skipped.

