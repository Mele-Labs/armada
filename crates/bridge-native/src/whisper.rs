//! Whisper for Talk, installed on the person's press and never before, into the folder the
//! Electron Bridge keeps it in, so a Mac that has one has both. whisper.cpp publishes no macOS
//! command-line build, so the library is the Apple build its release attaches, whose framework
//! Armada Voice loads; the release is pinned because the helper is compiled against its structs.
//! The model is `base.en` quantised to 5 bits. Both are checked against the digests below before
//! anything is kept. `apps/desktop/src/main/whisper-install.ts` is the original.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

const RELEASE: &str = "b5454";
const MODEL: &str = "ggml-base.en-q5_1.bin";
const FRAMEWORK_IN_ZIP: &str = "build-apple/whisper.xcframework/macos-arm64_x86_64/whisper.framework";
const LIBRARY_URL: &str = "https://github.com/ggml-org/whisper.cpp/releases/download/b5454/whisper-b5454-xcframework.zip";
const LIBRARY_SHA: &str = "e57f8c48933000acabc13bb913fbe82805d483a2deff692cc6738836bd92393b";
const MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-base.en-q5_1.bin";
const LIBRARY_BYTES: u64 = 65_816_082;
const MODEL_BYTES: u64 = 59_721_011;
const MODEL_SHA: &str = "4baf70dd0d7c4247ba2b81fafd9c01005ac77c2f9ef064e00dcf195d0e2fdd2f";

fn dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/Armada/whisper"))
}

/// What the marker holds when this release and model are in.
fn mark() -> String {
    format!(r#"{{"release":"{RELEASE}","model":"{MODEL}"}}"#)
}

/// The library and model Armada Voice loads, once installed.
pub fn paths() -> Option<(String, String)> {
    let dir = dir()?;
    let marked = std::fs::read_to_string(dir.join("installed.json")).ok()? == mark();
    marked.then(|| (format!("{}/whisper.framework/Versions/A/whisper", dir.display()), format!("{}/{MODEL}", dir.display())))
}

static RUNNING: AtomicBool = AtomicBool::new(false);

/// What has come down so far, as thousandths of both files together.
static PROGRESS: AtomicU32 = AtomicU32::new(0);

/// Whisper on this Mac: not here, coming down (0 to 1), or here.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Absent,
    Installing(f32),
    Installed,
}

pub fn state() -> State {
    if RUNNING.load(Ordering::SeqCst) {
        State::Installing(PROGRESS.load(Ordering::Relaxed) as f32 / 1000.)
    } else if paths().is_some() {
        State::Installed
    } else {
        State::Absent
    }
}

/// Starts the install, or joins the one under way. It runs off the main thread and keeps nothing
/// to report: the next press finds Whisper there, or does not.
pub fn install() {
    if paths().is_some() || RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    PROGRESS.store(0, Ordering::Relaxed);
    std::thread::spawn(|| {
        if let Some(dir) = dir() {
            if fetch(&dir).is_err() {
                // A stopped install leaves nothing: the folder holds only Whisper.
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
        RUNNING.store(false, Ordering::SeqCst);
    });
}

fn fetch(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| why.to_string())?;
    let zip = dir.join("whisper.zip");
    download(LIBRARY_URL, &zip, LIBRARY_SHA, 0, LIBRARY_BYTES)?;
    download(MODEL_URL, &dir.join(MODEL), MODEL_SHA, LIBRARY_BYTES, MODEL_BYTES)?;
    let staging = dir.join("unpacking");
    let _ = std::fs::remove_dir_all(&staging);
    // `unzip` keeps the framework's symlinks, and takes one slice of a zip that holds every Apple platform.
    let unzip = Command::new("/usr/bin/unzip")
        .args(["-q"])
        .arg(&zip)
        .arg(format!("{FRAMEWORK_IN_ZIP}/*"))
        .arg("-d")
        .arg(&staging)
        .status()
        .map_err(|why| why.to_string())?;
    if !unzip.success() {
        return Err("unpack".into());
    }
    let framework = dir.join("whisper.framework");
    let _ = std::fs::remove_dir_all(&framework);
    std::fs::rename(staging.join(FRAMEWORK_IN_ZIP), &framework).map_err(|why| why.to_string())?;
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_file(&zip);
    std::fs::write(dir.join("installed.json"), mark()).map_err(|why| why.to_string())
}

/// Fetches `url` into `file`, telling `PROGRESS` as it grows. `before` is what the earlier file
/// brought, so the bar is one for both. Held under 1 until unpacked: a full bar is installed.
fn download(url: &str, file: &Path, sha256: &str, before: u64, bytes: u64) -> Result<(), String> {
    let mut child = Command::new("/usr/bin/curl").args(["-fsSL", "-o"]).arg(file).arg(url).spawn().map_err(|why| why.to_string())?;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|why| why.to_string())? {
            break status;
        }
        let here = std::fs::metadata(file).map_or(0, |one| one.len().min(bytes));
        PROGRESS.store(progress_of(before + here), Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(200));
    };
    if !status.success() {
        return Err("download".into());
    }
    let out = Command::new("/usr/bin/shasum").args(["-a", "256"]).arg(file).stdout(Stdio::piped()).output().map_err(|why| why.to_string())?;
    let said = String::from_utf8_lossy(&out.stdout);
    if said.split_whitespace().next() == Some(sha256) {
        Ok(())
    } else {
        Err("digest".into())
    }
}

/// Thousandths of both files, never the last of them.
fn progress_of(done: u64) -> u32 {
    ((done * 1000 / (LIBRARY_BYTES + MODEL_BYTES)) as u32).min(990)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_bar_is_one_for_both_files_and_never_full_before_it_is_unpacked() {
        assert_eq!(super::progress_of(0), 0);
        assert_eq!(super::progress_of(super::LIBRARY_BYTES), 524);
        assert_eq!(super::progress_of(super::LIBRARY_BYTES + super::MODEL_BYTES), 990);
    }

    #[test]
    fn the_marker_names_this_release_and_model() {
        assert_eq!(super::mark(), r#"{"release":"b5454","model":"ggml-base.en-q5_1.bin"}"#);
    }
}
