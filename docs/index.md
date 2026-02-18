# Introduction to Pavidi

**Pavidi** (executable as `p`) is a modern, minimalist, cross-platform task runner built in Rust. It is designed to provide a consistent execution layer across different operating systems, handling dependencies, environment variables, and parallel execution with ease.

## Philosophy: Write Once, Run Anywhere

The core philosophy of Pavidi is to eliminate the friction of maintaining different scripts for Windows, Linux, and macOS. By using Pavidi's configuration and portable commands, you can define your build, test, and deployment pipelines once, and they will work seamlessly on any developer's machine or CI/CD environment.

## Table of Contents

1.  [**Getting Started**](getting-started.md)
    *   Installation (Script, Homebrew, Cargo)
    *   Hello World
2.  [**Configuration**](configuration.md)
    *   `p.toml` Structure
    *   Environment Variables & `.env` Files
    *   Dynamic Variables
3.  [**Task Runner**](task-runner.md)
    *   Task Definitions
    *   Dependencies & Parallel Execution
    *   Conditional Logic (`run_if`, `skip_if`)
    *   OS-Specific Overrides
4.  [**Portable Commands**](portable-commands.md)
    *   `p:rm`, `p:cp`, `p:mkdir`, etc.
    *   Why use them?
5.  [**Smart Caching**](smart-caching.md)
    *   Hashing Mechanism
    *   `sources` and `outputs`
    *   CI/CD Benefits
6.  [**Advanced Topics**](advanced.md)
    *   Modular Configuration (Extensions)
    *   Logging & Debugging
    *   Secret Redaction

---

[**Next step: Getting Started**](getting-started.md)
