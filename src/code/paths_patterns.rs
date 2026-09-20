use anyhow::{Context, Result};
use regex::Regex;

// ============================================================================
// ENV ABSTRAKCJA (BEZ-DYSKOWE SPRAWDZANIE RELACJI)
// ============================================================================

/// Trait umożliwiający bez-dyskowe (zero-I/O) sprawdzanie obecności
/// plików i katalogów w pamięci podczas weryfikacji relacji `@` oraz `$`.
pub trait PattEnvIndex {
    fn has_dir(&self, dir: &str) -> bool;
    fn has_file_with_prefix(&self, prefix: &str) -> bool;
    fn any_file_in_dir(&self, dir: &str, check: &mut dyn FnMut(&str) -> bool) -> bool;
}

// ============================================================================
// TYPY DOMENOWE
// ============================================================================

/// Silnie typowana lista surowych wzorców wejściowych.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PattRaw(pub Vec<String>);

/// Silnie typowana lista wzorców rozwiniętych po klamrach `{a|b}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PattExp(pub Vec<String>);

/// Główna struktura zarządzająca dopasowywaniem wzorców do ścieżek.
#[derive(Debug, Clone)]
pub struct PathsPatterns {
    pub raw: PattRaw,
    pub expanded: PattExp,
    compiled: Vec<PatternCompiled>,
}

// ============================================================================
// SKOMPILOWANA REGUŁA (PATTERN COMPILED)
// ============================================================================

#[derive(Debug, Clone)]
struct PatternCompiled {
    regex: Regex,
    targets_dir_only: bool,
    requires_sibling: bool,
    requires_orphan: bool,
    base_name: String,
    pub is_negated: bool,
}

// ============================================================================
// IMPLEMENTACJA PATHS PATTERNS
// ============================================================================

impl PathsPatterns {
    /// Tworzy i kompiluje zbiór wzorców.
    /// Rozwija klamry `{a|b}` oraz przelicza reguły na wyrażenia regularne.
    pub fn build<I, S>(patterns: I, ignore_case: bool) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut raw_vec = Vec::new();
        let mut expanded_vec = Vec::new();

        for p in patterns {
            let s: &str = p.as_ref();
            if s.trim().is_empty() {
                continue;
            }
            raw_vec.push(s.to_string());
            expanded_vec.extend(Self::expand_braces(s));
        }

        let mut compiled = Vec::with_capacity(expanded_vec.len());
        for p in &expanded_vec {
            let rule = PatternCompiled::compile(p, ignore_case)
                .with_context(|| format!("Błąd kompilacji wzorca: '{p}'"))?;
            compiled.push(rule);
        }

        Ok(Self {
            raw: PattRaw(raw_vec),
            expanded: PattExp(expanded_vec),
            compiled,
        })
    }

    /// Sprawdza, czy podana ścieżka spełnia zasady narzucone przez zbiór wzorców.
    /// Wzorce zanegowane (`!`) działają jako bezwarunkowe weto.
    #[must_use]
    pub fn is_match<E: PattEnvIndex>(&self, path: &str, env: &E) -> bool {
        if self.compiled.is_empty() {
            return true;
        }

        let mut has_positive: bool = false;
        let mut matched_positive: bool = false;

        for c in &self.compiled {
            if c.is_negated {
                if c.is_match(path, env) {
                    return false;
                }
            } else {
                has_positive = true;
                if !matched_positive && c.is_match(path, env) {
                    matched_positive = true;
                }
            }
        }

        if has_positive { matched_positive } else { true }
    }

    #[must_use]
    pub fn rules_count(&self) -> usize {
        self.compiled.len()
    }

    /// Rekurencyjnie rozwija alternatywy zaimplementowane w klamrach `{a|b}`.
    pub fn expand_braces(input: &str) -> Vec<String> {
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

        let mut result = Vec::new();
        for opt in options.split('|') {
            let merged = format!("{prefix}{opt}{suffix}");
            result.extend(Self::expand_braces(&merged));
        }
        result
    }
}

// ============================================================================
// KOMPILACJA I DOPASOWANIE REGUŁY
// ============================================================================

impl PatternCompiled {
    pub fn compile(pattern: &str, ignore_case: bool) -> Result<Self, regex::Error> {
        let is_negated: bool = pattern.starts_with('!');
        let mut p: &str = if is_negated {
            pattern.get(1..).unwrap_or("")
        } else {
            pattern
        };

        let mut requires_sibling: bool = false;
        let mut requires_orphan: bool = false;

        if p.starts_with('@') {
            requires_sibling = true;
            p = p.get(1..).unwrap_or("");
        } else if p.starts_with('$') {
            requires_orphan = true;
            p = p.get(1..).unwrap_or("");
        }

        let is_direct_regex: bool = p.starts_with("re:");
        if is_direct_regex {
            let raw_re: &str = p.get(3..).unwrap_or("");
            let re_str: String = if ignore_case {
                format!("(?i){raw_re}")
            } else {
                raw_re.to_string()
            };
            let regex = Regex::new(&re_str)?;
            return Ok(Self {
                regex,
                targets_dir_only: pattern.ends_with('/'),
                requires_sibling,
                requires_orphan,
                base_name: String::new(),
                is_negated,
            });
        }

        // Tylko jawnny slash '/' na końcu oznacza wzorzec wyłącznie dla katalogów
        let targets_dir_only: bool = p.ends_with('/');

        let base_name: String = p
            .trim_end_matches('/')
            .split('/')
            .next_back()
            .unwrap_or("")
            .split('.')
            .next()
            .unwrap_or("")
            .to_string();

        let mut re = String::new();
        if ignore_case {
            re.push_str("(?i)");
        }

        let mut anchored: bool = false;

        if p.starts_with("./") {
            anchored = true;
            p = p.get(2..).unwrap_or("");
        } else if p.starts_with("**/") {
            // Wzorzec **/ oznacza wyszukiwanie od dowolnego poziomu
            anchored = false;
            p = p.get(3..).unwrap_or("");
        }

        if anchored {
            re.push('^');
        } else {
            re.push_str("(?:^|/)");
        }

        let chars: Vec<char> = p.chars().collect();
        let mut i: usize = 0;

        while i < chars.len() {
            match chars[i] {
                '\\' => {
                    i += 1;
                    if i < chars.len() {
                        re.push_str(&regex::escape(&chars[i].to_string()));
                    }
                }
                '.' => re.push_str("\\."),
                '/' => re.push('/'),
                '*' => {
                    if i + 1 < chars.len() && chars[i + 1] == '*' {
                        // ** dopasowuje dowolny ciąg znaków (włącznie z /)
                        re.push_str(".*");
                        i += 1;
                    } else {
                        re.push_str("[^/]*");
                    }
                }
                '?' => re.push_str("[^/]"),
                '[' => {
                    re.push('[');
                    if i + 1 < chars.len() && chars[i + 1] == '!' {
                        re.push('^');
                        i += 1;
                    }
                }
                ']' | '-' | '^' => re.push(chars[i]),
                c => re.push_str(&regex::escape(&c.to_string())),
            }
            i += 1;
        }

        re.push('$');

        Ok(Self {
            regex: Regex::new(&re)?,
            targets_dir_only,
            requires_sibling,
            requires_orphan,
            base_name,
            is_negated,
        })
    }

    #[allow(clippy::bool_comparison)]
    pub fn is_match<E: PattEnvIndex>(&self, path: &str, env: &E) -> bool {
        let is_dir: bool = path.ends_with('/');
        let clean: &str = path.strip_prefix("./").unwrap_or(path);

        if self.targets_dir_only && is_dir == false {
            return false;
        }

        if self.regex.is_match(clean) == false {
            return false;
        }

        // Relacje rodzeństwa/sieroty dla plików
        if (self.requires_sibling || self.requires_orphan) && is_dir == false {
            let parent: &str = match clean.rsplit_once('/') {
                Some((p, _)) => p,
                std::option::Option::None => "",
            };

            let mut expected_folder = String::new();
            if !parent.is_empty() {
                expected_folder.push_str(parent);
                expected_folder.push('/');
            }
            expected_folder.push_str(&self.base_name);
            expected_folder.push('/');

            let exists: bool = env.has_dir(&expected_folder);

            if self.requires_sibling && exists == false {
                return false;
            }
            if self.requires_orphan && exists {
                return false;
            }
        }

        // Relacje rodzeństwa/sieroty dla katalogów
        if (self.requires_sibling || self.requires_orphan) && is_dir {
            let dir_no_slash: &str = path.trim_end_matches('/');
            let mut search_prefix = String::from(dir_no_slash);
            search_prefix.push('.');

            let has_file_sibling: bool = env.has_file_with_prefix(&search_prefix);

            if self.requires_sibling && has_file_sibling == false {
                return false;
            }
            if self.requires_orphan && has_file_sibling {
                return false;
            }
        }

        true
    }
}
