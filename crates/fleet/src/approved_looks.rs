//! The look he approved on a native walk, kept and checked. `docs/concepts/session/showing-a-window.md`,
//! *The look he approved*.
//!
//! **His Ship it or Approve plays the walk he approved from the build that was Ready**, headless, and
//! keeps one picture a step under `approved-looks/<job>/<walk>`. It runs off the send, which never waits
//! on it. A newer approval of the same walk replaces the older pictures, and where none could be taken
//! nothing is kept and the Job is told.
//!
//! **Before the Job's pull request opens, and on a push to one that is open**, each look the Job kept is
//! set against the Job's head commit (`crate::approved_looks_checking`). A drift goes to the Job, never to him.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{SessionRow, WalkBuildState};

use crate::daemon::Fleet;

/// Under Fleet's support folder: `<job>/<walk>`, one folder a kept look.
pub(crate) const DIR: &str = "approved-looks";

/// How long the walk holds each step before its picture is taken, in milliseconds.
pub(crate) const HOLD_MS: &str = "400";

/// What playing a walk headless may take, picture by picture.
pub(crate) const PLAY_BUDGET: Duration = Duration::from_secs(150);

/// Where one Job's kept looks are.
pub(crate) fn root_of(home: &str, session: &str) -> PathBuf {
    crate::runtime::support_dir(home).join(DIR).join(session)
}

/// How a played walk ended: its exit code, and the last of what it printed on stderr.
pub(crate) struct Played {
    pub(crate) code: Option<i32>,
    pub(crate) stderr: String,
}

/// Play a walk with the binary's own arguments, in the folder the binary is kept in.
pub(crate) async fn played(binary: &Path, args: &[String]) -> Result<Played, String> {
    let run = tokio::process::Command::new(binary)
        .args(args)
        .current_dir(binary.parent().unwrap_or(Path::new("/")))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .output();
    match tokio::time::timeout(PLAY_BUDGET, run).await {
        Ok(Ok(out)) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let lines: Vec<&str> = stderr.lines().collect();
            Ok(Played { code: out.status.code(), stderr: lines[lines.len().saturating_sub(10)..].join("\n") })
        }
        Ok(Err(why)) => Err(format!("the native Bridge could not be started: {why}")),
        Err(_) => Err(format!("the walk did not finish within {} seconds", PLAY_BUDGET.as_secs())),
    }
}

/// How many of a build's last lines the Job is given.
const TAIL: usize = 20;

/// The last [`TAIL`] lines of what a failed build printed: cargo prints on stderr.
pub(crate) fn tail_of(stderr: &str, stdout: &str) -> String {
    let text = if stderr.trim().is_empty() { stdout } else { stderr };
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(TAIL)..].join("\n")
}

/// How many pictures a folder holds.
fn pictures_in(folder: &Path) -> usize {
    std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|one| one.path().extension().is_some_and(|ext| ext == "png"))
        .count()
}

/// One walk to take the pictures of: what his press named and the build it was Ready at.
struct Taking {
    session: String,
    walk: String,
    url: String,
    commit: String,
    binary: PathBuf,
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// His press on `text`, **read before the walk's row is ended**: where it is a native walk whose build
    /// is Ready, the pictures of the walk he approved are taken from that build. A press that names a take
    /// keeps that take's walk. Returns at once: the pictures are taken in a task of their own.
    pub(crate) async fn look_kept(self: &Arc<Self>, session: &str, text: &str) {
        use crate::session_host::walking::{take_pressed, walk_pressed};
        let Some((_, address)) = walk_pressed(text) else { return };
        let Some(page) = crate::native_walks::walk_name(address) else { return };
        let Some((SessionRow::Window { variations, build: Some(build), .. }, _)) = self.walk_row_of(session, address).await else { return };
        if build.state != WalkBuildState::Ready {
            return;
        }
        let walk = match take_pressed(text) {
            Some((letter, name)) => variations
                .iter()
                .find(|take| take.name == name)
                .or_else(|| variations.get((letter as usize).wrapping_sub('A' as usize)))
                .and_then(|take| crate::native_walks::walk_name(&take.url))
                .unwrap_or(page),
            None => page,
        }
        .to_string();
        let Ok((_, hosting)) = self.session_and_hosting(session).await else { return };
        let home = self.host().home.clone();
        let binary = crate::walk_builds::binary_of(&home, &crate::walk_builds::key_of(&hosting.manifest_id), &build.commit);
        let taking = Taking { session: session.to_string(), walk, url: address.to_string(), commit: build.commit, binary };
        let fleet = Arc::clone(self);
        tokio::spawn(async move { fleet.look_taken(taking).await });
    }

    /// Take the pictures into a folder of their own and move it in once it is whole, so the older look is
    /// never half replaced. A failure keeps nothing and is told to the Job.
    async fn look_taken(self: Arc<Self>, taking: Taking) {
        let short = taking.commit.get(..7).unwrap_or(&taking.commit).to_string();
        let home = self.host().home.clone();
        let folder = root_of(&home, &taking.session).join(&taking.walk);
        let staging = root_of(&home, &taking.session).join(format!("{}.taking", taking.walk));
        let made = self.look_played(&taking, &staging, &short).await;
        let made = made.and_then(|()| {
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::rename(&staging, &folder).map_err(|why| format!("{} could not be kept: {why}", folder.display()))
        });
        match made {
            Ok(()) => {
                let look = store::KeptLook {
                    session_id: taking.session.clone(),
                    walk: taking.walk.clone(),
                    url: taking.url.clone(),
                    commit: taking.commit.clone(),
                    at: self.now().as_str().to_string(),
                    folder: folder.display().to_string(),
                };
                let _ = self.store().lock().await.keep_look(&look);
            }
            Err(why) => {
                let _ = std::fs::remove_dir_all(&staging);
                let text = format!(
                    "The look he approved on {} could not be kept from the build of {short}: {why}. Nothing is kept, so the screen that lands is not checked against it. \
                     Show the walk again for a new approval.",
                    taking.url
                );
                let _ = self.told_by_fleet(&taking.session, &text, ipc::Notice::WalkFailed).await;
            }
        }
    }

    async fn look_played(&self, taking: &Taking, into: &Path, short: &str) -> Result<(), String> {
        if !taking.binary.is_file() {
            return Err(format!("the build of {short} is no longer kept"));
        }
        let _ = std::fs::remove_dir_all(into);
        std::fs::create_dir_all(into).map_err(|why| format!("{} could not be made: {why}", into.display()))?;
        let args: Vec<String> =
            ["--walk", &taking.walk, "--check", "--shots", &into.display().to_string(), "--hold", HOLD_MS].map(String::from).to_vec();
        let played = played(&taking.binary, &args).await?;
        if played.code != Some(0) {
            return Err(format!("the walk {} did not play (exit {:?}): {}", taking.walk, played.code, played.stderr));
        }
        if pictures_in(into) == 0 {
            return Err(format!("the walk {} made no pictures", taking.walk));
        }
        Ok(())
    }
}
