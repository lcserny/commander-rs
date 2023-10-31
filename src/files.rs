use std::path::Path;

use eyre::eyre;
use walkdir::{WalkDir, DirEntry};

enum WalkOptions {
    OnlyFiles,
    OnlyDirectories,
}

pub fn walk_files(path: &Path, max_depth: u8) -> eyre::Result<Vec<DirEntry>> {
    walk(path, max_depth, WalkOptions::OnlyFiles)
}

pub fn walk_dirs(path: &Path, max_depth: u8) -> eyre::Result<Vec<DirEntry>> {
    walk(path, max_depth, WalkOptions::OnlyDirectories)
}

fn walk(path: &Path, max_depth: u8, options: WalkOptions) -> eyre::Result<Vec<DirEntry>> {
    if path.is_file() {
        return Err(eyre!("root walk path should be a dir, given {:?}", path));
    }

    if max_depth < 1 {
        return Err(eyre!("max_depth passed cannot be lower than 1, given {}", max_depth));
    }

    Ok(WalkDir::new(path)
        .max_depth(max_depth as usize)
        .follow_links(true)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|f| f.ok())
        .filter(|p| match options {
            WalkOptions::OnlyFiles => p.path().is_file(),
            WalkOptions::OnlyDirectories => p.path().is_dir(),
        })
        .collect()
    )
}
