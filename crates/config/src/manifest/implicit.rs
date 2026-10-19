//! The Manifest a repository with no `armada.yml` is hosted under.
//!
//! **Nothing is read or written.** It declares no Check and no Command, so
//! nothing gates on it, and it states no pool size, so the default one applies.
//! It exists so a Job there is named the way every Job is, by a Manifest id.

use std::path::Path;

use super::Manifest;

impl Manifest {
    /// What `root` is hosted under until it has an `armada.yml`: only `id`, and
    /// `base` where the repository's own branch is not `main`. Its `path` is the
    /// file a person would write, which does not exist.
    pub fn implicit(root: &Path, id: &str, base: Option<&str>) -> Manifest {
        let text = format!("version: 1\nid: '{}'\n", id.replace('\'', "''"));
        let mut manifest = Manifest::parse(&root.join("armada.yml"), &text)
            .expect("a version and an id are a Manifest");
        manifest.base = base.map(str::to_string);
        manifest
    }
}
