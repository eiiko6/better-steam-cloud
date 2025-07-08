use std::fs;
use std::path::{Path, PathBuf};

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
    let save_dirs = ["Local", "LocalLow", "Roaming"];
    for subdir in save_dirs {
        let full = root.join(subdir);
        if full.exists() {
            if let Some(custom) = find_custom_dirs(&full) {
                return Some(custom);
            }
        }
    }
    None
}

fn find_custom_dirs(dir: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        let fname = entry.file_name().into_string().ok()?;
        if fname != "Microsoft" && fname != "Temp" {
            return Some(path);
        }
    }
    None
}
