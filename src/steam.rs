use std::fs;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSetBuilder};
use walkdir::WalkDir;

pub fn collect_game_ids(base: &Path, filter: Option<&str>, ignore: &[String]) -> Vec<String> {
    let entries = fs::read_dir(base).unwrap();
    entries
        .filter_map(|e| {
            let name = e.ok()?.file_name().to_string_lossy().into_owned();
            if ignore.contains(&name) {
                return None;
            }
            if filter.is_none_or(|f| f == name) {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

pub fn get_save_path(base: &Path, game_id: &str) -> Option<PathBuf> {
    let root = base
        .join(game_id)
        .join("pfx/drive_c/users/steamuser/AppData");
    if root.exists() {
        return Some(root);
    }
    None
}

pub fn get_save_files(base: &Path, excluded_patterns: &Vec<String>) -> Vec<PathBuf> {
    // Build a glob set from patterns
    let mut builder = GlobSetBuilder::new();
    for pat in excluded_patterns {
        if let Ok(glob) = Glob::new(pat) {
            builder.add(glob);
        }
    }
    let set = builder.build().unwrap();

    WalkDir::new(base)
        .into_iter()
        .filter_entry(|e| {
            let relative = e.path().strip_prefix(base).unwrap();
            !set.is_match(relative)
        })
        .filter_map(Result::ok)
        .map(|p| p.into_path())
        .collect()
}
