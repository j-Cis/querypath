# querypath

[![Crates.io](https://img.shields.io/crates/v/querypath.svg)](https://crates.io/crates/querypath)
[![Docs.rs](https://docs.rs/querypath/badge.svg)](https://docs.rs/querypath)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Unlicense-blue.svg)](LICENSE)

A fast, modular directory query engine for Rust featuring advanced glob matching, exclusions, path normalization, and rich file metadata.

---

## Features

- 🔍 **Advanced Pattern Matching**: Full glob support with pattern negations (`!`), brace expansion (`{a|b}`), and wildcard safeguards.
- ⚡ **High Performance Engine**: Fast directory traversal built on `walkdir` with in-memory target indexing.
- 📊 **Rich Metadata**: Retrieves physical sizes, modification timestamps, and automated binary file detection via null-byte checking.
- 📁 **Dual-Size Directory Aggregation**: Evaluates both total physical directory size (`real_size`) and pattern-matched content size (`matched_size`).
- ⏱️ **Execution Metrics**: Accurate tracking of query lifecycle timestamps (`started_at_ms`, `finished_at_ms`) and duration in milliseconds.
- 🛠️ **Cross-Platform Normalization**: Automatic handling of UNC/POSIX paths and root resolution.

---

## Installation

Add `querypath` to your `Cargo.toml`:

```toml
[dependencies]
querypath = "1.0.0"

```

Or run:

```bash
cargo add querypath

```

---

## Quickstart

```rust
use anyhow::Result;
use querypath::QueryPath;

fn main() -> Result<()> {
    let results = QueryPath::new()
        .scan_at(["./"])
        .match_pattern(["*.rs", "!**/{tests|.git|target}/?**"])
        .keep_parent(true)
        .ignore_case(true)
        .run()?;

    println!("Scan finished in {} ms", results.duration_ms);
    println!("Scanned {} files, {} dirs", results.scanned_files, results.scanned_dirs);

    for dir in &results.dirs {
        println!("📁 {} (matched: {} B, total: {} B)", dir.path, dir.matched_size, dir.real_size);
    }

    for file in &results.files {
        println!("📄 {} (size: {} B, binary: {})", file.path, file.size, file.is_binary);
    }

    Ok(())
}

```

---

## Core API & Data Structures

### `QueryPath` Builder

| Method | Type Signature | Description |
| --- | --- | --- |
| `new()` | `() -> Self` | Initializes a default query instance. |
| `scan_at(paths)` | `(IntoIterator<Item = S>) -> Self` | Sets target directories or root entry paths. |
| `match_pattern(patterns)` | `(IntoIterator<Item = S>) -> Self` | Defines glob matching engine rules and negations. |
| `keep_parent(bool)` | `(bool) -> Self` | Retains structural parent directories of matched files. |
| `ignore_case(bool)` | `(bool) -> Self` | Toggles case insensitivity for glob patterns. |
| `run()` | `() -> Result<QueryResults>` | Executes the directory scan and returns structured metrics. |

### `QueryResults` Structure

```rust
pub struct QueryResults {
    pub execution_dir: String,
    pub scanned_paths: Vec<String>,
    pub patterns: Vec<String>,
    pub scanned_files: usize,
    pub scanned_dirs: usize,
    pub started_at_ms: u64,
    pub finished_at_ms: u64,
    pub duration_ms: u64,
    pub files: Vec<FileItem>,
    pub dirs: Vec<DirItem>,
}

```
