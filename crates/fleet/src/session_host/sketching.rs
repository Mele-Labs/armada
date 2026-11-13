//! `show_sketch`: a session draws the person a diagram, which is kept on its ledger.
//! `docs/concepts/session.md`, *Drawing a sketch*.
//!
//! **An artifact of form `sketch`, keyed by its title.** A redraw under the same title upserts the
//! one row, so each scene is also kept as a revision in Fleet's attachments folder, as a document's
//! page is, and the row's `revisions` names them: the person can still read the drawing it replaced.
//! The scene is checked before anything is written: Bridge draws one that fails as an empty frame
//! and says nothing, so this is where the agent hears of it.
//!
//! **Every sketch shown is put to the person on its card**, and Fleet raises it in his Needs you as
//! `sketch:<title>` ([`sketch_items`]), never the agent's `waiting_for`: `answer_sketch` approves it or
//! sends it back, and a redraw of the title asks again.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{
    AnswerSketch, AttachmentReport, SendSessionMessage, SessionFact, SessionId, SessionRecord, SessionReport, SessionRow,
    SessionVoice, ShowSketch, Sketch, SketchHow, WaitingAct, WaitingActKind, WaitingItem, WaitingSource, WireError, SKETCH_ASKED, SKETCH_DRAWN, SKETCH_SETTLED, SKETCH_SETTLED_AT,
    SKETCH_WITHDRAWN, SKETCH_WORDS,
};

use super::artifact_for::with_for;
use super::documents::{keep, one_name, revisions_in};
use crate::daemon::Fleet;

pub(crate) const SKETCH_REFUSED: &str = "fleet.sketch_refused";
const SKETCH_UNNAMED: &str = "fleet.sketch_unnamed";
const SKETCH_AGAINST_UNKNOWN: &str = "fleet.sketch_against_unknown";
const SKETCH_NOT_OPEN: &str = "fleet.sketch_not_open";
const SKETCH_REDRAWN: &str = "fleet.sketch_redrawn";
const SKETCH_ANSWER_EMPTY: &str = "fleet.sketch_answer_empty";
const SKETCH_WITHDRAW_UNNAMED: &str = "fleet.sketch_withdraw_unnamed";
const SKETCH_WITHDRAW_NOT_SHOWN: &str = "fleet.sketch_withdraw_not_shown";
const SKETCH_ANSWER_UNNAMED: &str = "fleet.sketch_answer_unnamed";

/// What a sketch's ledger row is keyed by, apart from a file or a page an artifact may be.
pub(crate) const TARGET_PREFIX: &str = "sketch:";

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
    pub(crate) async fn sketch_shown(
        &self,
        caller: Option<api::Caller>,
        show: ShowSketch,
    ) -> Result<SessionRecord, Refusal> {
        let sketch = show
            .checked()
            .map_err(|why| self.hosted_refusal(SKETCH_REFUSED, &why))?;
        let against = show
            .against_title()
            .map_err(|why| self.hosted_refusal(SKETCH_REFUSED, &why))?;
        let said = ipc::ArtifactFor::checked(show.for_line.as_deref(), &show.links)
            .map_err(|why| self.hosted_refusal(SKETCH_REFUSED, &why))?;
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| show.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                self.hosted_refusal(
                    SKETCH_UNNAMED,
                    "a sketch needs a job to draw it for, and this call places none",
                )
            })?;
        let (harness, held) = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SKETCH_UNNAMED, "a job Fleet knows"))?;
            let held = self.ledger_row(&store, &session)?.attachments;
            (session.harness, held)
        };
        // Kept before `against` is read, so the earlier scene's `kept` rides in the size the cap counts.
        let sketch = self.sketch_styled(&id, sketch, "show_sketch").await?;
        let sketch = match against {
            None => sketch,
            Some(title) => {
                let target = format!("{TARGET_PREFIX}{title}");
                let is_sketch = |one: &&ipc::Attachment| {
                    one.kind == "artifact" && one.detail.get("form").map(String::as_str) == Some("sketch")
                };
                let earlier = held.iter().filter(is_sketch).find(|one| one.target == target);
                let Some(earlier) = earlier.and_then(|one| one.detail.get("scene")) else {
                    let titles: Vec<&str> = held
                        .iter()
                        .filter(is_sketch)
                        .filter_map(|one| one.detail.get("title").map(String::as_str))
                        .take(20)
                        .collect();
                    let held = match titles.is_empty() {
                        true => String::from("none"),
                        false => titles.join(", "),
                    };
                    return Err(self.hosted_refusal(
                        SKETCH_AGAINST_UNKNOWN,
                        &format!(
                            "this job holds no sketch titled `{title}` to read this one against. \
                             Sketches it holds: {held}. Draw that one first, or leave `against` out"
                        ),
                    ));
                };
                sketch
                    .read_against(&title, earlier)
                    .map_err(|why| self.hosted_refusal(SKETCH_REFUSED, &why))?
            }
        };
        let revisions = self.scene_kept(&id, &sketch).await;
        let mut attachment = sketch_attachment(sketch);
        attachment.detail.insert(SKETCH_ASKED.into(), "true".into());
        attachment.detail.insert(SKETCH_DRAWN.into(), self.row_id(&id));
        if let Some(revisions) = revisions {
            attachment.detail.insert("revisions".into(), revisions);
        }
        with_for(&mut attachment.detail, self.for_line(&id, said).await.as_ref());
        // A sketch he dismissed waits again once the job draws it again, as a walk shown again does.
        let target = attachment.target.clone();
        self.store().lock().await.restore_waiting(&id, &target).map_err(|why| self.ledger_fault(why))?;
        self.report_session(SessionReport {
            harness,
            session_id: SessionId::carried(&id),
            fact: SessionFact::Attached { attachment },
        })
        .await
    }

    pub(crate) async fn sketch_withdrawn(
        &self,
        caller: Option<api::Caller>,
        withdraw: ipc::WithdrawSketch,
    ) -> Result<SessionRecord, Refusal> {
        let title = withdraw.title.trim().to_string();
        let target = format!("{TARGET_PREFIX}{title}");
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| withdraw.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(SKETCH_WITHDRAW_UNNAMED, "a job to withdraw it for"))?;
        let (_, held) = self.sketch_held(&id, &target).await?;
        if held.is_none() {
            return Err(self.hosted_refusal(
                SKETCH_WITHDRAW_NOT_SHOWN,
                &format!("this job showed no sketch titled `{title}`. Name the title you passed to show_sketch"),
            ));
        }
        // The dismissal the person's own Withdraw writes, so both end the same way.
        self.store().lock().await.dismiss_waiting(&id, &target).map_err(|why| self.ledger_fault(why))?;
        self.sketch_withdrawn_mark(&id, &target, true).await;
        self.published_hosted(&id).await
    }

    /// Mark the sketch held as `target` withdrawn. **The one place the mark is written**: the job's
    /// `withdraw_sketch` and the person's Withdraw both come here, and `agent` is the job's, which also
    /// writes `withdrawn_by`, so the artifact reads `withdrawn` and his reads `dismissed`. A new drawing of
    /// the title replaces the attachment, so it does not carry the mark. A sketch the job does not hold is left alone.
    pub(crate) async fn sketch_withdrawn_mark(&self, id: &str, target: &str, agent: bool) {
        let Ok((harness, Some(mut sketch))) = self.sketch_held(id, target).await else { return };
        sketch.detail.insert(SKETCH_WITHDRAWN.into(), "true".into());
        if agent {
            sketch.detail.insert(ipc::ARTIFACT_WITHDRAWN_BY.into(), "agent".into());
        }
        let _ = self
            .report_session(SessionReport {
                harness,
                session_id: SessionId::carried(id),
                fact: SessionFact::Attached {
                    attachment: AttachmentReport { kind: sketch.kind, target: sketch.target, detail: sketch.detail },
                },
            })
            .await;
    }

    /// Keep `sketch`'s scene as the next revision of its title, unless the newest holds it already, and
    /// answer every revision as the encoded list of paths, oldest first. `None` where keeping failed:
    /// the sketch is still drawn, only without its history.
    async fn scene_kept(&self, id: &str, sketch: &Sketch) -> Option<String> {
        let folder = Path::new(&self.host().attachments_dir).join("sketches").join(id).join(titled_folder(&sketch.title));
        revisions_kept(folder, sketch.scene.clone().into_bytes(), "json").await
    }
}

/// `title` as one folder name. The digest of the exact title keeps "a b" and "a_b" apart, and a title of
/// `..` out of the parent.
pub(crate) fn titled_folder(title: &str) -> String {
    let digest: String = Sha256::digest(title.as_bytes()).iter().take(6).map(|byte| format!("{byte:02x}")).collect();
    format!("{}-{digest}", one_name(title))
}

/// Keep `bytes` as the next revision in `folder` unless the newest holds them, and answer every revision as
/// the encoded list of paths, oldest first. `None` where keeping failed.
pub(crate) async fn revisions_kept(folder: PathBuf, bytes: Vec<u8>, ext: &'static str) -> Option<String> {
    let kept = tokio::task::spawn_blocking(move || {
        Ok::<_, std::io::Error>(keep(&folder, &bytes, ext)?.unwrap_or_else(|| revisions_in(&folder, ext)))
    })
    .await;
    let Ok(Ok(revisions)) = kept else { return None };
    let names: Vec<String> = revisions.iter().map(|one| one.to_string_lossy().into_owned()).collect();
    ipc::encode(&names).ok()
}

/// The first words of the person's approval, which [`sketch_approved`] reads it back by.
pub(crate) const APPROVED: &str = "Approved sketch: ";

/// The person's message for each answer to a sketch. Words with an approval go on a line after it.
pub(crate) fn approved(title: &str, words: Option<&str>) -> String {
    let mut body = format!("{APPROVED}{title}\nGo on from it as drawn.");
    if let Some(words) = words {
        body.push('\n');
        body.push_str(words);
    }
    body
}

pub(crate) fn sent_back(title: &str, note: &str) -> String {
    format!("Sent back sketch: {title}\n{note}\nDraw it again with show_sketch under the same title.")
}

/// When the person last approved any sketch, or `None` while he has not. **Read off the thread**, as
/// `walk_approval` reads a walk's: his message [`approved`] wrote, which stays when the sketch is redrawn.
pub(crate) fn sketch_approved(rows: &[SessionRow]) -> Option<&str> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { at, from: SessionVoice::You, text, .. } if text.starts_with(APPROVED) => Some(at),
            _ => None,
        })
        .max()
        .map(ipc::Instant::as_str)
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
    /// `answer_sketch`: the person approves a sketch, or sends it back with a note. The job is told as
    /// his message first, so a failed send leaves the sketch open, then `detail` keeps how it ended.
    pub(crate) async fn sketch_answered(self: Arc<Self>, said: AnswerSketch) -> Result<SessionRecord, Refusal> {
        let id = said.session_id.as_str().trim().to_string();
        let target = said.sketch.trim().to_string();
        let words = said.words.as_deref().map(str::trim).filter(|words| !words.is_empty());
        if said.how == SketchHow::SentBack && words.is_none() {
            return Err(self.hosted_refusal(SKETCH_ANSWER_EMPTY, "a sketch is sent back with a note saying what to change"));
        }
        let terminal = self.terminal_session(&id).await?;
        let not_open = |why: String| self.hosted_refusal(SKETCH_NOT_OPEN, &why);
        // Held as a draft's answer is, so two presses cannot both settle it.
        let _settling = self
            .hosts()
            .settling(&id, &target)
            .ok_or_else(|| not_open(format!("the sketch `{target}` is being answered already")))?;
        let (harness, held) = self.sketch_held(&id, &target).await?;
        let sketch = held.ok_or_else(|| not_open(format!("this job holds no sketch `{target}`")))?;
        let title = sketch.detail.get("title").cloned().unwrap_or_else(|| target.trim_start_matches(TARGET_PREFIX).to_string());
        if sketch.detail.get(SKETCH_ASKED).map(String::as_str) != Some("true") {
            return Err(not_open(format!("the sketch `{title}` was not put to the person: a plan's is approved with the plan")));
        }
        if sketch.detail.get(SKETCH_DRAWN) != Some(&said.drawn) {
            return Err(self.redrawn(&title));
        }
        if let Some(how) = sketch.detail.get(SKETCH_SETTLED) {
            return Err(not_open(format!("the sketch `{title}` was {} already", how.replace('_', " "))));
        }
        let body = match said.how {
            SketchHow::Approved => approved(&title, words),
            SketchHow::SentBack => sent_back(&title, words.unwrap_or_default()),
        };
        let message = SendSessionMessage {
            session_id: said.session_id.clone(),
            text: body,
            attachments: Vec::new(),
            mentions: Vec::new(),
            sketches: Vec::new(),
        };
        Arc::clone(&self).send_known(terminal, message).await?;
        // Read again after the send: a drawing the job made meanwhile is not the one he answered.
        let (_, now) = self.sketch_held(&id, &target).await?;
        let Some(mut sketch) = now.filter(|one| one.detail.get(SKETCH_DRAWN) == Some(&said.drawn)) else {
            return self.published_hosted(&id).await;
        };
        sketch.detail.insert(SKETCH_SETTLED.into(), said.how.word().into());
        sketch.detail.insert(SKETCH_SETTLED_AT.into(), self.instant().as_str().to_string());
        if let Some(words) = words {
            sketch.detail.insert(SKETCH_WORDS.into(), words.to_string());
        }
        self.report_session(SessionReport {
            harness,
            session_id: SessionId::carried(&id),
            fact: SessionFact::Attached {
                attachment: AttachmentReport { kind: sketch.kind, target: sketch.target, detail: sketch.detail },
            },
        })
        .await
    }

    /// The job's harness, and the sketch it holds as `target`.
    async fn sketch_held(&self, id: &str, target: &str) -> Result<(String, Option<ipc::Attachment>), Refusal> {
        let store = self.store().lock().await;
        let session = store
            .session(id)
            .map_err(|why| self.ledger_fault(why))?
            .ok_or_else(|| self.hosted_refusal(SKETCH_ANSWER_UNNAMED, "a job Fleet knows"))?;
        let held = self
            .ledger_row(&store, &session)?
            .attachments
            .into_iter()
            .find(|one| one.kind == "artifact" && one.target == target && one.detail.get("form").map(String::as_str) == Some("sketch"));
        Ok((session.harness, held))
    }

    fn redrawn(&self, title: &str) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            SKETCH_REDRAWN,
            format!("the sketch `{title}` is not the drawing that was read: the job has drawn it again since"),
            self.run_id(),
        ))
    }
}

/// A sketch as the ledger keeps it: the artifact a title replaces where it stands. A plan's sketch is
/// kept the same way, so the scene is on the ledger once and the plan names it.
pub(crate) fn sketch_attachment(sketch: Sketch) -> AttachmentReport {
    AttachmentReport {
        kind: String::from("artifact"),
        target: format!("{TARGET_PREFIX}{}", sketch.title),
        detail: [
            ("form".to_string(), "sketch".to_string()),
            ("by".to_string(), "agent".to_string()),
            ("title".to_string(), sketch.title),
            ("scene".to_string(), sketch.scene),
        ]
        .into_iter()
        .chain(sketch.against.map(|kept| ("against".to_string(), kept)))
        .collect(),
    }
}

/// When a sketch was last drawn, as its `drawn` row id carries it (`<job>-<instant>-<n>`), so answering
/// it or taking it back, which change the row, do not make it the newest. One with no `drawn` reads its
/// `changed_at`, written the way a row id writes an instant.
fn drawn_at(sketch: &ipc::Attachment) -> (String, u64) {
    drawn_in(&sketch.detail, sketch.changed_at.as_str())
}

/// [`drawn_at`] of a sketch's detail and `changed_at`, so the ledger's own rows read it the same way.
pub(super) fn drawn_in(detail: &std::collections::BTreeMap<String, String>, changed_at: &str) -> (String, u64) {
    let drawn = detail.get(SKETCH_DRAWN).and_then(|id| {
        let mut parts = id.rsplitn(3, '-');
        let n = parts.next()?.parse().ok()?;
        Some((parts.next()?.to_string(), n))
    });
    drawn.unwrap_or_else(|| (changed_at.replace([':', '-', '.'], ""), 0))
}

/// Every sketch put to the person, answered or not.
fn asked_sketches(attachments: &[ipc::Attachment]) -> impl Iterator<Item = &ipc::Attachment> {
    attachments
        .iter()
        .filter(|one| one.kind == "artifact" && one.detail.get("form").map(String::as_str) == Some("sketch"))
        .filter(|one| one.detail.get(SKETCH_ASKED).map(String::as_str) == Some("true"))
}

/// Whether a sketch put to him is neither answered nor taken back.
fn is_open(sketch: &ipc::Attachment) -> bool {
    !sketch.detail.contains_key(SKETCH_SETTLED) && !sketch.detail.contains_key(SKETCH_WITHDRAWN)
}

/// Every sketch put to the person and neither answered nor taken back.
fn open_sketches(attachments: &[ipc::Attachment]) -> impl Iterator<Item = &ipc::Attachment> {
    asked_sketches(attachments).filter(|one| is_open(one))
}

/// Every sketch put to the person and not answered or taken back, newest drawn first, as the items that
/// wait on him: id the sketch's target, text its title, opening the Session. A plan's sketch is approved
/// with the plan, so it is never one of these. The first is the thread head's pin, so an answered
/// sketch never stays it.
pub(crate) fn sketch_items<'a>(attachments: &'a [ipc::Attachment], session: &'a str) -> impl Iterator<Item = WaitingItem> + 'a {
    let mut open = open_sketches(attachments).collect::<Vec<_>>();
    open.sort_by_key(|one| std::cmp::Reverse(drawn_at(one)));
    open.into_iter().map(move |one| WaitingItem {
            approves: false,
            id: one.target.clone(),
            text: one.detail.get("title").cloned().unwrap_or_else(|| one.target.trim_start_matches(TARGET_PREFIX).to_string()),
            since: one.changed_at.clone(),
            source: WaitingSource::Sketch,
            act: Some(WaitingAct { kind: WaitingActKind::Message, target: session.to_string() }),
            options: Vec::new(),
            steps: Vec::new(),
            place: None,
            confirmed: false,
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
            operator_read: None,
        })
}
