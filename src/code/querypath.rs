use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::fs_walk::FsWalk;
use crate::paths_entry::PathsEntry;
use crate::paths_options::PathsOptions;
use crate::paths_patterns::{PathsPatterns, PattEnvIndex};

// ============================================================================
// STRUKTURY WYNIKOWE (DOMENA METADANYCH)
// ============================================================================

/// Metadane dopasowanego pliku.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileItem {
	pub path: String,
	pub size: u64,
	pub is_binary: bool,
	pub modified_at: Option<u64>,
}

/// Metadane dopasowanego katalogu z podwójnym rozliczeniem rozmiaru.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirItem {
	pub path: String,
	/// Rzeczywisty, pełny rozmiar fizyczny wszystkich plików wewnątrz katalogu.
	pub real_size: u64,
	/// Sumaryczny rozmiar tylko tych plików, które spełniły kryteria wzorca.
	pub matched_size: u64,
	pub modified_at: Option<u64>,
}

/// Główny wynik zapytania `querypath`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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

// ============================================================================
// INDEX I POMOCNIKI
// ============================================================================

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

/// Badanie pierwszego kilobajta pliku pod kątem bajtu zerowego (`0x00`).
fn check_is_binary(path_str: &str) -> bool {
	let Ok(mut file) = fs::File::open(path_str) else {
		return false;
	};
	let mut buffer = [0u8; 1024];
	let Ok(n) = file.read(&mut buffer) else {
		return false;
	};
	buffer.iter().take(n).any(|&byte| byte == 0)
}

/// Odczyt czasu modyfikacji z metadanych w formacie UNIX timestamp.
fn get_modified_time(path_str: &str) -> Option<u64> {
	fs::metadata(path_str).ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

// ============================================================================
// ORKIESTRATOR (QUERY PATH)
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct QueryPath {
	pub paths: Vec<String>,
	pub patterns: Vec<String>,
	pub options: PathsOptions,
}

impl QueryPath {
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	#[must_use]
	pub fn scan_at<I, S>(mut self, paths: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: Into<String>,
	{
		self.paths.extend(paths.into_iter().map(Into::into));
		self
	}

	#[must_use]
	pub fn match_pattern<I, S>(mut self, patterns: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: Into<String>,
	{
		self.patterns.extend(patterns.into_iter().map(Into::into));
		self
	}

	#[must_use]
	pub fn options(mut self, options: PathsOptions) -> Self {
		self.options = options;
		self
	}

	#[must_use]
	pub fn keep_parent(mut self, keep: bool) -> Self {
		self.options.keep_parent = keep;
		self
	}

	#[must_use]
	pub fn ignore_case(mut self, ignore: bool) -> Self {
		self.options.ignore_case = ignore;
		self
	}

	pub fn run(&self) -> Result<QueryResults> {
		let start_instant = Instant::now();
		let started_at_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

		let entry = PathsEntry::build(&self.paths)?;
		let walk = FsWalk::scan(&entry)?;
		let env = WalkEnvIndex::from_walk(&walk);
		let patterns = PathsPatterns::build(&self.patterns, self.options.ignore_case)?;

		// 1. Zbieranie rozmiarów fizycznych wszystkich zeskanowanych plików
		let mut file_sizes: HashMap<String, u64> = HashMap::new();
		for file in &walk.files {
			let meta_size = fs::metadata(&file.str).map(|m| m.len()).unwrap_or(0);
			file_sizes.insert(file.str.clone(), meta_size);
		}

		// 2. Filtrowanie plików wzorcem oraz budowa FileItem
		let mut matched_files: Vec<FileItem> = Vec::new();
		for file in &walk.files {
			if patterns.is_match(&file.str, &env) {
				let size = *file_sizes.get(&file.str).unwrap_or(&0);
				let is_binary = check_is_binary(&file.str);
				let modified_at = get_modified_time(&file.str);

				matched_files.push(FileItem { path: file.str.clone(), size, is_binary, modified_at });
			}
		}

		// 3. Budowa zbioru katalogów i ich wyliczanie
		let mut matched_dirs_set: HashSet<String> =
			walk.dirs.iter().map(|d| d.str.as_str()).filter(|d| patterns.is_match(d, &env)).map(String::from).collect();

		if self.options.keep_parent {
			for file_item in &matched_files {
				let mut current: &str = file_item.path.as_str();
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

		// 4. Kalkulacja real_size oraz matched_size dla każdego katalogu
		let matched_files_map: HashMap<&str, u64> = matched_files.iter().map(|f| (f.path.as_str(), f.size)).collect();

		let mut matched_dir_items: Vec<DirItem> = Vec::new();

		for dir_path in matched_dirs_set {
			let mut real_size: u64 = 0;
			let mut matched_size: u64 = 0;

			for file in &walk.files {
				if file.str.starts_with(&dir_path) {
					let f_size = *file_sizes.get(&file.str).unwrap_or(&0);
					real_size += f_size;

					if matched_files_map.contains_key(file.str.as_str()) {
						matched_size += f_size;
					}
				}
			}

			let modified_at = get_modified_time(&dir_path);

			matched_dir_items.push(DirItem { path: dir_path, real_size, matched_size, modified_at });
		}

		matched_dir_items.sort_by(|a, b| a.path.cmp(&b.path));

		let finished_at_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

		let duration_ms = start_instant.elapsed().as_millis() as u64;

		Ok(QueryResults {
			execution_dir: entry.execution_dir.str,
			scanned_paths: entry.targets.iter().map(|t| t.relative_path.clone()).collect(),
			patterns: self.patterns.clone(),
			scanned_files: walk.files.len(),
			scanned_dirs: walk.dirs.len(),
			started_at_ms,
			finished_at_ms,
			duration_ms,
			files: matched_files,
			dirs: matched_dir_items,
		})
	}
}
