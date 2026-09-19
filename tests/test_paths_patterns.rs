use std::collections::HashSet;
use querypath::{FsWalk, PattEnvIndex, PathsEntry, PathsPatterns};

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
fn scenariusz_1_dopasowanie_globo_klamrowe() {
    let entry = PathsEntry::build(["./tests/_/.gh/"]).unwrap();
    let walk = FsWalk::scan(&entry).unwrap();
    let env = WalkEnvIndex::from_walk(&walk);

    // Szukamy plików z rozszerzeniem .rs oraz .wq za pomocą separatora '|'
    let patterns = PathsPatterns::build(["*.{rs|wq}"], false).unwrap();

    let matched: Vec<&str> = walk
        .files
        .iter()
        .map(|f| f.str.as_str())
        .filter(|f| patterns.is_match(f, &env))
        .collect();

    assert_eq!(matched.len(), 4);
    assert!(matched.contains(&"./tests/_/.gh/wrt.rs"));
    assert!(matched.contains(&"./tests/_/.gh/lib/jewe.rs"));
    assert!(matched.contains(&"./tests/_/.gh/lib/dw/wewe.rs"));
    assert!(matched.contains(&"./tests/_/.gh/lib/dw/dwew/src/erw.wq"));
}

#[test]
fn scenariusz_2_negacja_wykluczenia() {
    let entry = PathsEntry::build(["./tests/_/.gh/"]).unwrap();
    let walk = FsWalk::scan(&entry).unwrap();
    let env = WalkEnvIndex::from_walk(&walk);

    // Szukamy plików .rs, ale wykluczamy zawartość folderu dw/
    let patterns = PathsPatterns::build(["*.rs", "!**/dw/*"], false).unwrap();

    let matched: Vec<&str> = walk
        .files
        .iter()
        .map(|f| f.str.as_str())
        .filter(|f| patterns.is_match(f, &env))
        .collect();

    assert_eq!(matched.len(), 2);
    assert!(matched.contains(&"./tests/_/.gh/wrt.rs"));
    assert!(matched.contains(&"./tests/_/.gh/lib/jewe.rs"));
    assert!(!matched.contains(&"./tests/_/.gh/lib/dw/wewe.rs"));
}

#[test]
fn scenariusz_3_bezposredni_regex() {
    let entry = PathsEntry::build(["./tests/_/.gh/"]).unwrap();
    let walk = FsWalk::scan(&entry).unwrap();
    let env = WalkEnvIndex::from_walk(&walk);

    // Używamy prefiksu re: do znalezienia pliku z 'erw' i rozszerzeniem 'wq'
    let patterns = PathsPatterns::build(["re:.*erw\\.wq$"], false).unwrap();

    let matched: Vec<&str> = walk
        .files
        .iter()
        .map(|f| f.str.as_str())
        .filter(|f| patterns.is_match(f, &env))
        .collect();

    assert_eq!(matched, vec!["./tests/_/.gh/lib/dw/dwew/src/erw.wq"]);
}

#[test]
fn scenariusz_4_tylko_katalogi() {
    let entry = PathsEntry::build(["./tests/_/"]).unwrap();
    let walk = FsWalk::scan(&entry).unwrap();
    let env = WalkEnvIndex::from_walk(&walk);

    // Wzorzec z '/' na końcu dotyczy WYŁĄCZNIE katalogów o nazwie 'src'
    let patterns = PathsPatterns::build(["**/src/"], false).unwrap();

    let matched_dirs: Vec<&str> = walk
        .dirs
        .iter()
        .map(|d| d.str.as_str())
        .filter(|d| patterns.is_match(d, &env))
        .collect();

    // W nowej strukturze znajduje się dokładnie 7 katalogów 'src/'
    assert_eq!(matched_dirs.len(), 7);
    assert!(matched_dirs.contains(&"./tests/_/.gh/src/"));
    assert!(matched_dirs.contains(&"./tests/_/.gh/lib/dw/dwew/src/"));
    assert!(matched_dirs.contains(&"./tests/_/.ijk/src/"));
    assert!(matched_dirs.contains(&"./tests/_/abc/src/"));
    assert!(matched_dirs.contains(&"./tests/_/def/src/"));
    assert!(matched_dirs.contains(&"./tests/_/mn@copy/src/"));
    assert!(matched_dirs.contains(&"./tests/_/mn@copy/lib/d$w/dwew/src/"));

    // Żaden plik nie powinien zostać dopasowany przez wzorzec katalogowy
    let matched_files_count = walk
        .files
        .iter()
        .filter(|f| patterns.is_match(&f.str, &env))
        .count();

    assert_eq!(matched_files_count, 0);
}

#[test]
fn scenariusz_5_ochrona_znaki_specialne_w_fizycznych_sciezkach() {
    let entry = PathsEntry::build(["./tests/_/"]).unwrap();
    let walk = FsWalk::scan(&entry).unwrap();
    let env = WalkEnvIndex::from_walk(&walk);

    // 1. Sprawdzamy dopasowanie plików zawierających '@' i '$' w nazwie
    let patterns_rt = PathsPatterns::build(["*.rt"], false).unwrap();
    let matched_rt: Vec<&str> = walk
        .files
        .iter()
        .map(|f| f.str.as_str())
        .filter(|f| patterns_rt.is_match(f, &env))
        .collect();

    assert_eq!(matched_rt.len(), 2);
    assert!(matched_rt.contains(&"./tests/_/def/io@.rt"));
    assert!(matched_rt.contains(&"./tests/_/def/src/io$.rt"));

    // 2. Dopasowanie wewnątrz mn@copy/ dla plików bezpośrednich oraz w podfolderach {*.rs|**/*.rs}
    let patterns_copy = PathsPatterns::build(["**/mn@copy/{*.rs|**/*.rs}"], false).unwrap();
    let matched_copy: Vec<&str> = walk
        .files
        .iter()
        .map(|f| f.str.as_str())
        .filter(|f| patterns_copy.is_match(f, &env))
        .collect();

    assert_eq!(matched_copy.len(), 3);
    assert!(matched_copy.contains(&"./tests/_/mn@copy/wrt.rs"));
    assert!(matched_copy.contains(&"./tests/_/mn@copy/lib/jewe.rs"));
    assert!(matched_copy.contains(&"./tests/_/mn@copy/lib/d$w/wewe.rs"));
}