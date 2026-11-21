//! `show_sketch`: a session draws the person a diagram, which is kept on its ledger.
//! `docs/concepts/session.md`, *Drawing a sketch*.
//!
//! **An artifact of form `sketch`, keyed by its title.** A redraw under the same title upserts the
//! one row, so each scene is also kept as a revision in Fleet's attachments folder, as a document's
//! page is, and the row's `revisions` names them: the person can still read the drawing it replaced.
//! The scene is checked before anything is written: Bridge draws one that fails as an empty frame
//! and says nothing, so this is where the agent hears of it.
//!
//! **Every sketch shown is put to the person on its card unless it was shown with `asks: false`**, and Fleet
//! raises it in his Needs you as `sketch:<title>` ([`sketch_items`]), never the agent's `waiting_for`:
//! `answer_sketch` approves it or sends it back, and a redraw of the title asks again. A sketch that asks
//! nothing never waits on him, and a redraw of its title asks nothing either.

use std::collections::BTreeSet;
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
pub(crate) const SKETCH_TAKES: &str = "fleet.sketch_takes";
/// On a sketch row: the question its drawing puts to him, which titles his item.
pub(crate) const SKETCH_QUESTION: &str = "question";

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
        if let Some(why) = takes_refused(&sketch.scene) {
            return Err(self.hosted_refusal(SKETCH_TAKES, &format!("the sketch was not drawn: {why}")));
        }
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
        let (harness, held, in_plan) = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SKETCH_UNNAMED, "a job Fleet knows"))?;
            let record = self.ledger_row(&store, &session)?;
            let in_plan = in_pending_plan(&record);
            (session.harness, record.attachments, in_plan)
        };
        // Kept before `against` is read, so the earlier scene's `kept` rides in the size the cap counts.
        let sketch = self.sketch_styled(&id, sketch, "show_sketch").await?;
        let by_title = |wanted: &str| held.iter().filter(|one| is_artifact_sketch(one)).find(|one| titled_as(one, wanted));
        // A drawing shown `against` an earlier title is that artifact's next revision, retitled: his list never holds both.
        let earlier = match &against {
            None => None,
            Some(title) => match by_title(title) {
                Some(earlier) => Some(earlier),
                None => {
                    let titles: Vec<&str> = held
                        .iter()
                        .filter(|one| is_artifact_sketch(one))
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
                }
            },
        };
        let sketch = match (&against, earlier.and_then(|one| one.detail.get("scene"))) {
            (Some(title), Some(scene)) => sketch.read_against(title, scene).map_err(|why| self.hosted_refusal(SKETCH_REFUSED, &why))?,
            _ => sketch,
        };
        // Every ask is a question in plain words, which titles his item. One the pending plan names asks with the plan.
        let question = match show.asks && !in_plan.contains(&sketch.title) {
            true => Some(ipc::question_checked(show.question.as_deref(), "`question`").map_err(|why| self.hosted_refusal(SKETCH_REFUSED, &format!("the sketch was not drawn: {why}")))?),
            false => show.question.as_deref().and_then(|said| ipc::question_checked(Some(said), "`question`").ok()),
        };
        let existing = earlier.or_else(|| by_title(&sketch.title));
        if let Some(why) = existing.and_then(|one| super::artifact_parts::drawing_refused(&held, one, show.asks)) {
            return Err(self.hosted_refusal(SKETCH_TAKES, &format!("the sketch was not drawn: {why}")));
        }
        let target = existing.map_or_else(|| format!("{TARGET_PREFIX}{}", sketch.title), |one| one.target.clone());
        // A sketch that asks nothing is no part of an artifact that does, so a redraw of one that joined leaves it.
        let kept_keys: &[&str] = if show.asks { &[ipc::ARTIFACT_IN, ipc::ARTIFACT_PARTS] } else { &[ipc::ARTIFACT_PARTS] };
        let carried: Vec<(String, String)> = existing
            .map(|one| {
                kept_keys
                    .iter()
                    .filter_map(|key| Some((key.to_string(), one.detail.get(*key)?.clone())))
                    .collect()
            })
            .unwrap_or_default();
        let revisions = self.scene_kept(&id, target.trim_start_matches(TARGET_PREFIX), &sketch).await;
        let mut attachment = sketch_attachment(sketch);
        attachment.target = target;
        attachment.detail.extend(carried);
        if show.asks {
            attachment.detail.insert(SKETCH_ASKED.into(), "true".into());
        }
        if let Some(question) = question {
            attachment.detail.insert(SKETCH_QUESTION.into(), question);
        }
        attachment.detail.insert(SKETCH_DRAWN.into(), self.row_id(&id));
        if let Some(revisions) = revisions {
            attachment.detail.insert("revisions".into(), revisions);
        }
        with_for(&mut attachment.detail, self.for_line(&id, said).await.as_ref());
        ipc::feature_in_detail(&mut attachment.detail, show.feature.as_ref());
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
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| withdraw.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(SKETCH_WITHDRAW_UNNAMED, "a job to withdraw it for"))?;
        let rows = self.sketch_rows(&id).await?;
        let found = rows.iter().find(|one| titled_as(one, &title));
        let target = found.map_or_else(|| format!("{TARGET_PREFIX}{title}"), |one| one.target.clone());
        let (_, held) = self.sketch_held(&id, &target).await?;
        if held.is_none() {
            return Err(self.hosted_refusal(
                SKETCH_WITHDRAW_NOT_SHOWN,
                &format!("this job showed no sketch titled `{title}`. Name the title you passed to show_sketch"),
            ));
        }
        // The dismissal the person's own Withdraw writes, so both end the same way. An artifact that holds other
        // parts is not dismissed whole: only this drawing is taken back.
        let others = held.as_ref().is_some_and(|one| !super::artifact_parts::stored(&one.detail).is_empty())
            || rows.iter().any(|one| one.detail.get(ipc::ARTIFACT_IN) == Some(&target))
            || super::artifact_parts::holds_members(&rows, &target);
        if !others {
            self.store().lock().await.dismiss_waiting(&id, &target).map_err(|why| self.ledger_fault(why))?;
        }
        self.sketch_withdrawn_mark(&id, &target, true).await;
        self.published_hosted(&id).await
    }

    /// Every artifact row the Job holds, as the ledger keeps them.
    async fn sketch_rows(&self, id: &str) -> Result<Vec<ipc::Attachment>, Refusal> {
        let store = self.store().lock().await;
        let session = store
            .session(id)
            .map_err(|why| self.ledger_fault(why))?
            .ok_or_else(|| self.hosted_refusal(SKETCH_ANSWER_UNNAMED, "a job Fleet knows"))?;
        Ok(self.ledger_row(&store, &session)?.attachments)
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
    async fn scene_kept(&self, id: &str, title: &str, sketch: &Sketch) -> Option<String> {
        let folder = Path::new(&self.host().attachments_dir).join("sketches").join(id).join(titled_folder(title));
        revisions_kept(folder, sketch.scene.clone().into_bytes(), "json").await
    }
}

/// A sketch row, which is what an artifact is kept as.
fn is_artifact_sketch(one: &ipc::Attachment) -> bool {
    one.kind == "artifact" && one.detail.get("form").map(String::as_str) == Some("sketch")
}

/// Whether the sketch row is the one `wanted` names: by the title it was first kept under, or the one it has now.
fn titled_as(one: &ipc::Attachment, wanted: &str) -> bool {
    one.target.strip_prefix(TARGET_PREFIX) == Some(wanted) || one.detail.get("title").map(String::as_str) == Some(wanted)
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

/// His message when his note only picks one of the sketch's takes. It is an approval, so the rail and a
/// `sketch_approved` gate read it as one.
pub(crate) fn picked(title: &str, take: &str) -> String {
    format!("{APPROVED}{title}\nHe picked take {take}. Go on from take {take} as drawn, and drop the other takes.")
}

/// The take `note` picks, where the note is nothing but the name of one take the scene draws, such as
/// `B` or `Take 2`; `None` for any other note. A take is named by the leading words of a mock's, a
/// screen's or a label's title (`Take B: …`, `B · …`, `A (my pick): …`) or by a mock's or screen's id,
/// and only a scene of two or more mocks or screens has takes.
pub(crate) fn take_picked(scene: &str, note: &str) -> Option<String> {
    let scene: ipc::Scene = ipc::decode("a sketch's scene", scene.as_bytes()).ok()?;
    let pictures = |node: &&ipc::SceneNode| is_picture(node);
    if scene.nodes.iter().filter(pictures).count() < 2 {
        return None;
    }
    let wanted = take_name(note);
    let mut names = Vec::new();
    for node in &scene.nodes {
        let titled = match node.kind {
            ipc::SceneKind::Label => node.title.as_deref().or(node.body.as_deref()),
            _ if pictures(&node) => node.title.as_deref(),
            _ => None,
        };
        if let Some(title) = titled {
            let lead = title.split([',', ':', '·', '(', '—', '–']).next().unwrap_or_default();
            let lead = lead.split(". ").next().unwrap_or_default().trim().trim_end_matches('.');
            let lead = take_name(lead);
            if let Some(first) = lead.split_whitespace().next().filter(|first| first.chars().count() <= 2) {
                names.push(first.to_string());
            }
            names.push(lead);
        }
        if pictures(&node) {
            names.push(take_name(&node.id));
        }
    }
    let found = names.iter().find(|name| !name.is_empty() && **name == wanted)?;
    Some(found.to_uppercase().chars().take(1).chain(found.chars().skip(1)).collect())
}

/// `said` lowercased, with the words that only introduce a take's name taken off: `Take B.` reads `b`.
fn take_name(said: &str) -> String {
    let said = said.trim().trim_matches(['"', '\'', '`', '*']).trim_end_matches(['.', '!']).trim().to_lowercase();
    let mut said = said.as_str();
    for lead in TAKE_LEADS {
        said = said.strip_prefix(lead).unwrap_or(said).trim_start();
    }
    said.to_string()
}

/// The words that only introduce a take's name, lowercased.
const TAKE_LEADS: [&str; 5] = ["i pick ", "pick ", "go with ", "take ", "option "];

/// A mock or a screen: what a take is drawn as. Boxes and arrows draw no takes.
fn is_picture(node: &ipc::SceneNode) -> bool {
    matches!(node.kind, ipc::SceneKind::Mock | ipc::SceneKind::Screen)
}

/// How far above a picture's top, in scene units, a label still names it.
const LABEL_REACH: f64 = 200.0;

/// Why `scene` is refused as takes drawn side by side for Approve to stand in for a pick, or `None`. **Two or
/// more mocks or screens are takes** when two of them lead with different take letters (`A · …`, `B: …`), by
/// their own title or a label's just above them, or one says `(my pick)`. His ruling of 14 Nov: takes are a
/// vote, so they go in a `takes` part he answers with a pick.
pub(crate) fn takes_refused(scene: &str) -> Option<String> {
    let scene: ipc::Scene = ipc::decode("a sketch's scene", scene.as_bytes()).ok()?;
    let pictures: Vec<&ipc::SceneNode> = scene.nodes.iter().filter(|node| is_picture(node)).collect();
    if pictures.len() < 2 {
        return None;
    }
    let labels: Vec<&ipc::SceneNode> = scene.nodes.iter().filter(|node| matches!(node.kind, ipc::SceneKind::Label)).collect();
    let mut letters = BTreeSet::new();
    let mut picked = false;
    let mut titles = Vec::new();
    for picture in pictures {
        let wide = picture.w.unwrap_or(240.0);
        let above = labels
            .iter()
            .filter(|label| label.y < picture.y && picture.y - label.y <= LABEL_REACH && label.x >= picture.x - 40.0 && label.x < picture.x + wide)
            .max_by(|one, two| one.y.total_cmp(&two.y))
            .and_then(|label| label.title.as_deref().or(label.body.as_deref()));
        for title in [picture.title.as_deref(), above].into_iter().flatten() {
            let letter = take_letter(title);
            let mine = title.to_lowercase().contains("(my pick)");
            if letter.is_some() || mine {
                letters.extend(letter);
                picked |= mine;
                titles.push(format!("`{}`", title.trim()));
                break;
            }
        }
    }
    if letters.len() < 2 && !picked {
        return None;
    }
    Some(format!(
        "{} are takes, so put them to him with add_part, `kind` `takes`, each take a scene, your pick first, and he picks one with Send pick; Approve never stands in for a pick",
        titles.join(", ")
    ))
}

/// The take letter `title` leads with, A to F, past any word that only introduces it: `B · Tabs`, `Take C: …`.
fn take_letter(title: &str) -> Option<char> {
    let mut rest = title.trim();
    for lead in TAKE_LEADS {
        if rest.get(..lead.len()).is_some_and(|start| start.eq_ignore_ascii_case(lead)) {
            rest = rest[lead.len()..].trim_start();
        }
    }
    let mut chars = rest.chars();
    let letter = chars.next().filter(|one| ('A'..='F').contains(one))?;
    chars.next().is_some_and(|after| matches!(after, ' ' | ':' | '·' | '.' | ')')).then_some(letter)
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
            return Err(not_open(format!("the sketch `{title}` asks nothing: a plan's is approved with the plan, and a note on one that only explains reaches the job as a message")));
        }
        if sketch.detail.get(SKETCH_DRAWN) != Some(&said.drawn) {
            return Err(self.redrawn(&title));
        }
        if let Some(how) = sketch.detail.get(SKETCH_SETTLED) {
            return Err(not_open(format!("the sketch `{title}` was {} already", how.replace('_', " "))));
        }
        // A note that only names one take is a pick, so the job goes on from that take, not a redraw.
        let take = (said.how == SketchHow::SentBack)
            .then(|| sketch.detail.get("scene").zip(words).and_then(|(scene, note)| take_picked(scene, note)))
            .flatten();
        let how = if take.is_some() { SketchHow::Approved } else { said.how };
        let body = match (said.how, &take) {
            (_, Some(take)) => picked(&title, take),
            (SketchHow::Approved, None) => approved(&title, words),
            (SketchHow::SentBack, None) => sent_back(&title, words.unwrap_or_default()),
        };
        let message = SendSessionMessage {
            session_id: said.session_id.clone(),
            text: body,
            attachments: said.extras.attachments.clone(),
            mentions: said.extras.mentions.clone(),
            sketches: Vec::new(),
        };
        Arc::clone(&self).send_known(terminal, message).await?;
        let question = sketch.detail.get(SKETCH_QUESTION).cloned().unwrap_or_else(|| title.clone());
        let answered = match (&take, said.how) {
            (Some(_), _) => ipc::AnswerHow::Picked,
            (None, SketchHow::Approved) => ipc::AnswerHow::Approved,
            (None, SketchHow::SentBack) => ipc::AnswerHow::AskedForChanges,
        };
        let kept = super::answers_kept::Answered {
            words,
            pick: take.as_deref(),
            artifact: Some(&target),
            extras: &said.extras,
            ..super::answers_kept::Answered::to(&target, &question, answered)
        };
        self.answer_kept(&id, kept).await;
        // Read again after the send: a drawing the job made meanwhile is not the one he answered.
        let (_, now) = self.sketch_held(&id, &target).await?;
        let Some(mut sketch) = now.filter(|one| one.detail.get(SKETCH_DRAWN) == Some(&said.drawn)) else {
            return self.published_hosted(&id).await;
        };
        sketch.detail.insert(SKETCH_SETTLED.into(), how.word().into());
        sketch.detail.insert(SKETCH_SETTLED_AT.into(), self.instant().as_str().to_string());
        if let Some(words) = words {
            sketch.detail.insert(SKETCH_WORDS.into(), words.to_string());
        }
        let record = self
            .report_session(SessionReport {
                harness,
                session_id: SessionId::carried(&id),
                fact: SessionFact::Attached {
                    attachment: AttachmentReport { kind: sketch.kind, target: sketch.target, detail: sketch.detail },
                },
            })
            .await?;
        Ok(self.approved_items_settled(&id, record).await)
    }

    /// The job's harness, and the sketch it holds as `target`.
    pub(crate) async fn sketch_held(&self, id: &str, target: &str) -> Result<(String, Option<ipc::Attachment>), Refusal> {
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

/// Whether the sketch `target` was drawn at or after `began`, when the turn still running began. **Such a
/// sketch waits only once the turn ends**, so the one `all_done` settles in the same turn never shows and goes.
pub(crate) fn drawn_in_turn(attachments: &[ipc::Attachment], target: &str, began: Option<&str>) -> bool {
    let Some(began) = began else { return false };
    let digits = |instant: &str| instant.chars().filter(char::is_ascii_digit).collect::<String>();
    let began = digits(began);
    attachments.iter().filter(|one| one.target == target).any(|one| digits(&drawn_at(one).0) >= began)
}

/// The titles of the sketches the record's pending plan names, which wait with the plan and not on their own.
pub(crate) fn in_pending_plan(record: &SessionRecord) -> Vec<String> {
    record
        .plan
        .as_ref()
        .filter(|plan| !plan.approved && plan.set_aside_at.is_none())
        .map(|plan| plan.sketches.iter().chain(plan.groups.iter().flat_map(|group| group.sketches.iter())).map(|one| one.title.clone()).collect())
        .unwrap_or_default()
}

/// The items an answer or a dismissal may name: what waits on him, and each sketch drawn in the turn still
/// running, `held`. **Such a sketch is not raised until the turn ends, and still takes his answer on its card.**
pub(crate) fn answerable(mut items: Vec<WaitingItem>, held: Vec<WaitingItem>) -> Vec<WaitingItem> {
    let fresh: Vec<WaitingItem> = held.into_iter().filter(|one| !items.iter().any(|waits| waits.id == one.id)).collect();
    items.extend(fresh);
    items
}

/// Whether a sketch row has a part put to him that is neither answered nor taken back: its drawing, or one
/// added to it. A row that joined an artifact is asked for by the artifact.
fn is_open(sketch: &ipc::Attachment) -> bool {
    super::artifact_parts::row_asks(&sketch.detail)
}

/// Whether a sketch was put to him and is neither answered nor taken back.
pub(crate) fn still_asks(sketch: &ipc::Attachment) -> bool {
    is_open(sketch)
}

/// Every artifact with a part still asking him, newest drawn first, as the one item that waits on him: id the
/// artifact's and naming it, text the question its one ask puts to him, opening the Session. **A sketch, draft or page that joined an artifact is a part of it and raises no item of its own.** An
/// artifact the pending plan names, by `in_plan` titles, is approved with the plan, and an answered one never stays
/// the thread head's pin. `open_drafts` is the ledger id of each draft whose row is still open.
pub(crate) fn sketch_items<'a>(
    attachments: &'a [ipc::Attachment],
    session: &'a str,
    in_plan: &'a [String],
    open_drafts: &BTreeSet<String>,
    dismissed: &[String],
) -> impl Iterator<Item = WaitingItem> + 'a {
    let titled = |one: &ipc::Attachment| one.detail.get("title").cloned().unwrap_or_else(|| one.target.trim_start_matches(TARGET_PREFIX).to_string());
    let mut open: Vec<(&ipc::Attachment, String)> = super::artifact_parts::asking_groups(attachments, in_plan, open_drafts, dismissed)
        .into_iter()
        .filter_map(|asking| {
            let root = attachments.iter().find(|one| is_artifact_sketch(one) && one.target == asking.root)?;
            Some((root, super::artifact_parts::asking_text(&titled(root), &asking)))
        })
        .collect();
    open.sort_by_key(|(one, _)| std::cmp::Reverse(drawn_at(one)));
    open.into_iter().map(move |(one, text)| WaitingItem {
        approves: false,
        id: one.target.clone(),
        text,
        since: one.changed_at.clone(),
        source: WaitingSource::Sketch,
        act: Some(WaitingAct { kind: WaitingActKind::Message, target: session.to_string(), on: None }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
        artifact: Some(one.target.clone()),
    })
}
