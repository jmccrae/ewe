//! Staleness detection for a WordNet YAML source directory - shared by anything that caches
//! or holds an in-memory copy of the source and needs to notice when the files on disk have
//! moved on since (`ewe_dioxus`'s ReDB cache rebuild, `ewe_mcp`'s save-time staleness check).

use std::io;
use std::path::Path;
use std::time::SystemTime;

/// The most recent modification time among every file under `source` (searched recursively,
/// since large sources like NameNet split files across subdirectories) and the sibling
/// `deprecations.csv` (`source/../deprecations.csv`), if present.
pub fn latest_source_mtime(source: impl AsRef<Path>) -> io::Result<SystemTime> {
    let source = source.as_ref();
    let mut latest = SystemTime::UNIX_EPOCH;
    latest_mtime_recursive(source, &mut latest)?;
    let dep_file = source.join("../deprecations.csv");
    if let Ok(mtime) = dep_file.metadata().and_then(|m| m.modified()) {
        latest = latest.max(mtime);
    }
    Ok(latest)
}

/// Recurse into `dir` and its subdirectories, updating `latest` with the newest modification
/// time found among all files.
fn latest_mtime_recursive(dir: &Path, latest: &mut SystemTime) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            latest_mtime_recursive(&path, latest)?;
        } else {
            let mtime = entry.metadata()?.modified()?;
            *latest = (*latest).max(mtime);
        }
    }
    Ok(())
}
