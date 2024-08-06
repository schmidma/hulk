use std::{env::current_dir, path::PathBuf};

/// Get the repository root directory.
///
/// This function searches for the `hulk.toml` in the current directory and its ancestors.
/// If found, it returns the path to the directory containing the `hulk.toml` directory.
pub fn find_repository_root() -> Option<PathBuf> {
    let path = current_dir().ok()?;
    let ancestors = path.as_path().ancestors();
    ancestors
        .filter_map(|ancestor| std::fs::read_dir(ancestor).ok())
        .flatten()
        .find_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_name() == "hulk.toml" {
                Some(entry.path().parent()?.to_path_buf())
            } else {
                None
            }
        })
}
