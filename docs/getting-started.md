# Getting Started with Pavidi

## Installation

Pavidi is designed to be easy to install on any platform. Choose the method that works best for you.

### Option 1: Automated Installer (Recommended)

You can use the automated installer script provided by `cargo-dist`.

**Linux & macOS:**

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/CodeTease/p/releases/latest/download/pavidi-installer.sh | sh
```

**Windows (PowerShell):**

```powershell
irm https://github.com/CodeTease/p/releases/latest/download/pavidi-installer.ps1 | iex
```

### Option 2: Homebrew (macOS & Linux)

If you use Homebrew, you can install Pavidi from our custom tap:

```bash
brew install CodeTease/tap/pavidi
```

### Option 3: Build from Source (Cargo)

If you have Rust installed, you can build Pavidi from source:

```bash
# Install from git
cargo install --git https://github.com/CodeTease/p

# Or if you have cloned the repository locally:
cargo install --path .
```

Ensure that `~/.cargo/bin` is in your system's `PATH`.

---

## Hello World

Let's create your first task. Pavidi uses a file named `p.toml` in your project root to define configuration and tasks.

1.  **Create a `p.toml` file:**

    ```toml
    [project]
    name = "hello-pavidi"
    version = "0.1.0"

    [runner.hello]
    cmds = ["echo 'Hello, Pavidi!'"]
    description = "Prints a greeting"
    ```

2.  **Run the task:**

    Open your terminal and run:

    ```bash
    p hello
    ```

    You should see output similar to:

    ```text
    Hello, Pavidi!
    ```

3.  **List available tasks:**

    Run `p --list` (or `p -l`) to see all tasks defined in your project:

    ```bash
    p --list
    ```

---

[**Next step: Configuration**](configuration.md)
