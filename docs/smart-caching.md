# Smart Caching

Pavidi includes a built-in caching mechanism to speed up your builds and tests. If a task's inputs and outputs haven't changed, Pavidi can skip execution.

## How it Works

When you define `sources` and `outputs` for a task, Pavidi:

1.  **Calculates a Hash:**
    *   Computes a cryptographic hash (BLAKE3) of the contents of all files matching the `sources` patterns.
    *   Includes the values of all environment variables defined in `[env]`.
    *   Includes the command string itself.

2.  **Checks Consistency:**
    *   Verifies if all files specified in `outputs` exist on disk.

3.  **Compares:**
    *   If the calculated hash matches the stored hash from the previous run **AND** all output files exist, the task is considered "up-to-date".
    *   Pavidi skips execution and logs that the task was cached.

## Configuration

To enable caching for a task, define `sources` and `outputs` in your `p.toml`.

```toml
[runner.build]
cmds = ["cargo build --release"]
# Files to watch for changes
sources = ["src/**/*.rs", "Cargo.toml", "Cargo.lock"]
# Files created by the command
outputs = ["target/release/pavidi"]
```

### Glob Patterns

Pavidi supports standard glob patterns for `sources`:

*   `*`: Matches any sequence of characters (except path separators).
*   `**`: Matches directories recursively.
*   `?`: Matches any single character.

## Benefits for CI/CD

Smart caching is particularly powerful in Continuous Integration (CI) environments.

1.  **Faster Builds:** Avoid rebuilding artifacts that haven't changed.
2.  **Resource Efficiency:** Save CPU cycles and reduce billable minutes on CI providers.
3.  **Consistency:** Ensure that tasks run only when necessary, reducing the chance of flaky tests due to stale artifacts.

---

[**Next step: Advanced Features**](advanced.md)
