# Task Runner

The core of Pavidi is its task runner, which allows you to define and execute complex workflows.

## Defining Tasks

Tasks are defined in the `[runner]` section of `p.toml`.

### Simple Commands

For simple tasks that only run a single command, you can use a string or a list of strings:

```toml
[runner]
clean = "rm -rf target/"
format = ["cargo fmt", "prettier --write ."]
```

### Advanced Task Configuration

For more complex scenarios, use a table to define properties like dependencies, conditions, and timeouts.

```toml
[runner.deploy]
cmds = ["./deploy.sh"]
description = "Deploy to production"
timeout = 600 # Timeout in seconds
```

## Dependencies & Parallel Execution

Tasks can depend on other tasks. Pavidi ensures that dependencies run *before* the main task.

```toml
[runner.test]
cmds = ["cargo test"]
deps = ["build"] # Run 'build' before 'test'
```

### Parallel Execution

By default, dependencies run sequentially. You can enable parallel execution to speed up your workflow.

```toml
[runner.ci]
cmds = ["echo 'CI Finished'"]
deps = ["lint", "test", "audit"]
parallel = true # Run 'lint', 'test', and 'audit' simultaneously
```

## Conditional Logic

Pavidi allows you to control *when* a task runs using `run_if` and `skip_if`.

### `run_if`

Executes the task **only if** the provided command succeeds (exit code 0).

```toml
[runner.migrate]
cmds = ["./migrate_db.sh"]
run_if = "test -f db/migrations.sql" # Only migrate if migration file exists
```

### `skip_if`

Skips the task **if** the provided command succeeds (exit code 0).

```toml
[runner.setup]
cmds = ["./install_deps.sh"]
skip_if = "test -d node_modules" # Skip if node_modules already exists
```

### `ignore_failure`

If a command fails, Pavidi usually stops execution. Set `ignore_failure = true` to continue anyway.

```toml
[runner.flaky_task]
cmds = ["./sometimes_fails.sh"]
ignore_failure = true
```

## Cleanup (`finally`)

The `finally` block specifies commands that run **after** the main commands, regardless of success or failure. This is useful for cleanup.

```toml
[runner.integration_test]
cmds = ["./run_tests.sh"]
finally = ["./cleanup_db.sh"] # Always runs
```

## OS-Specific Overrides

Pavidi lets you define different commands for Windows, Linux, and macOS. This is essential for true cross-platform compatibility.

```toml
[runner.open_browser]
cmds = ["echo 'Opening browser...'"]
windows = ["start http://localhost:8080"]
linux = ["xdg-open http://localhost:8080"]
macos = ["open http://localhost:8080"]
```

---

[**Next step: Portable Commands**](portable-commands.md)
