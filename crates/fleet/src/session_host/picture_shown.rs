//! The pictures a Job showed: what the Look `all_done` check counts (`ways_checking.rs`) and what
//! `get_lead_picture` names for the native done card. **One reading of the ledger serves both**, so a Job
//! the check let through always has a picture to lead with, unless it withdrew every one.
//! `docs/practices/protocol/how-he-works-with-jobs-and-a-question-offered-as-pictures.md`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{LeadPicture, WireError};
use store::{Holder, KeptAttachment};

use super::picture_keeping::served_address;
use super::showing_file::{media_type_in, PICTURES};
use super::sketching::{drawn_in, TARGET_PREFIX};
use crate::daemon::Fleet;

/// `get_lead_picture` for a Job that has shown no picture, or withdrew every one. A 404.
const NO_LEAD_PICTURE: &str = "fleet.no_lead_picture";

/// One picture a Job showed.
pub(super) struct Shown {
    /// When it was shown, as digits, so an instant written either way compares with another. The second is
    /// the order two sketches drawn in the same instant were.
    at: (String, u64),
    /// A sketch the Job withdrew, or a window it withdrew or the person dismissed. A file is never withdrawn.
    withdrawn: bool,
    picture: LeadPicture,
}

/// The digits of an instant: `2026-11-13T10:00:00.123Z` and `20261113T100000123Z` are the same time.
fn digits(instant: &str) -> String {
    instant.chars().filter(char::is_ascii_digit).collect()
}

/// Every picture among `held`: a sketch, a window, or a file shown that is an image. **A picture withdrawn
/// was shown, so it is here, marked.** `dismissed` is the ids the person dismissed, which is how a window
/// is known withdrawn (`walk:<url>`, as `withdraw_window` writes it).
pub(super) fn pictures_shown(session: &str, held: &[KeptAttachment], dismissed: &[String]) -> Vec<Shown> {
    held.iter()
        .filter(|one| one.kind == "artifact")
        .filter_map(|one| {
            let said = |key: &str| one.detail.get(key).map(String::as_str);
            match said("form")? {
                "sketch" => {
                    let title = said("title").unwrap_or_else(|| one.target.trim_start_matches(TARGET_PREFIX));
                    let (instant, n) = drawn_in(&one.detail, &one.changed_at);
                    Some(Shown {
                        at: (digits(&instant), n),
                        withdrawn: said(ipc::SKETCH_WITHDRAWN).is_some(),
                        picture: LeadPicture::Sketch { title: title.to_string() },
                    })
                }
                "window" => Some(Shown {
                    at: (digits(said("shown").unwrap_or(&one.changed_at)), 0),
                    withdrawn: dismissed.contains(&format!("walk:{}", one.target)),
                    picture: LeadPicture::Window { url: one.target.clone() },
                }),
                "file" => {
                    let shown = said("shown")?;
                    media_type_in(&PICTURES, &one.target)?;
                    Some(Shown {
                        at: (digits(shown), 0),
                        withdrawn: false,
                        picture: LeadPicture::Image { path: served_address(session, &one.target) },
                    })
                }
                _ => None,
            }
        })
        .collect()
}

/// The picture shown last that was not withdrawn.
pub(super) fn lead_of(shown: Vec<Shown>) -> Option<LeadPicture> {
    shown.into_iter().filter(|one| !one.withdrawn).max_by(|a, b| a.at.cmp(&b.at)).map(|one| one.picture)
}

/// The item `all_done` takes off his list with no verdict: the lead picture, where it is a sketch still put
/// to him. **The done card is then the one place it shows**, and his End the one press. A sketch the pending
/// plan names, or one beside a question of the Job's own still open, is not a result, so it still waits.
pub(super) fn result_sketch(store: &store::Store, id: &str) -> Result<Option<String>, store::WriteError> {
    let held = store.attachments_of(&Holder::session(id))?;
    let dismissed = store.dismissed_waiting(id)?;
    let Some(LeadPicture::Sketch { title }) = lead_of(pictures_shown(id, &held, &dismissed)) else {
        return Ok(None);
    };
    let target = format!("{TARGET_PREFIX}{title}");
    let asked = held.iter().any(|one| {
        let said = |key: &str| one.detail.get(key).map(String::as_str);
        one.kind == "artifact"
            && one.target == target
            && said(ipc::SKETCH_ASKED) == Some("true")
            && said(ipc::SKETCH_SETTLED).is_none()
            && said(ipc::SKETCH_WITHDRAWN).is_none()
    });
    let in_plan = store.session_plan(id)?.is_some_and(|plan| crate::plan_record::pending_sketches(&plan).contains(&title));
    let questioned = store.waiting_for(id)?.iter().any(|one| one.approves || !one.options.is_empty());
    Ok((asked && !dismissed.contains(&target) && !in_plan && !questioned).then_some(target))
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
    /// The picture the Job last showed. 404 `fleet.no_lead_picture` where it showed none, or withdrew them all.
    pub(crate) async fn lead_picture(&self, id: &str) -> Result<LeadPicture, Refusal> {
        let (held, dismissed) = {
            let store = self.store().lock().await;
            store.session(id).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| {
                Refusal::NotFound(WireError::raised(super::NO_SUCH_SESSION, format!("no job is named {id}"), self.run_id()))
            })?;
            let held = store.attachments_of(&Holder::session(id)).map_err(|why| self.ledger_fault(why))?;
            (held, store.dismissed_waiting(id).map_err(|why| self.ledger_fault(why))?)
        };
        lead_of(pictures_shown(id, &held, &dismissed)).ok_or_else(|| {
            Refusal::NotFound(WireError::raised(NO_LEAD_PICTURE, format!("job {id} has shown no picture that stands"), self.run_id()))
        })
    }
}
