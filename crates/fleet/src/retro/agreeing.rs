//! A person's answer to a retro item: agree or disagree.
//! `docs/concepts/retro.md`, *Agree and disagree*.
//!
//! **Agreeing is the one place a retro acts**, and it acts by starting a
//! Session on the item: that is `starting`. Disagreeing keeps the row and
//! lists it nowhere a person has not asked for.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{Lesson, ManifestId, WireError};

use crate::daemon::Fleet;

/// No item answers to that id. A 404.
pub(super) const NO_SUCH_LESSON: &str = "fleet.no_such_lesson";
/// An item written before 23.15, which names no place its fix lands. A 422:
/// where its Session would go is not Fleet's to guess.
pub(super) const LESSON_NAMES_NO_PLACE: &str = "fleet.lesson_names_no_place";
/// A fix that lands in Armada, and the repository Armada is built in is not
/// one this Fleet serves. A 422.
const ARMADA_NOT_SERVED: &str = "fleet.lesson_armada_not_served";

/// The Manifest id of the repository Armada itself is built in. **A name and
/// not a probe**: Fleet holds no marker for which repository it is, and a
/// machine that serves Armada serves it under this id.
pub(crate) const ARMADAS_OWN: &str = "armada";

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

    /// Disagree with an item. The row stays, `discarded`. **An item already
    /// answered answers with where it stands**: a Session started is never
    /// taken back by pressing this.
    pub(crate) async fn disagreed_with(&self, lesson_id: &str) -> Result<Lesson, Refusal> {
        let session = super::serving::parsed_session(lesson_id)
            .ok_or_else(|| self.no_such_lesson(lesson_id))?;
        self.disagreed_with_session(lesson_id, session).await
    }

    pub(super) fn no_such_lesson(&self, lesson_id: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(
            NO_SUCH_LESSON,
            format!("no retro item is named {lesson_id}"),
            self.run_id(),
        ))
    }

    /// Armada's own repository, where this Fleet serves it.
    pub(crate) fn armadas_own(&self) -> Option<ManifestId> {
        let own = ManifestId::carried(ARMADAS_OWN);
        self.served_named(Some(&own)).is_ok().then_some(own)
    }

    /// [`Self::armadas_own`], or the refusal for a fix in Armada.
    pub(super) fn armada_served(&self, lesson_id: &str) -> Result<ManifestId, Refusal> {
        self.armadas_own().ok_or_else(|| {
            Refusal::Unacceptable(WireError::raised(
                ARMADA_NOT_SERVED,
                format!(
                    "{lesson_id} is a fix in Armada, and this Fleet does not serve Armada's own \
                     repository (`{ARMADAS_OWN}`), so there is nowhere to start a Job for it. \
                     Serving it, then agreeing again, starts one"
                ),
                self.run_id(),
            ))
        })
    }
}

