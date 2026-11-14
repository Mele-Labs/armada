//! Launch Services knows this app by its bundle id, and `lsregister -dump` lists every `builds/<stamp>/Armada.app`
//! ever opened under it. A notification press launches the bundle Launch Services picks, which can be an old one
//! beside the running app. So the live app registers its own bundle and unregisters every other one under
//! `<support>/builds/`, and `scripts/keep-build` unregisters a bundle before it deletes it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::log;

const LSREGISTER: &str = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
const BUNDLE: &str = "Armada.app";

/// The bundle an executable runs from (`<bundle>/Contents/MacOS/<exe>`), when `builds` holds it.
/// Both paths are canonical.
pub fn own_bundle(exe: &Path, builds: &Path) -> Option<PathBuf> {
    let bundle = exe.ancestors().nth(3)?;
    (bundle.extension()? == "app" && bundle.starts_with(builds)).then(|| bundle.to_path_buf())
}

/// Every bundle in `found` but `own`, each once. `found` is canonical, so `builds/current` and the stamp it
/// points at are one bundle.
pub fn others(own: &Path, found: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    found.into_iter().filter(|bundle| bundle != own).collect::<BTreeSet<_>>().into_iter().collect()
}

/// `<support>/builds`, canonical, where this app's bundles are kept.
fn builds_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    PathBuf::from(home).join("Library/Application Support/Armada/builds").canonicalize().ok()
}

fn lsregister(flag: &str, bundle: &Path) {
    match Command::new(LSREGISTER).arg(flag).arg(bundle).output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => log::Note::warn("lsregister refused a bundle")
            .field("flag", flag)
            .field("bundle", bundle.display().to_string())
            .field("status", out.status.to_string())
            .emit(),
        Err(why) => log::Note::warn("lsregister did not run").field("bundle", bundle.display().to_string()).field("why", why.to_string()).emit(),
    }
}

/// On a background thread: register this bundle and unregister the others, when this app runs from the builds
/// folder. A run from anywhere else (a cargo target, a walk) leaves Launch Services alone.
pub fn point_at_current_build() {
    std::thread::spawn(|| {
        let Some(builds) = builds_dir() else { return };
        let Some(own) = std::env::current_exe().ok().and_then(|exe| exe.canonicalize().ok()).and_then(|exe| own_bundle(&exe, &builds)) else { return };
        let found = std::fs::read_dir(&builds)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.path().join(BUNDLE).canonicalize().ok());
        lsregister("-f", &own);
        for stale in others(&own, found) {
            lsregister("-u", &stale);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(stamp: &str) -> PathBuf {
        PathBuf::from(format!("/s/Armada/builds/{stamp}/Armada.app"))
    }

    #[test]
    fn the_own_bundle_is_the_one_the_executable_sits_in_when_the_builds_folder_holds_it() {
        let builds = Path::new("/s/Armada/builds");
        let exe = bundle("2026-11-14T0Z-a").join("Contents/MacOS/Armada");
        assert_eq!(own_bundle(&exe, builds), Some(bundle("2026-11-14T0Z-a")));
        assert_eq!(own_bundle(Path::new("/repo/target/debug/bridge-native"), builds), None, "a cargo run is not a build");
        assert_eq!(own_bundle(Path::new("/elsewhere/Armada.app/Contents/MacOS/Armada"), builds), None, "a bundle outside builds");
    }

    #[test]
    fn every_other_bundle_is_stale_once_and_the_own_one_never() {
        let own = bundle("b");
        // `current` and `previous` canonicalise to a stamp, so the same bundle can be found twice.
        let found = [bundle("c"), bundle("b"), bundle("a"), bundle("c"), bundle("b")];
        assert_eq!(others(&own, found), vec![bundle("a"), bundle("c")]);
        assert!(others(&own, [bundle("b")]).is_empty());
    }
}
