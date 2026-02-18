# Configuration

Pavidi uses `p.toml` as its primary configuration file. This file sits at the root of your project and defines metadata, environment variables, and tasks.

## `p.toml` Structure

A typical `p.toml` file consists of three main sections: `[project]`, `[env]`, and `[runner]`.

```toml
[project]
name = "my-project"
version = "1.0.0"
shell = "bash" # Optional: Force a specific shell (defaults to system default)

[env]
PORT = "8080"
APP_ENV = "dev"

[runner]
# Task definitions go here
```

### Project Metadata (`[project]`)

*   `name`: The name of your project.
*   `version`: The current version of your project.
*   `authors`: (Optional) List of authors.
*   `description`: (Optional) Description of the project.
*   `shell`: (Optional) Override the default shell used to execute commands.
    *   Defaults: `sh` on Unix, `pwsh` or `cmd` on Windows.
*   `log_strategy`: (Optional) Control logging verbosity ("always", "error-only", "none").
*   `log_plain`: (Optional) Set to `true` to disable colored output.
*   `secret_patterns`: (Optional) List of regex patterns to redact from logs.

### Environment Variables (`[env]`)

You can define environment variables that will be available to all tasks executed by Pavidi.

```toml
[env]
DATABASE_URL = "postgres://localhost:5432/mydb"
API_KEY = "secret-123"
```

#### Dynamic Variables

Pavidi supports dynamic variable resolution using the `$(command)` syntax. The command inside the parentheses is executed, and its standard output is captured as the variable value.

```toml
[env]
# Capture the current git commit hash
GIT_HASH = "$(git rev-parse --short HEAD)"
# Capture the current date
BUILD_DATE = "$(date +%Y-%m-%d)"
```

### `.env` File Integration

Pavidi has first-class support for `.env` files.

1.  **Automatic Loading:** If a `.env` file exists in the directory where `p` is run, it is automatically loaded.
2.  **Precedence:** Variables defined in `.env` files **override** those defined in `p.toml`.
3.  **Environment Switching:** If the `P_ENV` environment variable is set (e.g., `P_ENV=prod`), Pavidi will attempt to load `.env.prod` instead of `.env`.

---

[**Next step: Task Runner**](task-runner.md)
