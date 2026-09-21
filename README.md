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

---
---
---

Dokumentacja architektury, publicznego API oraz specyfikacji języka wzorców (DSL) dla biblioteki `querypath`.
Architecture documentation, public API reference, and pattern DSL specification for the `querypath` crate.

---

## 🔍 Specyfikacja Wzorców i DSL / DSL & Pattern Specification

### 1. Standardowe modyfikatory dopasowań (Globbing & Wildcards)

Warstwa parsowania przekształcająca znaki tekstowe w reguły wyrażeń regularnych, wspierająca opcjonalną wrażliwość na wielkość liter.
Parsing layer transforming text patterns into regular expression rules, supporting optional case sensitivity.

| Wzorzec | Nazwa techniczna | Zachowanie silnika / Engine Behavior |
| :--- | :--- | :--- |
| `*` | Single-level Wildcard | <br>**[POL]:** Dopasowuje zero lub więcej znaków w obrębie jednego poziomu (nie dopasowuje `/`).<br>**[ENG]:** Matches zero or more characters within a single level (does not match `/`). |
| `**` | Multi-level Wildcard | <br>**[POL]:** Dopasowuje dowolną liczbę znaków łącznie z separatorami `/` (rekurencja wielopoziomowa).<br>**[ENG]:** Matches any number of characters including `/` separators (multi-level recursion). |
| `?` | Single Character | <br>**[POL]:** Dopasowuje dokładnie jeden dowolny znak, z wyłączeniem separatora `/`.<br>**[ENG]:** Matches exactly one arbitrary character, excluding the `/` separator. |
| `{a\|b}` | Brace Expansion | <br>**[POL]:** Rozwija wzorzec na oddzielne warianty logiczne przed kompilacją z użyciem separatora `\|`.<br>**[ENG]:** Expands the pattern into separate logical variants before compilation using `\|` separator. |
| `[a-z]` | Character Class | <br>**[POL]:** Dopasowuje jeden znak z podanego zakresu lub zbioru.<br>**[ENG]:** Matches one character from the specified range or set. |
| `\` | Escape Character | <br>**[POL]:** Traktuje następny znak dosłownie (np. `\.` dopasowuje kropkę).<br>**[ENG]:** Treats the next character literally (e.g. `\.` matches a dot). |

---

### 2. Prefiksy Specjalne i Kotwiczenie / Prefixes & Anchoring

Bariery logiczne analizujące surowy wzorzec na podstawie prefiksów wejściowych w celu precyzyjnego ustalenia docelowych obiektów.
Logic gates evaluating raw input patterns based on prefixes to pinpoint target filesystem objects.

| Prefiks / Wzorzec | Nazwa techniczna | Zasada działania / Operating Principle |
| :--- | :--- | :--- |
| `!` | Hard Veto | <br>**[POL]:** Umieszczony na początku wzorca. Dopasowanie negatywne bezwzględnie odrzuca ścieżkę.<br>**[ENG]:** Placed at the start of a pattern. A negative match unconditionally rejects the path. |
| `@` | Sibling Requirement | <br>**[POL]:** Umieszczony na początku wzorca (np. `@core`). Wymaga istnienia pary plik + katalog o tej samej nazwie rdzennej.<br>**[ENG]:** Placed at the start of a pattern (e.g. `@core`). Requires a file + directory pair of the same core name. |
| `$` | Orphan Requirement | <br>**[POL]:** Umieszczony na początku wzorca (np. `$core`). Przeciwieństwo `@`. Dopasowuje element tylko wtedy, gdy w środowisku brakuje odpowiadającej mu pary.<br>**[ENG]:** Placed at the start of a pattern (e.g. `$core`). Opposite of `@`. Matches an element only if its corresponding pair is missing. |
| `re:` | Direct Regex | <br>**[POL]:** Umieszczony na początku wzorca (np. `re:^src/.*\.rs$`). Przekazuje czyste wyrażenie regularne bezpośrednio do silnika.<br>**[ENG]:** Placed at the start of a pattern (e.g. `re:^src/.*\.rs$`). Passes raw regex straight to the engine. |
| `./...` | Root Anchor | <br>**[POL]:** Wymusza szukanie ścieżki dokładnie od korzenia skanowanego środowiska.<br>**[ENG]:** Enforces path searching exactly from the root of the scanned environment. |
| `.../` | Directory Target | <br>**[POL]:** Wzorzec kończący się ukośnikiem. Dopasowuje wyłącznie katalogi.<br>**[ENG]:** Pattern ending with a slash. Matches directories exclusively. |

---

### 3. Opcje Skanowania (PathsOptions) / Scanning Options

Zachowanie strukturalne drzewa jest kontrolowane na poziomie konfiguracji skanera (`PathsOptions`), a nie w samej składni wzorców.
Structural tree behavior is configured via scanner options (`PathsOptions`), independent of pattern syntax.

| Opcja | Metoda API | Opis / Description |
| :--- | :--- | :--- |
| `keep_parent` | `.keep_parent(bool)` | <br>**[POL]:** Zachowuje katalogi nadrzędne dla dopasowanych elementów, zapobiegając powstawaniu osieroconych węzłów w drzewie.<br>**[ENG]:** Retains parent directories for matched elements, preventing orphan nodes in the tree structure. |
| `ignore_case` | `.ignore_case(bool)` | <br>**[POL]:** Włącza ignorowanie wielkości liter we wszystkich skompilowanych wzorcach.<br>**[ENG]:** Enables case-insensitive matching across all compiled patterns. |

---

# modules 

* `fs_walk` - Skanowanie i surowe przechodzenie po systemie plików / File system traversal and scanning utilities.
* `paths_entry` - Kanonizacja ścieżek, wyliczanie relacji CWD i analiza kontekstu / Structures and functions for representing and manipulating individual paths within the scanned filesystem.
* `paths_options` - Flagi konfiguracyjne określające zachowanie skanera / Configuration options governing the behavior of path scanning and matching.
* `paths_patterns` - Kompilacja wzorców, rozwijanie klamer i bez-dyskowa weryfikacja relacji / Core logic for defining, parsing, and applying matching patterns (including custom DSL elements).
* `querypath` - Główny orkiestrator łączący skanowanie, filtrowanie i zbieranie wyników / The main orchestrator module, integrating scanning, matching, and result collection.

---

## module `paths_entry`

### structs

* `paths_entry::PathsEntry` - <br>**[POL]:** Menadżer punktów wejściowych rozwijający klamry ścieżek i weryfikujący ich istnienie na dysku. <br>**[ENG]:** Represents a specific entry (file or directory) discovered and validated during the path scanning process.
* `paths_entry::AnchoredPath` - <br>**[POL]:** Punkt zakotwiczenia ścieżki docelowej wyliczający relatywną ścieżkę z miejsca wywołania (CWD). <br>**[ENG]:** Represents a path bound to a specific root or anchor point, ensuring correct relative resolution.
* `paths_entry::PathContext` - <br>**[POL]:** Bezalokacyjna analiza relacji (rodzic/plik) na wycinkach tekstowych `&str`. <br>**[ENG]:** Holds contextual information about a path slice, such as its relationship to parent or sibling nodes.
* `paths_entry::PathNode` - <br>**[POL]:** Węzeł ścieżki przechowujący `PathBuf` oraz znormalizowaną postać tekstową `String`. <br>**[ENG]:** A building block representing a segment or node within a full path structure.

### functions

* `paths_entry::new` - <br>**[POL]:** Tworzy nową strukturę `PathNode` z automatyczną normalizacją. <br>**[ENG]:** Constructs a new instance of a path-related structure (`PathNode`). [Accepts: `PathBuf` | Returns: `PathNode`]
* `paths_entry::build` - <br>**[POL]:** Buduje zbiór zakotwiczonych celów skanowania z rozwinięciem klamer. <br>**[ENG]:** A builder pattern method for creating complex path entry structures. [Accepts: `IntoIterator<Item = S>` where `S: AsRef<str>` | Returns: `Result<PathsEntry>`]
* `paths_entry::name` - <br>**[POL]:** Wyciąga samą końcową nazwę obiektu z kontekstu ścieżki. <br>**[ENG]:** Extracts or retrieves the name component of the path context. [Returns: `&str`]
* `paths_entry::normalize_path` - <br>**[POL]:** Normalizuje ścieżkę tekstową (zamienia `\` na `/`, usuwa prefiksy UNC `\\?\` oraz `/./`). <br>**[ENG]:** Normalizes a given path string (e.g., resolving `.` and `..`, unifying slashes). [Accepts: `AsRef<Path>` | Returns: `String`]
* `paths_entry::resolve` - <br>**[POL]:** Wylicza relację relatywną pomiędzy katalogiem roboczym (CWD) a celem. <br>**[ENG]:** Resolves a target path relative to its anchor or current execution context. [Accepts: `AsRef<Path>`, `&PathNode` | Returns: `Result<AnchoredPath>`]
* `paths_entry::from_path` - <br>**[POL]:** Tworzy bezalokacyjny kontekst `PathContext` ze ścieżki tekstowej. <br>**[ENG]:** Creates a path context structure from a standard filesystem path slice. [Accepts: `&'a str` | Returns: `PathContext<'a>`]
* `paths_entry::parent` - <br>**[POL]:** Wyciąga wycinek ścieżki katalogu nadrzędnego. <br>**[ENG]:** Retrieves the parent path slice, if applicable. [Returns: `&str`]
* `paths_entry::is_root_level` - <br>**[POL]:** Sprawdza, czy dany obiekt znajduje się bezpośrednio w korzeniu skanowania. <br>**[ENG]:** Checks if the current path context represents the root level of the scan. [Returns: `bool`]

---

## module `fs_walk`

### structs

* `fs_walk::FsWalk` - <br>**[POL]:** Główna struktura wykonująca surowe przechodzenie po drzewie katalogów. <br>**[ENG]:** The primary structure responsible for executing the filesystem walk.
* `fs_walk::FsWalkStat` - <br>**[POL]:** Liczniki obiektów zarejestrowanych podczas skanowania dysku. <br>**[ENG]:** Contains statistics gathered during the filesystem walk operations.

### functions

* `fs_walk::scan` - <br>**[POL]:** Inicjuje proces surowego skanowania dla podanych celów wejściowych. <br>**[ENG]:** Initiates the scanning process over the target directories. [Accepts: `&PathsEntry` | Returns: `Result<FsWalk>`]

---

## module `paths_options`

### structs

* `paths_options::PathsOptions` - <br>**[POL]:** Obiekt konfiguracji przechowujący opcje wykonania zapytania. <br>**[ENG]:** A configuration object holding various options for the path querying process.

### functions

* `paths_options::new` - <br>**[POL]:** Tworzy nowy, domyślny zestaw opcji skanowania. <br>**[ENG]:** Creates a new, default set of path options. [Accepts: No arguments | Returns: `PathsOptions`]
* `paths_options::keep_parent` - <br>**[POL]:** Konfiguruje zachowywanie katalogów nadrzędnych dla dopasowanych wyników. <br>**[ENG]:** Configures the option to retain parent directories in the results even if they don't explicitly match the query. [Accepts: `bool` | Returns: `Self`]
* `paths_options::ignore_case` - <br>**[POL]:** Włącza/wyłącza ignorowanie wielkości liter we wzorcach. <br>**[ENG]:** Enables or disables case-insensitive matching for path queries. [Accepts: `bool` | Returns: `Self`]

---

## module `paths_patterns`

### trait

* `paths_patterns::PattEnvIndex` - <br>**[POL]:** Trait definiujący indeks pamięci operacyjnej do bez-dyskowej weryfikacji relacji `@` oraz `$`. <br>**[ENG]:** A trait defining the in-memory index required for pattern relation evaluation (`@`, `$`).

### structs

* `paths_patterns::PathsPatterns` - <br>**[POL]:** Reprezentuje w pełni skompilowany zbiór reguł Regex gotowy do dopasowywania. <br>**[ENG]:** Represents a fully compiled set of pattern rules ready for matching.
* `paths_patterns::PattRaw` - <br>**[POL]:** Wrapper na surowe, nieprzetworzone napisy wzorców użytkownika. <br>**[ENG]:** Represents the raw, unparsed pattern strings provided by the user.
* `paths_patterns::PattExp` - <br>**[POL]:** Wrapper na wzorce rozwinięte po operacji rozwijania klamer `{a|b}`. <br>**[ENG]:** Represents expanded pattern strings resulting from brace expansion (`{a|b}`).

### functions

* `paths_patterns::build` - <br>**[POL]:** Kompiluje i tworzy `PathsPatterns` z surowych napisów po rozwinięciu klamer `{a|b}`. <br>**[ENG]:** Compiles or builds a `PathsPatterns` instance from raw input patterns. [Accepts: `IntoIterator<Item = S>`, `ignore_case: bool` | Returns: `Result<PathsPatterns>`]
* `paths_patterns::expand_braces` - <br>**[POL]:** Wykonuje rekurencyjne rozwijanie klamer po separatorze `|` (np. `{src|tests}`). <br>**[ENG]:** Performs brace expansion using the `|` separator on a given pattern string. [Accepts: `&str` | Returns: `Vec<String>`]
* `paths_patterns::is_match` - <br>**[POL]:** Weryfikuje czy dana ścieżka spełnia skompilowane reguły (uwzględnia negację `!`, `@`, `$`, `re:`). <br>**[ENG]:** Evaluates whether a specific path matches the compiled patterns and relations. [Accepts: `&str`, `&E` where `E: PattEnvIndex` | Returns: `bool`]
* `paths_patterns::rules_count` - <br>**[POL]:** Zwraca liczbę aktywnych reguł wewnątrz skompilowanego wzorca. <br>**[ENG]:** Returns the number of individual compiled rules contained within the pattern set. [Returns: `usize`]

---

## module `querypath`

### structs

* `querypath::FileItem` - <br>**[POL]:** Reprezentuje dopasowany plik w wynikach zapytania (rozmiar, status binarny, data modyfikacji). <br>**[ENG]:** Represents a matched file within the query results.
* `querypath::DirItem` - <br>**[POL]:** Reprezentuje dopasowany katalog w wynikach z podwójną wagą (`real_size` vs `matched_size`). <br>**[ENG]:** Represents a matched directory within the query results.
* `querypath::QueryResults` - <br>**[POL]:** Końcowy raport z wykonania zapytania zawierający zebrane metryki czasu i zbiory obiektów. <br>**[ENG]:** The final collection of matched files, directories, and benchmarks returned after a query operation.
* `querypath::QueryPath` - <br>**[POL]:** Główny builder używany do konfigurowania i uruchamiania zapytania. <br>**[ENG]:** The main facade structure used to configure and execute a query path operation.

### functions

* `querypath::new` - <br>**[POL]:** Tworzy nową instancję buildera `QueryPath`. <br>**[ENG]:** Instantiates a new `QueryPath` builder. [Accepts: No arguments | Returns: `QueryPath`]
* `querypath::run` - <br>**[POL]:** Wykonuje skonfigurowane zapytanie i zwaraca zagregowany raport wyników. <br>**[ENG]:** Executes the configured query against the target filesystem. [Returns: `Result<QueryResults>`]
* `querypath::scan_at` - <br>**[POL]:** Ustawia katalogi startowe skanowania. <br>**[ENG]:** Sets the starting directories or root paths for the scan operation. [Accepts: `IntoIterator<Item = S>` | Returns: `Self`]
* `querypath::options` - <br>**[POL]:** Aplikuje gotową strukturę opcji `PathsOptions`. <br>**[ENG]:** Applies a specific set of `PathsOptions` to the query configuration. [Accepts: `PathsOptions` | Returns: `Self`]
* `querypath::match_pattern` - <br>**[POL]:** Rejestruje wzorce dopasowań (np. `"*.rs"`, `"!*test*"`). <br>**[ENG]:** Defines patterns that the query should use for matching paths. [Accepts: `IntoIterator<Item = S>` | Returns: `Self`]
* `querypath::keep_parent` - <br>**[POL]:** Wygodna metoda do włączania zachowywania katalogów nadrzędnych. <br>**[ENG]:** A convenience method on the builder to enable or disable retaining parent directories. [Accepts: `bool` | Returns: `Self`]
* `querypath::ignore_case` - <br>**[POL]:** Wygodna metoda do włączania ignorowania wielkości liter we wzorcach. <br>**[ENG]:** A convenience method on the builder to enable or disable case-insensitive matching. [Accepts: `bool` | Returns: `Self`]

---

🫟
