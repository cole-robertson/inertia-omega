//! Files checked on every render, such as Vite's hot file and the SSR bundle.
//!
//! While watched, they're checked every time, so starting the dev server or
//! building the bundle takes effect at once. Otherwise the first answer is
//! kept for the life of the process: in production they only change with a
//! deploy, so checking them again is a filesystem call per render for nothing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, PoisonError, RwLock};

static CONTENTS: LazyLock<RwLock<HashMap<PathBuf, Option<Arc<str>>>>> = LazyLock::new(RwLock::default);
static EXISTS: LazyLock<RwLock<HashMap<PathBuf, bool>>> = LazyLock::new(RwLock::default);

/// The file's contents, or `None` when it can't be read.
pub(crate) fn read(path: &Path, watch: bool) -> Option<Arc<str>> {
    let read = || std::fs::read_to_string(path).ok().map(Arc::from);

    if watch { read() } else { remember(&CONTENTS, path, read) }
}

/// Whether the file exists.
pub(crate) fn exists(path: &Path, watch: bool) -> bool {
    let exists = || path.exists();

    if watch {
        exists()
    } else {
        remember(&EXISTS, path, exists)
    }
}

fn remember<T: Clone>(cache: &RwLock<HashMap<PathBuf, T>>, path: &Path, load: impl FnOnce() -> T) -> T {
    if let Some(value) = cache.read().unwrap_or_else(PoisonError::into_inner).get(path) {
        return value.clone();
    }

    let value = load();
    cache
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .entry(path.to_owned())
        .or_insert(value)
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path unique per test, since answers are kept for the whole process.
    fn path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("inertia-files-{name}-{}", std::process::id()))
    }

    #[test]
    fn watched_files_are_checked_every_time() {
        let path = path("watched");
        assert_eq!(read(&path, true), None);
        assert!(!exists(&path, true));

        std::fs::write(&path, "http://localhost:5173").unwrap();

        assert_eq!(read(&path, true).as_deref(), Some("http://localhost:5173"));
        assert!(exists(&path, true));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn unwatched_files_keep_their_first_answer() {
        let (missing, present) = (path("missing"), path("present"));
        std::fs::write(&present, "http://localhost:5173").unwrap();
        assert_eq!(read(&missing, false), None);
        assert!(!exists(&missing, false));
        assert!(exists(&present, false));

        std::fs::write(&missing, "http://localhost:5173").unwrap();
        std::fs::remove_file(&present).unwrap();

        assert_eq!(read(&missing, false), None);
        assert!(!exists(&missing, false));
        assert!(exists(&present, false));
        std::fs::remove_file(missing).unwrap();
    }
}
