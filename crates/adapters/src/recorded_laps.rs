//! The test map's laps, fetched from the forge's CI run of a commit.
//! `docs/concepts/test-map.md`, *How it stays current*.
//!
//! **Two `gh` calls, and the second only if there is a run.** The newest `push` run of the
//! workflow for the commit is listed; its artifact `test-map-<commit>` is downloaded, whether the
//! run has finished or not, since the job that uploads it ends before the run does. Nothing is
//! written to the forge.
//!
//! **No artifact is told apart three ways**, because the caller's next move differs. No run yet
//! and a run still going are both [`LapsOnForge::NotYet`], asked again later. A run that finished
//! without the artifact is [`LapsOnForge::Never`]. A forge that could not be asked at all, or a
//! download that failed for any other reason, is [`NotAsked`] with the tool's own words.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use adapter_traits::{LapsOnForge, NotAsked};

use crate::delivery::{read_in, said};

/// The workflow whose run records the laps. `.github/workflows/checks.yml`'s `test_map` job.
pub(crate) const WORKFLOW: &str = "checks.yml";

/// What the download says when the run holds no artifact of that name.
const NO_ARTIFACT: &str = "no valid artifacts found";

static NEXT_FOLDER: AtomicU64 = AtomicU64::new(0);

/// What a run of `gh` came to: whether it succeeded, and what it printed.
pub(crate) struct Ran {
    pub ok: bool,
    pub stdout: String,
    pub said: String,
}

/// The artifact a commit's run uploads. The workflow's `test-map-${{ github.sha }}`.
pub(crate) fn artifact_of(commit: &str) -> String {
    format!("test-map-{commit}")
}

/// The newest `push` run of the workflow for `commit`, one line: its id and its status.
pub(crate) fn list_arguments(commit: &str) -> Vec<String> {
    [
        "run",
        "list",
        "--workflow",
        WORKFLOW,
        "--commit",
        commit,
        "--event",
        "push",
        "--limit",
        "1",
        "--json",
        "databaseId,status",
        "--jq",
        ".[] | \"\\(.databaseId) \\(.status)\"",
    ]
    .map(String::from)
    .to_vec()
}

/// The artifact of `run` for `commit`, into the empty folder `into`.
pub(crate) fn download_arguments(run: &str, commit: &str, into: &Path) -> Vec<String> {
    vec![
        "run".to_string(),
        "download".to_string(),
        run.to_string(),
        "--name".to_string(),
        artifact_of(commit),
        "--dir".to_string(),
        into.to_string_lossy().to_string(),
    ]
}

/// A run as `list_arguments` prints it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Run {
    pub id: String,
    pub finished: bool,
}

/// The run the listing names, or none where it names none.
pub(crate) fn run_in_listing(listing: &str) -> Option<Run> {
    let line = listing.lines().map(str::trim).find(|line| !line.is_empty())?;
    let (id, status) = line.split_once(' ')?;
    (!id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
        .then(|| Run { id: id.to_string(), finished: status.trim() == "completed" })
}

/// Ask the forge through `gh`, in `in_repo`.
pub(crate) fn fetch(in_repo: &str, commit: &str) -> Result<LapsOnForge, NotAsked> {
    fetch_through(
        commit,
        &|args: &[String]| {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            read_in(in_repo, "gh", &args)
                .map(|run| Ran {
                    ok: run.status.success(),
                    stdout: String::from_utf8_lossy(&run.stdout).to_string(),
                    said: said(&run),
                })
                .map_err(|why| why.to_string())
        },
    )
}

/// [`fetch`] over any way of running `gh`, so the decision between not yet, never and a fault is
/// proved without a network.
pub(crate) fn fetch_through(
    commit: &str,
    gh: &dyn Fn(&[String]) -> Result<Ran, String>,
) -> Result<LapsOnForge, NotAsked> {
    let listed = gh(&list_arguments(commit)).map_err(NotAsked)?;
    if !listed.ok {
        return Err(NotAsked(listed.said));
    }
    let Some(run) = run_in_listing(&listed.stdout) else {
        return Ok(LapsOnForge::NotYet);
    };
    let folder = Folder::made()?;
    let downloaded = gh(&download_arguments(&run.id, commit, folder.path())).map_err(NotAsked)?;
    if downloaded.ok {
        return folder.only_file().map(LapsOnForge::Found).map_err(NotAsked);
    }
    match (run.finished, downloaded.said.contains(NO_ARTIFACT)) {
        (false, _) => Ok(LapsOnForge::NotYet),
        (true, true) => Ok(LapsOnForge::Never),
        (true, false) => Err(NotAsked(downloaded.said)),
    }
}

/// An empty folder of our own, gone when this is.
struct Folder(PathBuf);

impl Folder {
    fn made() -> Result<Folder, NotAsked> {
        let path = std::env::temp_dir().join(format!(
            "armada-test-map-{}-{}",
            std::process::id(),
            NEXT_FOLDER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).map_err(|why| NotAsked(format!("{}: {why}", path.display())))?;
        Ok(Folder(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// The bytes of the one file the artifact held.
    fn only_file(&self) -> Result<Vec<u8>, String> {
        let mut files = std::fs::read_dir(&self.0)
            .map_err(|why| format!("{}: {why}", self.0.display()))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .map(|entry| entry.path());
        let file = files.next().ok_or_else(|| "the artifact held no file".to_string())?;
        std::fs::read(&file).map_err(|why| format!("{}: {why}", file.display()))
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
