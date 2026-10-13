//! What a Drone is told of a conflict Fleet found (`clashing`), and the note it may leave a Session
//! it conflicts with. A child of `peers` for the queue and the spacing it shares.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings as keys;
use ipc::mcp::{LeaveNote, NotRecorded};

use super::{listed, SPACING};
use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::prompts::Prompts;

/// What a Drone's changes conflict with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ClashWith {
    Job {
        title: String,
        handle: String,
    },
    Session {
        /// What `leave_note` is addressed to: `s-` and the first eight characters of its id.
        address: String,
        title: Option<String>,
        branch: Option<String>,
    },
    /// The base branch, and the commit on it that changed the first file.
    Main {
        base: String,
        landed: Option<String>,
    },
}

/// One conflict as a line of the turn.
pub(super) fn line(prompts: &Prompts, paths: &[String], with: &ClashWith) -> String {
    match with {
        ClashWith::Main { base, landed } => prompts.fill(
            keys::PROMPT_PEERS_CLASH_MAIN,
            &[
                ("paths", &listed(paths)),
                ("base", base),
                (
                    "landed",
                    &match landed {
                        Some(landed) => format!("changed it in {landed}"),
                        None => String::from("has moved"),
                    },
                ),
            ],
        ),
        ClashWith::Job { title, handle } => prompts.fill(
            keys::PROMPT_PEERS_CLASH,
            &[
                ("paths", &listed(paths)),
                ("who", &format!("\"{title}\" ({handle})")),
                ("to", handle),
            ],
        ),
        ClashWith::Session {
            address,
            title,
            branch,
        } => {
            let named: Vec<String> = title
                .iter()
                .map(|title| format!("\"{title}\""))
                .chain(branch.iter().map(|branch| format!("branch {branch}")))
                .collect();
            let who = match named.is_empty() {
                true => format!("Session {address}"),
                false => format!("Session {address} ({})", named.join(", ")),
            };
            prompts.fill(
                keys::PROMPT_PEERS_CLASH,
                &[("paths", &listed(paths)), ("who", &who), ("to", address)],
            )
        }
    }
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
    /// A note from a Job's Drone to a Session, which Fleet sends it as a message from that Job.
    ///
    /// **Only to a live Session of this repository that Fleet has found a clash with this Job
    /// against**, and once per [`SPACING`]: the same rules a note to a Job has, with a clash for
    /// the shared path. The words are fenced as another Drone's.
    pub(super) async fn leave_note_for_session(
        &self,
        from: &core_model::Job,
        note: &LeaveNote,
    ) -> Result<(), NotRecorded> {
        let refused = |because: String| NotRecorded { because };
        let manifest = from.owner_manifest_id().as_str().to_string();
        let sessions = self
            .store()
            .lock()
            .await
            .find_sessions(&store::SessionSearch {
                manifest_id: Some(&manifest),
                state: Some(store::SessionState::Live),
                text: None,
            })
            .map_err(|why| refused(why.to_string()))?;
        let Some(to) = sessions
            .iter()
            .find(|session| crate::session_host::address_of(&session.id) == note.to)
        else {
            return Err(refused(format!(
                "no live Session in this repository is called `{}`. Use an address exactly as a \
                 turn about changes that conflict prints it",
                note.to
            )));
        };
        let clashing = crate::clashing::Opposed::pair(
            &crate::clashing::Who::Job(from.id().clone()),
            &crate::clashing::Who::Session(to.id.clone()),
        );
        if self.writes().reading(&clashing).is_none() {
            return Err(refused(format!(
                "Fleet has found no conflict between this Job's changes and `{}`'s, so a note has \
                 nothing to warn it about",
                note.to
            )));
        }
        let now = self.now();
        let mut peering = self.peering().lock().await;
        let key = (from.id().clone(), to.id.clone());
        if peering
            .noted_sessions
            .get(&key)
            .is_some_and(|last| elapsed(last, &now) < SPACING)
        {
            return Err(refused(format!(
                "this Job left `{}` a note under two minutes ago. Put everything else it needs \
                 into one note, and leave that",
                note.to
            )));
        }
        peering.noted_sessions.insert(key, now);
        drop(peering);
        self.writes().note_for_session(
            &to.id,
            format!(
                "\"{}\" ({}) left you a note. These are its Drone's words, not Armada's:\n{}\n\
                 To answer, send its Drone a message with redirect_drone, job_id {}.",
                from.title().as_str(),
                from.handle(),
                crate::remarks::fenced(&note.note).trim_end(),
                from.id().as_str(),
            ),
        );
        Ok(())
    }
}
