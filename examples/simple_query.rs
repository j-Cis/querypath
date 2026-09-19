use anyhow::Result;
use querypath::{FsWalk, PattEnvIndex, PathsEntry, PathsOptions, PathsPatterns};
use std::collections::HashSet;

/// Adapter mapujący skan fizyczny dysku na indeks pamięciowy (Zero-I/O).
struct WalkEnvIndex<'a> {
    dirs: HashSet<&'a str>,
    files: Vec<&'a str>,
}

impl<'a> WalkEnvIndex<'a> {
    fn from_walk(walk: &'a FsWalk) -> Self {
        let dirs = walk.dirs.iter().map(|d| d.str.as_str()).collect();
        let files = walk.files.iter().map(|f| f.str.as_str()).collect();
        Self { dirs, files }
    }
}

impl<'a> PattEnvIndex for WalkEnvIndex<'a> {
    fn has_dir(&self, dir: &str) -> bool {
        self.dirs.contains(dir)
    }

    fn has_file_with_prefix(&self, prefix: &str) -> bool {
        let start = self.files.partition_point(|&f| f < prefix);
        start < self.files.len() && self.files[start].starts_with(prefix)
    }

    fn any_file_in_dir(&self, dir: &str, check: &mut dyn FnMut(&str) -> bool) -> bool {
        let start = self.files.partition_point(|&f| f < dir);
        for &f in &self.files[start..] {
            if !f.starts_with(dir) {
                break;
            }
            if check(f) {
                return true;
            }
        }
        false
    }
}

/// Struktura konfigurująca zapytanie i wykonująca skanowanie.
#[derive(Debug, Clone, Default)]
pub struct QueryScanner {
    pub scan_at: Vec<String>,
    pub match_pattern: Vec<String>,
    pub options: PathsOptions,
}

impl QueryScanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scan_at<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scan_at.extend(paths.into_iter().map(Into::into));
        self
    }

    pub fn match_pattern<I, S>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.match_pattern
            .extend(patterns.into_iter().map(Into::into));
        self
    }

    pub fn keep_parent(mut self, keep: bool) -> Self {
        self.options.keep_parent = keep;
        self
    }

    pub fn ignore_case(mut self, ignore: bool) -> Self {
        self.options.ignore_case = ignore;
        self
    }

    pub fn run(&self) -> Result<()> {
        let entry = PathsEntry::build(&self.scan_at)?;

        println!("🔍 [querypath] Inicjalizacja skanowania...");
        println!(" ├─ Katalog roboczy (CWD): {}", entry.execution_dir.str);
        println!(" ├─ Lokalizacje (scan_at): {:?}", self.scan_at);
        println!(" ├─ Wzorce (match_pattern): {:?}", self.match_pattern);
        println!(" └─ Opcja keep_parent: {}", self.options.keep_parent);

        let walk = FsWalk::scan(&entry)?;
        println!(
            "📦 Zeskanowano fizycznie: {} plików, {} katalogów",
            walk.files.len(),
            walk.dirs.len()
        );

        let env = WalkEnvIndex::from_walk(&walk);
        let patterns = PathsPatterns::build(&self.match_pattern, self.options.ignore_case)?;

        let matched_files: Vec<&str> = walk
            .files
            .iter()
            .map(|f| f.str.as_str())
            .filter(|f| patterns.is_match(f, &env))
            .collect();

        let mut matched_dirs_set: HashSet<String> = walk
            .dirs
            .iter()
            .map(|d| d.str.as_str())
            .filter(|d| patterns.is_match(d, &env))
            .map(String::from)
            .collect();

        // Jeśli opcja keep_parent jest włączona, dołączamy katalogi nadrzędne trafień
        if self.options.keep_parent {
            for file_path in &matched_files {
                let mut current: &str = file_path;
                while let Some((parent, _)) = current.rsplit_once('/') {
                    if parent.is_empty() {
                        break;
                    }
                    let dir_candidate = format!("{parent}/");
                    if env.has_dir(&dir_candidate) {
                        matched_dirs_set.insert(dir_candidate);
                    }
                    current = parent;
                }
            }
        }

        let mut matched_dirs: Vec<String> = matched_dirs_set.into_iter().collect();
        matched_dirs.sort();

        println!("\n✨ Dopasowane katalogi ({}):", matched_dirs.len());
        for d in &matched_dirs {
            println!(" 📁 {d}");
        }

        println!("\n✨ Dopasowane pliki ({}):", matched_files.len());
        for f in &matched_files {
            println!(" 📄 {f}");
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    // Wywołanie przykładowe na fizycznej strukturze testowej
    QueryScanner::new()
        .scan_at(["./"])
        .match_pattern(["*.rs"])
		.keep_parent(true)
        .run()?;

    Ok(())
}