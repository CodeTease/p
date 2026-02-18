# Portable Commands

One of the biggest challenges in cross-platform development is inconsistent shell commands. Pavidi solves this with **Portable Commands**—built-in utilities that work identically on Windows, Linux, and macOS.

These commands are prefixed with `p:` and do not require external tools like Git Bash or Coreutils on Windows.

## Available Commands

### `p:rm` (Remove)
Removes files or directories.

*   **Syntax:** `p:rm [flags] <paths...>`
*   **Flags:**
    *   `-r`, `-R`, `--recursive`: Recursively remove directories.
    *   `-f`, `--force`: Ignore nonexistent files and arguments.

```toml
clean = "p:rm -rf target/ dist/"
```

### `p:cp` (Copy)
Copies files or directories.

*   **Syntax:** `p:cp [flags] <source> <destination>`
*   **Flags:**
    *   `-r`, `-R`, `--recursive`: Recursively copy directories.

```toml
backup = "p:cp -r src/ src_backup/"
```

### `p:mkdir` (Make Directory)
Creates directories.

*   **Syntax:** `p:mkdir <paths...>`
*   **Behavior:** Implicitly creates parent directories (like `mkdir -p`).

```toml
setup = "p:mkdir -p build/logs"
```

### `p:mv` (Move/Rename)
Moves or renames files/directories.

*   **Syntax:** `p:mv <source> <destination>`

```toml
rename = "p:mv output.txt final_output.txt"
```

### `p:ls` (List)
Lists directory contents.

*   **Syntax:** `p:ls <paths...>`

```toml
check = "p:ls dist/"
```

### `p:cat` (Concatenate)
Reads files and prints to standard output.

*   **Syntax:** `p:cat <files...>`

```toml
show_config = "p:cat p.toml"
```

## Why Use Portable Commands?

1.  **Consistency:** No more `rm -rf` failing on Windows Command Prompt or `del` failing on Linux.
2.  **No Dependencies:** Users don't need to install extra tools.
3.  **Speed:** Built directly into Pavidi (Rust), they are faster than spawning external shell processes.

---

[**Next step: Smart Caching**](smart-caching.md)
