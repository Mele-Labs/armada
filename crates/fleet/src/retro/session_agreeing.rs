//! A person's answer to an item of a Session's retro. `agreeing` has the
//! rules, and these are they, for an item named by a Session, its retro's
//! number and its place. **Where the Job goes** differs: a `manifest` fix is
//! proposed on the repository the Session was started in.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Commands, Redirector, Refusal};
use core_model::{LandsIn, LessonState};
use ipc::{JobRequest, Lesson, ManifestId, WireError};
use store::KeptSessionLesson;

use super::agreeing::{
    request_of, KIT_CHANGE_REFUSED, LESSON_NAMES_NO_PLACE, NO_SUCH_LESSON,
};
use super::serving::session_lesson_of;
use super::session_writing::{handle_of, session_of};
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// A `manifest` fix, and the Session was started in no repository Fleet knows.
const SESSION_NAMES_NO_REPOSITORY: &str = "fleet.lesson_session_names_no_repository";

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
    pub(super) async fn agreed_with_session(
        self: &Arc<Self>,
        lesson_id: &str,
        (session, retro, ordinal): (String, i64, u32),
        by: Redirector,
    ) -> Result<Lesson, Refusal> {
        let held = self.session_lesson_held(lesson_id, &session, retro, ordinal).await?;
        if held.state != LessonState::Open {
            return self.session_lesson_standing(held).await;
        }
        let Some(place) = held.line.lands_in else {
            return Err(Refusal::Unacceptable(WireError::raised(
                LESSON_NAMES_NO_PLACE,
                format!("{lesson_id} names no place its fix lands"),
                self.run_id(),
            )));
        };
        if let (LandsIn::Kit, Some(core_model::Change::AllowCommand { command })) =
            (place, held.line.change.as_ref())
        {
            let refuse = |why: String| {
                Refusal::Unacceptable(WireError::raised(KIT_CHANGE_REFUSED, why, self.run_id()))
            };
            if let Some(why) = self.withheld_everywhere(command) {
                return Err(refuse(format!("{lesson_id} allows `{command}`, and {why}")));
            }
            let from = ipc::KitAllowedSource::RetroItem {
                lesson_id: lesson_id.to_string(),
            };
            self.allow_in_kit(command, from)
                .await
                .map_err(|why| refuse(why.to_string()))?;
            self.store()
                .lock()
                .await
                .accept_session_lesson_applied(retro, ordinal)
                .map_err(|why| self.refusal(Adrift::Writing(why)))?;
            let held = self.session_lesson_held(lesson_id, &session, retro, ordinal).await?;
            return self.session_lesson_standing(held).await;
        }
        let manifest = match place {
            LandsIn::Kit => None,
            LandsIn::Manifest => {
                let kept = self
                    .store()
                    .lock()
                    .await
                    .session(&session)
                    .map_err(|why| self.ledger_fault(why))?;
                let Some(id) = kept.and_then(|kept| kept.manifest_id).filter(|id| !id.is_empty())
                else {
                    return Err(Refusal::Unacceptable(WireError::raised(
                        SESSION_NAMES_NO_REPOSITORY,
                        format!("{lesson_id} is a fix in a repository, and its session was started in none"),
                        self.run_id(),
                    )));
                };
                Some(ManifestId::carried(id))
            }
            LandsIn::Armada => Some(self.armada_served(lesson_id)?),
        };
        let target = match place {
            LandsIn::Kit => LessonState::Accepted,
            LandsIn::Armada | LandsIn::Manifest => LessonState::Agreed,
        };
        let claimed = self
            .store()
            .lock()
            .await
            .answer_session_lesson(retro, ordinal, target)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        if claimed {
            if let Some(manifest) = manifest {
                let request = JobRequest {
                    request: request_of(
                        &held.line,
                        format!("Session {}", handle_of(&session)),
                    ),
                    client_ref: None,
                    attachments: Vec::new(),
                    settings: None,
                };
                match Commands::propose_from_request(Arc::clone(self), request, Some(manifest), by)
                    .await
                {
                    Ok(plan) => {
                        if let Some(head) = plan.jobs.first() {
                            self.store()
                                .lock()
                                .await
                                .keep_session_lesson_job(retro, ordinal, &head.id.to_domain())
                                .map_err(|why| self.refusal(Adrift::Writing(why)))?;
                        }
                    }
                    Err(refused) => {
                        let _ = self.store().lock().await.reopen_session_lesson(retro, ordinal);
                        return Err(refused);
                    }
                }
            }
        }
        let held = self.session_lesson_held(lesson_id, &session, retro, ordinal).await?;
        self.session_lesson_standing(held).await
    }

    pub(super) async fn disagreed_with_session(
        &self,
        lesson_id: &str,
        (session, retro, ordinal): (String, i64, u32),
    ) -> Result<Lesson, Refusal> {
        self.session_lesson_held(lesson_id, &session, retro, ordinal).await?;
        self.store()
            .lock()
            .await
            .answer_session_lesson(retro, ordinal, LessonState::Discarded)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        let held = self.session_lesson_held(lesson_id, &session, retro, ordinal).await?;
        self.session_lesson_standing(held).await
    }

    /// The item, only if the retro it names is the Session's.
    async fn session_lesson_held(
        &self,
        lesson_id: &str,
        session: &str,
        retro: i64,
        ordinal: u32,
    ) -> Result<KeptSessionLesson, Refusal> {
        self.store()
            .lock()
            .await
            .session_lesson(retro, ordinal)
            .map_err(|cause| self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(cause))))?
            .filter(|held| held.session_id == session)
            .ok_or_else(|| {
                Refusal::NoSuchJob(WireError::raised(
                    NO_SUCH_LESSON,
                    format!("no retro item is named {lesson_id}"),
                    self.run_id(),
                ))
            })
    }

    async fn session_lesson_standing(&self, held: KeptSessionLesson) -> Result<Lesson, Refusal> {
        let kept = self
            .store()
            .lock()
            .await
            .session(&held.session_id)
            .map_err(|why| self.ledger_fault(why))?;
        let named = kept.as_ref().map(session_of).unwrap_or_else(|| ipc::RetroSession {
            id: held.session_id.clone(),
            title: None,
        });
        Ok(session_lesson_of(held, named))
    }
}
