//! A folder under the system temp directory, removed on drop, for the cases that read plain files.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Scratch {
    root: PathBuf,
}

impl Scratch {
    pub fn new() -> Scratch {
        let root = std::env::temp_dir().join(format!(
            "armada-scratch-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos()),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("a temporary directory");
        Scratch { root: root.canonicalize().expect("a real path") }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    /// A folder, made.
    pub fn folder(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(&path).expect("a folder");
        path
    }

    /// A file, with its folders made.
    pub fn file(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("its folder");
        std::fs::write(&path, text).expect("the file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
