//! `start_terminal_job`: a quick terminal, a Job that opens onto its terminal `t1` and whose agent
//! runs nothing until the person writes. `docs/concepts/session/a-quick-terminal.md`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ManifestId, StartSession, StartTerminalJob, TerminalJobStarted};

use super::{refusal, COLS, ROWS};
use crate::daemon::Fleet;
use crate::session_host::{Marked, QuickStart};

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
    /// The Job, started with no workflow, no hold and no proposer, and its `t1` opened. **No agent
    /// process starts**: the first message does, as for any hosted Job. Everything is checked
    /// before the Job is made.
    pub(crate) async fn started_quick_terminal(
        self: Arc<Self>,
        start: StartTerminalJob,
    ) -> Result<TerminalJobStarted, Refusal> {
        let (cols, rows) = (start.cols.unwrap_or(COLS), start.rows.unwrap_or(ROWS));
        if cols == 0 || rows == 0 {
            return Err(self.no_cells());
        }
        let dir = start.dir.as_deref().map(str::trim).filter(|dir| !dir.is_empty());
        let (manifest_id, quick) = match (start.manifest_id, dir) {
            (Some(_), Some(_)) | (None, None) => {
                return Err(refusal("a quick terminal takes one of `manifest_id` and `dir`", self.run_id()))
            }
            (Some(manifest_id), None) => {
                // An unknown Manifest is refused by the start below, as `start_session` refuses it.
                (manifest_id, QuickStart::default())
            }
            (None, Some(dir)) => self.quick_placed(dir)?,
        };
        let loose = quick.loose.is_some();
        // A title he did not type stays unset, so his first message names the Job as it does any other.
        let title = start.title.map(|title| title.trim().to_string()).filter(|title| !title.is_empty());
        let begun = StartSession {
            manifest_id,
            title,
            model: start.model.or_else(|| self.quick_model()),
            effort: start.effort,
            mode: start.mode,
            fork: None,
            root: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
            project: start.project.filter(|_| !loose),
        };
        let record = self.started_hosted_as(begun, Marked::Plain, Some(quick)).await?;
        let (terminal, run_id) = Arc::clone(&self)
            .open_job_terminal(record.id.as_str(), None, false, Some(cols), Some(rows))
            .await?;
        Ok(TerminalJobStarted { session_id: record.id, terminal, run_id })
    }

    /// The model a quick terminal that names none starts on: **the Session default, never below
    /// Sonnet**. Its agent answers under every rule a Job carries, which a Haiku turn lost track of on
    /// 13 Nov 2026. `None` leaves the default to be read when the process starts.
    pub(crate) fn quick_model(&self) -> Option<String> {
        self.session_model().to_ascii_lowercase().contains("haiku").then(|| String::from("sonnet"))
    }

    /// Where `dir` puts the Job: the served repository that holds it, the deepest where one is
    /// inside another, with its terminals in `dir`; else a loose folder.
    fn quick_placed(&self, dir: &str) -> Result<(ManifestId, QuickStart), Refusal> {
        if !Path::new(dir).is_absolute() {
            return Err(refusal(format!("{dir} is not an absolute path"), self.run_id()));
        }
        let at = std::fs::canonicalize(dir)
            .ok()
            .filter(|at| at.is_dir())
            .ok_or_else(|| refusal(format!("{dir} is not a directory"), self.run_id()))?;
        let holding = self
            .repository_list()
            .repositories
            .into_iter()
            .filter_map(|one| {
                let id = one.manifest.map(|held| held.id).or(one.implicit_id)?;
                let root = std::fs::canonicalize(&one.root).unwrap_or_else(|_| PathBuf::from(&one.root));
                let under = at.strip_prefix(&root).ok()?.to_path_buf();
                Some((root.components().count(), id, under))
            })
            .max_by_key(|(depth, _, _)| *depth);
        let folder = at.to_string_lossy().into_owned();
        Ok(match holding {
            Some((_, id, under)) => {
                let under = under.to_string_lossy().into_owned();
                (id, QuickStart { loose: None, terminal_dir: (!under.is_empty()).then_some(under) })
            }
            None => (ManifestId::carried(""), QuickStart { loose: Some(folder.clone()), terminal_dir: None }),
        })
    }
}
