# Advanced Features

Unlock the full potential of Pavidi with these advanced configuration and debugging features.

## Modular Configuration (Extensions)

For large projects or user-specific overrides, you can split your configuration into multiple files. Pavidi automatically loads files matching the pattern `p.*.toml` in the project root.

### Example Workflow

1.  **`p.toml` (Base Config):** Contains the shared project configuration.
2.  **`p.local.toml` (User Overrides):** Contains developer-specific settings (e.g., local database URL). Add this file to `.gitignore`.
3.  **`p.ci.toml` (CI/CD Config):** Contains settings specific to the CI environment.

### Merging Rules

*   **Extensions are loaded alphabetically.**
*   **Deep Merge:** `[env]` and `[runner]` sections are merged.
*   **Overrides:** Values in later files override those in earlier files.

## Logging & Debugging

When things go wrong, Pavidi provides tools to help you understand what's happening.

### Trace Mode (`--trace`)

Use the `--trace` flag to see detailed execution logs, including environment variable resolution history.

```bash
p build --trace
```

### Environment Inspection (`--env`)

To see the final resolved environment variables available to tasks:

```bash
p --env
```

To see *where* each variable came from (e.g., system, `p.toml`, `.env`), combine with `--trace`:

```bash
p -e --trace
```

### Dry Run (`--dry-run`)

Preview the commands that would be executed without actually running them:

```bash
p build --dry-run
```

## Secret Redaction

Pavidi automatically attempts to redact sensitive information from logs. You can configure custom patterns in `p.toml`.

```toml
[project]
secret_patterns = ["API_KEY_.*", "PASSWORD_.*"]
```

Any output matching these regex patterns will be replaced with `[REDACTED]` in the console and log files.

---

[**Back to Introduction**](index.md)
