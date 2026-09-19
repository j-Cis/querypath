use querypath::{FsWalk, PattEnvIndex, PathsEntry, PathsOptions, PathsPatterns};
use std::collections::HashSet;

/// Adapter mapujący rzeczywisty skan dysku (`FsWalk`) na indeks pamięciowy `PattEnvIndex` (Zero-I/O).
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

#[test]
fn scenariusz_1_domyslne_wartosci_opcji() {
    let opts = PathsOptions::new();
    assert!(!opts.keep_parent);
    assert!(!opts.ignore_case);
}

#[test]
fn scenariusz_2_ignore_case_dopasowanie() {
    let entry = PathsEntry::build(["./tests/_/.gh/"]).unwrap();
    let walk = FsWalk::scan(&entry).unwrap();
    let env = WalkEnvIndex::from_walk(&walk);

    // 1. Zignorowanie wielkości liter wyłączone (domyślnie) -> *.RS nie znajdzie plików .rs
    let patterns_strict = PathsPatterns::build(["*.RS"], false).unwrap();
    let matched_strict: Vec<&str> = walk
        .files
        .iter()
        .map(|f| f.str.as_str())
        .filter(|f| patterns_strict.is_match(f, &env))
        .collect();
    assert_eq!(matched_strict.len(), 0);

    // 2. Zignorowanie wielkości liter włączone -> *.RS znajdzie wrt.rs, jewe.rs oraz wewe.rs
    let patterns_ignore = PathsPatterns::build(["*.RS"], true).unwrap();
    let matched_ignore: Vec<&str> = walk
        .files
        .iter()
        .map(|f| f.str.as_str())
        .filter(|f| patterns_ignore.is_match(f, &env))
        .collect();
    assert_eq!(matched_ignore.len(), 3);
    assert!(matched_ignore.contains(&"./tests/_/.gh/wrt.rs"));
    assert!(matched_ignore.contains(&"./tests/_/.gh/lib/jewe.rs"));
    assert!(matched_ignore.contains(&"./tests/_/.gh/lib/dw/wewe.rs"));
}

#[test]
fn scenariusz_3_keep_parent_katalogi_nadrzedne() {
    // Skanujemy od root ./tests/_/ aby .gh/ było wykryte jako podkatalog w env.dirs
    let entry = PathsEntry::build(["./tests/_/"]).unwrap();
    let walk = FsWalk::scan(&entry).unwrap();
    let env = WalkEnvIndex::from_walk(&walk);

    let opts = PathsOptions::new().keep_parent(true);
    let patterns = PathsPatterns::build(["erw.wq"], opts.ignore_case).unwrap();

    // Plik erw.wq znajduje się w: ./tests/_/.gh/lib/dw/dwew/src/erw.wq
    let matched_files: Vec<&str> = walk
        .files
        .iter()
        .map(|f| f.str.as_str())
        .filter(|f| patterns.is_match(f, &env))
        .collect();

    assert_eq!(matched_files.len(), 2);
    assert!(matched_files.contains(&"./tests/_/.gh/lib/dw/dwew/src/erw.wq"));
    assert!(matched_files.contains(&"./tests/_/mn@copy/lib/d$w/dwew/src/erw.wq"));

    // Przetwarzanie katalogów z uwzględnieniem keep_parent
    let mut matched_dirs_set: HashSet<String> = walk
        .dirs
        .iter()
        .map(|d| d.str.as_str())
        .filter(|d| patterns.is_match(d, &env))
        .map(String::from)
        .collect();

    if opts.keep_parent {
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

    // Sprawdzamy, czy wszystkie katalogi nadrzędne dla .gh/ zostały poprawnie wyciągnięte
    assert!(matched_dirs_set.contains("./tests/_/.gh/lib/dw/dwew/src/"));
    assert!(matched_dirs_set.contains("./tests/_/.gh/lib/dw/dwew/"));
    assert!(matched_dirs_set.contains("./tests/_/.gh/lib/dw/"));
    assert!(matched_dirs_set.contains("./tests/_/.gh/lib/"));
    assert!(matched_dirs_set.contains("./tests/_/.gh/"));
}