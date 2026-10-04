//! What a change reaches in a Cargo workspace: the members it touches, and
//! every member that depends on one. The merge line's `armada check --changed`
//! and Fleet's step gate both narrow a Check over these, through the
//! Manifest's own `narrow` read by [`narrowed_over`](crate::narrowed_over).
//! `docs/capabilities/merge-line.md`, *What a narrowed Check runs*.
//!
//! **The only Cargo fact either gate holds**, asked of `cargo tree` rather than
//! read from a manifest, and kept out of [`narrow`](crate::narrowed) for the
//! reason that file gives. Here rather than in `armada` since 4 Oct 2026, so
//! Fleet's step gate asks the same question the merge line does.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// What [`reached`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reach {
    /// The changed paths, with the directory of every member depending on one
    /// they touch.
    Paths(Vec<String>),
    /// Nothing narrows, and why.
    Whole(String),
}

/// Files whose change can reach every member, whoever depends on whom: what
/// the workspace resolves, how it compiles, or what a member's build runs.
const WHOLE_WORKSPACE: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "build.rs",
    "rust-toolchain",
    "rust-toolchain.toml",
];

/// `changed`, widened to the members depending on what it touches, or the
/// reason a Check runs whole. **Blocking**: it runs `cargo tree` twice.
pub fn reached(worktree: &Path, changed: &[String]) -> Reach {
    if let Some(path) = changed.iter().find(|path| reaches_everything(path)) {
        return Reach::Whole(format!("{path} reaches the whole workspace"));
    }
    if !worktree.join("Cargo.toml").is_file() {
        return Reach::Whole("there is no Cargo workspace to say what depends on what".into());
    }
    let root = match worktree.canonicalize() {
        Ok(root) => root,
        Err(why) => {
            return Reach::Whole(format!("{} could not be read: {why}", worktree.display()))
        }
    };
    let workspace = match tree(worktree, &["--workspace", "--depth", "0"]) {
        Ok(listed) => members(&listed, &root),
        Err(why) => return Reach::Whole(why),
    };
    if workspace.iter().any(|(_, dir)| dir.is_empty()) {
        return Reach::Whole("the workspace root is a package of its own".into());
    }
    // Only Rust source travels by dependency. `ipc`'s tests read `testkit`'s
    // fixtures without depending on it, and nothing in the graph says so.
    let inside = |path: &String| workspace.iter().any(|(_, dir)| within(path, dir));
    if let Some(path) = changed
        .iter()
        .find(|path| inside(path) && !path.ends_with(".rs"))
    {
        return Reach::Whole(format!(
            "{path} is not Rust source, and a member that does not depend on its own may read it"
        ));
    }
    let touched: BTreeSet<&str> = workspace
        .iter()
        .filter(|(_, dir)| changed.iter().any(|path| within(path, dir)))
        .map(|(name, _)| name.as_str())
        .collect();
    let mut paths: BTreeSet<String> = changed.iter().cloned().collect();
    if !touched.is_empty() {
        let mut args = vec!["-e", "normal,build,dev"];
        for name in &touched {
            args.extend(["-i", name]);
        }
        match tree(worktree, &args) {
            Ok(listed) => paths.extend(members(&listed, &root).into_iter().map(|(_, dir)| dir)),
            Err(why) => return Reach::Whole(why),
        }
    }
    Reach::Paths(paths.into_iter().collect())
}

fn reaches_everything(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    WHOLE_WORKSPACE.contains(&name) || path.starts_with(".cargo/")
}

fn within(path: &str, dir: &str) -> bool {
    path == dir
        || path
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// `cargo tree`, offline and on the lockfile as it stands: a change to either
/// has already run whole.
fn tree(worktree: &Path, args: &[&str]) -> Result<String, String> {
    let ran = Command::new("cargo")
        .args(["tree", "--offline", "--locked", "--prefix", "none"])
        .args(args)
        .current_dir(worktree)
        .stdin(Stdio::null())
        .output()
        .map_err(|why| format!("`cargo tree` could not be run: {why}"))?;
    match ran.status.success() {
        true => Ok(String::from_utf8_lossy(&ran.stdout).into_owned()),
        false => Err(format!(
            "`cargo tree` could not say what depends on what: {}",
            String::from_utf8_lossy(&ran.stderr).trim()
        )),
    }
}

/// Every package `cargo tree` listed at a path inside `root`, by name and by
/// its directory relative to `root`. A line reads `name v0.1.0 (/abs/path)`,
/// with ` (*)` after one already shown.
fn members(listed: &str, root: &Path) -> BTreeSet<(String, String)> {
    listed
        .lines()
        .filter_map(|line| {
            let name = line.split_whitespace().next()?;
            let path = line
                .split(" (")
                .skip(1)
                .map(|rest| rest.trim_end_matches(" (*)").trim_end_matches(')'))
                .find(|inside| inside.starts_with('/'))?;
            let dir = PathBuf::from(path);
            let dir = dir.strip_prefix(root).ok()?;
            Some((name.to_string(), dir.to_string_lossy().into_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{members, reached, Reach};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// A directory for one test, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> TempDir {
            let path = std::env::temp_dir().join(format!(
                "armada-reach-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).expect("a temporary directory");
            TempDir(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn write(&self, relative: &str, contents: &str) {
            let at = self.0.join(relative);
            std::fs::create_dir_all(at.parent().expect("a parent")).expect("the parent");
            std::fs::write(at, contents).expect("the file");
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn paths(of: &[&str]) -> Vec<String> {
        of.iter().map(|path| path.to_string()).collect()
    }

    #[test]
    fn a_listing_names_each_package_inside_the_root_once() {
        let listed = "fleet v0.0.0 (/w/crates/fleet)\n\
                      armada v0.0.0 (/w/crates/armada)\n\
                      fleet v0.0.0 (/w/crates/fleet) (*)\n\
                      serde v1.0.0\n\
                      outside v0.1.0 (/elsewhere/outside)\n\
                      \n\
                      xtask v0.0.0 (/w/xtask)\n";
        let found: Vec<(String, String)> = members(listed, Path::new("/w")).into_iter().collect();
        assert_eq!(
            found,
            vec![
                ("armada".to_string(), "crates/armada".to_string()),
                ("fleet".to_string(), "crates/fleet".to_string()),
                ("xtask".to_string(), "xtask".to_string()),
            ]
        );
    }

    /// `leaf` depends on `mid` depends on `base`, and `apart` on nothing.
    fn a_workspace() -> TempDir {
        let dir = TempDir::new();
        dir.write(
            "Cargo.toml",
            "[workspace]\nresolver = \"2\"\nmembers = [\"crates/*\"]\n",
        );
        for (name, depends) in [
            ("base", ""),
            ("mid", "base = { path = \"../base\" }\n"),
            ("leaf", "mid = { path = \"../mid\" }\n"),
            ("apart", ""),
        ] {
            dir.write(
                &format!("crates/{name}/Cargo.toml"),
                &format!(
                    "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
                     [dependencies]\n{depends}"
                ),
            );
            dir.write(&format!("crates/{name}/src/lib.rs"), "");
        }
        let locked = Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(dir.path())
            .output()
            .expect("cargo runs");
        assert!(locked.status.success(), "{locked:?}");
        dir
    }

    #[test]
    fn a_change_reaches_its_member_and_every_member_depending_on_it() {
        let dir = a_workspace();
        assert_eq!(
            reached(dir.path(), &paths(&["crates/base/src/lib.rs", "docs/a.md"])),
            Reach::Paths(paths(&[
                "crates/base",
                "crates/base/src/lib.rs",
                "crates/leaf",
                "crates/mid",
                "docs/a.md",
            ]))
        );
        assert_eq!(
            reached(dir.path(), &paths(&["crates/leaf/src/lib.rs"])),
            Reach::Paths(paths(&["crates/leaf", "crates/leaf/src/lib.rs"]))
        );
    }

    #[test]
    fn the_lockfile_a_manifest_a_build_script_or_a_data_file_runs_whole() {
        let dir = a_workspace();
        for path in [
            "Cargo.lock",
            "crates/leaf/Cargo.toml",
            "crates/leaf/build.rs",
            ".cargo/config.toml",
            "crates/apart/fixtures/one.ndjson",
        ] {
            assert!(
                matches!(
                    reached(dir.path(), &paths(&["crates/leaf/src/lib.rs", path])),
                    Reach::Whole(_)
                ),
                "{path}"
            );
        }
    }

    #[test]
    fn a_tree_with_no_cargo_workspace_runs_whole() {
        let dir = TempDir::new();
        assert!(matches!(
            reached(dir.path(), &paths(&["crates/a/src/lib.rs"])),
            Reach::Whole(_)
        ));
    }
}
