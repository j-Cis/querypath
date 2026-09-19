// ./src/code/fs_walk.rs
use anyhow::Result;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::paths_entry::{AnchoredPath, PathNode, PathsEntry, normalize_path};

/// Statystyki surowego skanowania dysku (przed filtrowaniem wzorcami i opcjami).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FsWalkStat {
    pub count_files: usize,
    pub count_dirs: usize,
    pub count_empty_dirs: usize,
    pub count_empty_files: usize,
}

/// Surowy rezultat przejścia po drzewie katalogów.
#[derive(Debug, Clone)]
pub struct FsWalk {
    pub files: Vec<PathNode>,
    pub dirs: Vec<PathNode>,
    pub stat: FsWalkStat,
}

impl FsWalk {
    /// Wykonuje surowe skanowanie dla wszystkich celów w `PathsEntry`.
    pub fn scan(entry: &PathsEntry) -> Result<Self> {
        let mut files = Vec::new();
        let mut dirs = Vec::new();

        let mut count_files = 0;
        let mut count_dirs = 0;
        let mut count_empty_dirs = 0;
        let mut count_empty_files = 0;

        for target in &entry.targets {
            Self::scan_target(
                target,
                &mut files,
                &mut dirs,
                &mut count_files,
                &mut count_dirs,
                &mut count_empty_dirs,
                &mut count_empty_files,
            )?;
        }

        // Deterministyczne sortowanie
        files.sort_unstable_by(|a, b| a.str.cmp(&b.str));
        dirs.sort_unstable_by(|a, b| a.str.cmp(&b.str));

        // Usunięcie ewentualnych nakładających się duplikatów
        files.dedup_by(|a, b| a.str == b.str);
        dirs.dedup_by(|a, b| a.str == b.str);

        Ok(Self {
            files,
            dirs,
            stat: FsWalkStat {
                count_files,
                count_dirs,
                count_empty_dirs,
                count_empty_files,
            },
        })
    }

    fn scan_target(
        target: &AnchoredPath,
        files: &mut Vec<PathNode>,
        dirs: &mut Vec<PathNode>,
        count_files: &mut usize,
        count_dirs: &mut usize,
        count_empty_dirs: &mut usize,
        count_empty_files: &mut usize,
    ) -> Result<()> {
        let root = &target.target_dir.buf;

        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            // Pomijamy korzeń (depth 0)
            if entry.depth() == 0 {
                continue;
            }

            // Pomijamy dowiązania symboliczne
            if entry.path_is_symlink() {
                continue;
            }

            let Ok(rel_prefix) = entry.path().strip_prefix(&target.execution_dir.buf) else {
                continue;
            };

            let norm_rel = normalize_path(rel_prefix);
            let clean_rel = norm_rel.trim_start_matches("./");

            if entry.file_type().is_dir() {
                *count_dirs += 1;

                let is_empty = entry
                    .path()
                    .read_dir()
                    .map(|mut r| r.next().is_none())
                    .unwrap_or(false);

                if is_empty {
                    *count_empty_dirs += 1;
                }

                let dir_str = format!("./{}/", clean_rel.trim_end_matches('/'));
                dirs.push(PathNode::new(PathBuf::from(dir_str)));
            } else {
                *count_files += 1;

                let is_empty = entry.metadata().map(|m| m.len() == 0).unwrap_or(false);

                if is_empty {
                    *count_empty_files += 1;
                }

                let file_str = format!("./{}", clean_rel);
                files.push(PathNode::new(PathBuf::from(file_str)));
            }
        }

        Ok(())
    }
}
