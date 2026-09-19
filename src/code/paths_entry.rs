use anyhow::{Context, Result, bail};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// NORMALIZACJA ŚCIEŻEK
// ============================================================================

/// Normalizuje ścieżkę do ujednoliconego formatu tekstowego (POSIX-like).
/// - Zmienia `\` na `/`
/// - Usuwa Windowsowe prefiksy UNC (`\\?\`)
/// - Redukuje zduplikowane sekwencje `/./` do `/`
#[must_use]
pub fn normalize_path<P: AsRef<Path>>(p: P) -> String {
    p.as_ref()
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/")
        .replace("/./", "/")
}

// ============================================================================
// BAZOWE STRUKTURY DANYCH (DOMENA)
// ============================================================================

/// Reprezentacja pojedynczego węzła ścieżki przechowująca zarówno
/// oryginalny obiekt systemowy (`PathBuf`), jak i szybką, znormalizowaną
/// postać tekstową (`String`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathNode {
    pub buf: PathBuf,
    pub str: String,
}

impl PathNode {
    pub fn new(buf: PathBuf) -> Self {
        let str = normalize_path(&buf);
        Self { buf, str }
    }
}

/// ============================================================================
/// PATH CONTEXT (ZERO-COPY / STACK-BASED)
/// ============================================================================
/// Analiza relacji wewnątrz ścieżki tekstowej bez ani jednej alokacji na stercie.
/// Działa w 100% na pożyczonych wycinkach referencji (`&str`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathContext<'a> {
    pub parent: &'a str,
    pub file: &'a str,
}

impl<'a> PathContext<'a> {
    /// Dzieli ścieżkę na katalog nadrzędny i plik/katalog docelowy.
    #[must_use]
    pub fn from_path(path: &'a str) -> Self {
        let clean_path = path.trim_start_matches("./");

        let (parent, file) = match clean_path.rsplit_once('/') {
            Some((p, f)) => (p, f),
            std::option::Option::None => ("", clean_path),
        };

        Self { parent, file }
    }

    pub fn name(&self) -> &'a str {
        self.file
    }

    pub fn parent(&self) -> &'a str {
        self.parent
    }

    pub fn is_root_level(&self) -> bool {
        self.parent.is_empty()
    }
}

// ============================================================================
// RELACJE I ZAKOTWICZENIE (ANCHORING)
// ============================================================================

/// Odwzorowanie relacji pomiędzy miejscem odpalenia programu (CWD),
/// a fizycznym celem skanowania, wyliczające zawsze bezpieczną ścieżkę relatywną.
#[derive(Debug, Clone)]
pub struct AnchoredPath {
    pub execution_dir: PathNode, // Katalog, z którego wywołano aplikację (CWD)
    pub target_dir: PathNode,    // Fizyczna lokalizacja poddawana skanowaniu
    pub relative_path: String,   // Ścieżka pomostowa między nimi
}

impl AnchoredPath {
    /// Tworzy nowy punkt zakotwiczenia. Zwraca błąd, jeśli ścieżka docelowa
    /// nie istnieje na dysku (wczesna walidacja z kontekstem).
    pub fn resolve<P: AsRef<Path>>(target: P, cwd_node: &PathNode) -> Result<Self> {
        let target_ref = target.as_ref();

        // Kanonizacja rozwija m.in. symlinki, '..' i weryfikuje istnienie na dysku.
        let absolute_target = fs::canonicalize(target_ref).with_context(|| {
            format!(
                "Ścieżka docelowa nie istnieje lub jest niedostępna: {:?}",
                target_ref
            )
        })?;

        let target_node = PathNode::new(absolute_target);
        let is_dir = target_node.buf.is_dir();

        // Wyliczamy relację (jak dostać się z CWD do Targetu).
        let relative_path = match target_node.buf.strip_prefix(&cwd_node.buf) {
            Ok(rel) => {
                let clean_rel = normalize_path(rel).trim_matches('/').to_string();
                if clean_rel.is_empty() {
                    "./".to_string()
                } else if is_dir {
                    format!("./{clean_rel}/")
                } else {
                    format!("./{clean_rel}")
                }
            }
            Err(_) => {
                let mut s = normalize_path(&target_node.buf);
                if is_dir && !s.ends_with('/') {
                    s.push('/');
                }
                s
            }
        };

        Ok(Self {
            execution_dir: cwd_node.clone(),
            target_dir: target_node,
            relative_path,
        })
    }
}

// ============================================================================
// GŁÓWNY PUNKT WEJŚCIA I ORKIESTRACJA WEJŚĆ (ENTRY MANAGER)
// ============================================================================

/// Menadżer zarządzający wszystkimi lokalizacjami startowymi skanowania.
/// Rozwija alternatywy `{a|b}` oraz weryfikuje ich istnienie.
#[derive(Debug, Clone)]
pub struct PathsEntry {
    pub execution_dir: PathNode,
    pub targets: Vec<AnchoredPath>,
}

impl PathsEntry {
    /// Inicjuje listę celów skanowania na podstawie podanych przez użytkownika stringów.
    /// Jeśli lista jest pusta lub zawiera puste napisy (`""`), domyślnie używa katalogu roboczego (`./`).
    pub fn build<I, S>(inputs: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        // 1. Ustalenie kanonicznego CWD (również przepuszczonego przez fs::canonicalize)
        let raw_cwd = env::current_dir().context("Nie można odczytać katalogu roboczego (CWD)")?;
        let cwd_buf = fs::canonicalize(&raw_cwd).unwrap_or(raw_cwd);
        let cwd_node = PathNode::new(cwd_buf);

        let mut raw_inputs: Vec<String> = Vec::new();
        for input in inputs {
            let s = input.as_ref().trim();
            if s.is_empty() {
                raw_inputs.push("./".to_string());
            } else {
                raw_inputs.push(s.to_string());
            }
        }

        // 2. Jeśli po przefiltrowaniu lista jest pusta - domyślnie badamy CWD
        if raw_inputs.is_empty() {
            raw_inputs.push("./".to_string());
        }

        let mut expanded_paths = Vec::new();
        for input in raw_inputs {
            expanded_paths.extend(Self::expand_alternatives(&input));
        }

        // 3. Budowanie punktów zakotwiczenia.
        let mut targets = Vec::new();
        for expanded in expanded_paths {
            #[allow(clippy::collapsible_if)]
            if let Ok(anchored) = AnchoredPath::resolve(&expanded, &cwd_node) {
                if !targets
                    .iter()
                    .any(|t: &AnchoredPath| t.target_dir == anchored.target_dir)
                {
                    targets.push(anchored);
                }
            }
        }

        if targets.is_empty() {
            bail!("Żadna z podanych lub rozwiniętych ścieżek wejściowych nie istnieje na dysku.");
        }

        Ok(Self {
            execution_dir: cwd_node,
            targets,
        })
    }

    /// Rekurencyjnie rozwija alternatywne bloki zapisane w klamrach,
    /// np.: `./{abc/ao/|def/}` -> `./abc/ao/` oraz `./def/`
    fn expand_alternatives(input: &str) -> Vec<String> {
        let Some(end) = input.find('}') else {
            return vec![input.to_string()];
        };

        let Some(sub) = input.get(..end) else {
            return vec![input.to_string()];
        };

        let Some(start) = sub.rfind('{') else {
            return vec![input.to_string()];
        };

        let (Some(prefix), Some(suffix), Some(options)) = (
            input.get(..start),
            input.get(end + 1..),
            input.get(start + 1..end),
        ) else {
            return vec![input.to_string()];
        };

        let mut expanded = Vec::new();

        for opt in options.split('|') {
            let merged = format!("{prefix}{opt}{suffix}");
            expanded.extend(Self::expand_alternatives(&merged));
        }

        expanded
    }
}
