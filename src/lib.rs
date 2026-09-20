// ./src/lib.rs
// querypath

#[path = "code/paths_entry.rs"]
pub mod paths_entry;
pub use paths_entry::{AnchoredPath, PathContext, PathNode, PathsEntry, normalize_path};

#[path = "code/fs_walk.rs"]
pub mod fs_walk;
pub use fs_walk::{FsWalk, FsWalkStat};

#[path = "code/paths_patterns.rs"]
pub mod paths_patterns;
pub use paths_patterns::{PathsPatterns, PattEnvIndex, PattExp, PattRaw};

#[path = "code/paths_options.rs"]
pub mod paths_options;
pub use paths_options::PathsOptions;

#[path = "code/querypath.rs"]
pub mod querypath;
pub use querypath::{DirItem, FileItem, QueryPath, QueryResults};
