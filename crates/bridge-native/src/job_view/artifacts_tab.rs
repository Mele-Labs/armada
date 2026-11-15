//! The Artifacts tab, in the Rows look the owner approved on 14 Nov 2026: each of the Job's own sketches, drafts and
//! pages as one row, newest first, with the archived ones folded at the foot. Fleet's `get_job_artifacts` is the
//! list; the cards in the thread read the same list for their revision, who made it and whether it is folded away,
//! and a page's first screen (the card's well) is made here once, from the sealed page Fleet serves.
//!
//! The list is read again when the Job's thread, record or waiting items move, which is when Fleet says the Job
//! changed. With no Fleet, as on mock data, it is built from the thread's own cards.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, RenderImage, Role, SharedString, Window};
use ipc::{ArtifactArchived, ArtifactBy, ArtifactForm, ArtifactRef, ArtifactRevision, ArtifactState, DraftHow, Instant, JobArtifact, SessionId, SessionRow, SketchHow, ThreadItem};

use super::band_card::{kind_look, revision_chip, state_chip, tile, Stand};
use super::rows::tip;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// What a page's first screen is: asked for, drawn at a height in points, or one Fleet or the web view would not give.
pub(super) enum Picture {
    Asked,
    Ready(Arc<RenderImage>, f32),
    Failed,
}

/// One picture of a page: the revision, the width it is drawn at and the theme, so a resize or a theme change draws it again.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct PictureKey {
    pub job: String,
    pub page: String,
    pub revision: u32,
    pub width: u32,
    pub dark: bool,
}

/// The width a page is laid out at for its card, in CSS points: the width a page reads at, whatever the well's own.
const READING_WIDTH: f32 = 960.;

/// How much the page laid out at the reading width is scaled to fill a well `width` points across, so every line shows whole.
fn fit_scale(width: u32) -> f32 {
    width.max(1) as f32 / READING_WIDTH
}

/// How many first screens are made at once: a thread of twelve pages opens two web views, not twelve.
const MAKING: usize = 2;

/// What the Job view holds of its Jobs' artifacts.
#[derive(Default)]
pub(super) struct Artifacts {
    read: HashMap<String, Read>,
    pictures: HashMap<PictureKey, Picture>,
    waiting: VecDeque<PictureKey>,
    making: usize,
    /// The Jobs whose Archived fold is open.
    unfolded: HashSet<String>,
    /// The row J and K are on, by Job.
    cursor: HashMap<String, String>,
    /// What the mock Fleet folded away or restored here, by Job and artifact, since it keeps nothing.
    folded_here: HashMap<(String, String), bool>,
    /// What the mock Fleet was told he opened and edited here, by Job and artifact, since it keeps nothing: the newest
    /// revision he opened, and how many revisions of his own were kept.
    seen_here: HashMap<(String, String), u32>,
    edits_here: HashMap<(String, String), u32>,
    /// What the mock Fleet keeps of the artifact of parts, since it keeps nothing: the artifact as it stands once he has answered
    /// parts of it, and every `answer_part` it was sent, in order.
    mock_parts: Vec<JobArtifact>,
    mock_posts: Vec<ipc::AnswerPart>,
    /// Every `mark_artifact_seen` the mock Fleet was sent, in order.
    mock_seen: Vec<ipc::MarkArtifactSeen>,
}

struct Read {
    /// What the list was read for: the thread's version and how much the record held, so a change of either reads again.
    stamp: (u64, usize, usize),
    /// None until the first answer, so the tab draws its shapes.
    rows: Option<Vec<JobArtifact>>,
}

/// A Job's artifacts as the mock Fleet would list them: the thread's own sketches, pages and drafts, newest first.
pub(super) fn mock_artifacts(items: &[ThreadItem]) -> Vec<JobArtifact> {
    let revisions = |count: u32, at: &Instant, form: ArtifactForm, by_last: ArtifactBy| -> Vec<ArtifactRevision> {
        (1..=count.max(1)).map(|number| ArtifactRevision { number, at: at.clone(), by: if number == count.max(1) { by_last } else { ArtifactBy::Agent }, form }).collect()
    };
    let state_of = |settled: Option<&ipc::CardSettled>, asked: bool| match settled {
        Some(settled) if settled.how == SketchHow::Approved => ArtifactState::Approved,
        Some(_) => ArtifactState::SentBack,
        None if asked => ArtifactState::Asking,
        None => ArtifactState::Reading,
    };
    let mut out: Vec<JobArtifact> = Vec::new();
    let mut put = |one: JobArtifact| {
        out.retain(|held| held.id != one.id);
        out.push(one);
    };
    for item in items {
        match item {
            ThreadItem::Sketch(card) if !card.replaced => {
                let state = if card.withdrawn { ArtifactState::Withdrawn } else { state_of(card.settled.as_ref(), card.asked) };
                put(JobArtifact {
                    id: card.sketch.clone(),
                    title: card.title.clone(),
                    form: ArtifactForm::Sketch,
                    revisions: revisions(card.revisions.len() as u32, &card.at, ArtifactForm::Sketch, ArtifactBy::Agent),
                    state,
                    words: card.settled.as_ref().and_then(|settled| settled.words.clone()),
                    item: (state == ArtifactState::Asking).then(|| card.sketch.clone()),
                    in_plan: None,
                    archived: None,
                    for_line: card.for_line.clone(),
                    presses: Vec::new(),
                    votes: Vec::new(),
                    seen: None,
                    parts: Vec::new(),
                });
            }
            ThreadItem::Document(card) => {
                let state = state_of(card.settled.as_ref(), card.asked);
                put(JobArtifact {
                    id: card.page.clone(),
                    title: card.title.clone(),
                    form: ArtifactForm::Page,
                    revisions: revisions(card.revision, &card.at, ArtifactForm::Page, ArtifactBy::Agent),
                    state,
                    words: card.settled.as_ref().and_then(|settled| settled.words.clone()),
                    item: (state == ArtifactState::Asking).then(|| format!("document:{}", card.page)),
                    in_plan: None,
                    archived: None,
                    for_line: card.for_line.clone(),
                    presses: Vec::new(),
                    votes: Vec::new(),
                    seen: None,
                    parts: Vec::new(),
                });
            }
            ThreadItem::Row { row: SessionRow::Draft { id, at, title, settled, change: None, for_line, before, .. } } => {
                let (state, words, mine) = match settled.as_ref().map(|settled| (settled.how, settled.words.clone())) {
                    Some((DraftHow::Approved, _)) => (ArtifactState::Approved, None, false),
                    Some((DraftHow::Fixed, words)) => (ArtifactState::Approved, words, true),
                    Some((DraftHow::SentBack, words)) => (ArtifactState::SentBack, words, false),
                    Some((DraftHow::Withdrawn, words)) => (ArtifactState::Dismissed, words, false),
                    None => (ArtifactState::Asking, None, false),
                };
                put(JobArtifact {
                    id: format!("draft:{title}"),
                    title: title.clone(),
                    form: ArtifactForm::Draft,
                    revisions: revisions(if mine || before.is_some() { 2 } else { 1 }, at, ArtifactForm::Draft, if mine { ArtifactBy::Owner } else { ArtifactBy::Agent }),
                    state,
                    words,
                    item: (state == ArtifactState::Asking).then(|| format!("draft:{id}")),
                    in_plan: None,
                    archived: None,
                    for_line: for_line.clone(),
                    presses: Vec::new(),
                    votes: Vec::new(),
                    seen: before.is_some().then_some(1),
                    parts: Vec::new(),
                });
            }
            _ => {}
        }
    }
    out.sort_by(|a, b| b.revisions.last().map(|r| &r.at).cmp(&a.revisions.last().map(|r| &r.at)));
    out
}

/// The list as the tab draws it: the ones not folded away, newest first, then the folded.
pub(super) fn split(rows: &[JobArtifact]) -> (Vec<&JobArtifact>, Vec<&JobArtifact>) {
    rows.iter().partition(|one| one.archived.is_none())
}

/// The row after (`1`) or before (`-1`) the one the cursor is on, among `ids`; the first where it is on none.
pub(super) fn step(ids: &[String], on: Option<&str>, by: i32) -> Option<String> {
    let at = on.and_then(|on| ids.iter().position(|id| id == on));
    let next = match (at, by) {
        (None, _) => 0,
        (Some(at), by) if by > 0 => (at + 1).min(ids.len().saturating_sub(1)),
        (Some(at), _) => at.saturating_sub(1),
    };
    ids.get(next).cloned()
}

impl JobView {
    /// The artifact of Job `job` by its id, as Fleet last listed it.
    pub(super) fn artifact_of(&self, job: &str, id: &str) -> Option<&JobArtifact> {
        self.artifacts.read.get(job)?.rows.as_ref()?.iter().find(|one| one.id == id)
    }

    /// The artifact a waiting item of Job `job` names, from the item as Fleet kept it: the id of one of the Job's artifacts.
    pub(super) fn row_artifact(&self, job: &str, item: &str, cx: &gpui::App) -> Option<String> {
        self.record(job, cx)?.waiting_for.iter().find(|one| one.id == item)?.artifact.clone()
    }

    /// Whether Fleet has listed Job `job`'s artifacts and `artifact` is not among them: it no longer holds it. Until the list
    /// is read nothing is said to be gone.
    pub(super) fn artifact_gone(&self, job: &str, artifact: &str) -> bool {
        self.artifacts.read.get(job).is_some_and(|read| read.rows.as_ref().is_some_and(|rows| rows.iter().all(|one| one.id != artifact)))
    }

    /// The words of a Needs you row whose press keeps its own act: a press on them opens the artifact the item names. A tab stop
    /// that Enter presses, named for the artifact.
    pub(super) fn artifact_words(&self, job: &str, artifact: &str, text: String, truncate: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let title = self.artifact_of(job, artifact).map_or_else(|| artifact.to_string(), |one| one.title.clone());
        let named = artifact.to_string();
        labelled(
            div()
                .id(SharedString::from(format!("needs-artifact:{artifact}")))
                .flex_1()
                .min_w_0()
                .rounded(look.length(token::RADIUS_SM))
                .cursor_pointer()
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .hover(|style| style.text_color(look.color(token::ACCENT)))
                .tooltip(tip(format!("Open {title}")))
                .when(truncate, |words| words.truncate())
                .child(SharedString::from(text))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.open_named(&named, window, cx);
                })),
            Role::Button,
            format!("Open {title}"),
        )
        .into_any_element()
    }

    /// Open the artifact of the shown Job where it is read, as every press that names one does: the Artifacts tab's row, a card's
    /// Open, a Needs you item and a folded line. Returns whether the Job holds it: one it no longer holds opens nothing.
    pub(super) fn open_named(&mut self, artifact: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(job) = self.shown.clone() else { return false };
        if self.artifact_of(&job, artifact).is_none() {
            return false;
        }
        self.set_needs(false, None, window, cx);
        self.open_artifact(&job, artifact, window, cx);
        cx.notify();
        true
    }

    /// Every artifact of Job `job`, as Fleet last listed them.
    pub(super) fn artifacts_of(&self, job: &str) -> Vec<&JobArtifact> {
        self.artifacts.read.get(job).and_then(|read| read.rows.as_ref()).map(|rows| rows.iter().collect()).unwrap_or_default()
    }

    /// Every `answer_part` the mock Fleet was sent, oldest first.
    #[cfg(test)]
    pub(super) fn mock_part_posts(&self) -> &[ipc::AnswerPart] {
        &self.artifacts.mock_posts
    }

    /// Mock data: Fleet takes the answer to a part as it does, in its own words where it refuses, and lists the artifact again.
    pub(super) fn mock_part_answered(&mut self, job: &SharedString, body: Result<String, String>, cx: &mut Context<Self>) -> Result<(), String> {
        let said = body.and_then(|body| ipc::decode::<ipc::AnswerPart>("AnswerPart", body.as_bytes()).map_err(|why| why.to_string()))?;
        self.artifacts.mock_posts.push(said.clone());
        let mut kept = self.artifacts.mock_parts.iter().find(|one| one.id == said.artifact).cloned().ok_or_else(|| "the mock Fleet holds no such artifact of parts".to_string())?;
        crate::mock_parts::apply(&mut kept, &said)?;
        // A Job that is sent a part back draws it again, as the next revision, which asks.
        if crate::mock_parts::folds() && kept.state == ArtifactState::SentBack {
            crate::mock_parts::redraw(&mut kept, Instant::carried(Local::now().to_rfc3339()));
        }
        self.mock_parts_kept(job, kept, cx);
        Ok(())
    }

    /// Mock data: Fleet keeps a press on a buttons part and lists the artifact again.
    pub(super) fn mock_part_pressed(&mut self, job: &SharedString, body: Result<String, String>, cx: &mut Context<Self>) {
        let Ok(pressed) = body.map_err(|_| ()).and_then(|body| ipc::decode::<ipc::PressPageAction>("PressPageAction", body.as_bytes()).map_err(|_| ())) else { return };
        let Some(mut kept) = self.artifacts.mock_parts.iter().find(|one| one.id == pressed.artifact).cloned() else { return };
        let Some(action) = kept.parts.iter().find_map(|part| match &part.content {
            ipc::PartContent::Buttons { actions } => actions.iter().find(|action| action.id == pressed.action).cloned(),
            _ => None,
        }) else {
            return;
        };
        kept.presses.push(ipc::PagePress { revision: pressed.revision, action: action.id, item: action.item, act: action.act, at: Instant::carried(Local::now().to_rfc3339()), did: ipc::PressDid::Sent });
        self.mock_parts_kept(job, kept, cx);
    }

    fn mock_parts_kept(&mut self, job: &SharedString, kept: JobArtifact, cx: &mut Context<Self>) {
        if let Some(rows) = self.artifacts.read.get_mut(job.as_ref()).and_then(|read| read.rows.as_mut()) {
            match rows.iter_mut().find(|one| one.id == kept.id) {
                Some(one) => *one = kept.clone(),
                None => rows.insert(0, kept.clone()),
            }
        }
        match self.artifacts.mock_parts.iter_mut().find(|one| one.id == kept.id) {
            Some(one) => *one = kept,
            None => self.artifacts.mock_parts.push(kept),
        }
        // As Fleet's list arriving: the cards are placed again, since a new revision is a card of its own.
        self.parts_arrived(job, cx);
        self.parts_redrawn(job, cx);
    }

    /// A page's first screen, as it stands.
    pub(super) fn picture_of(&self, key: &PictureKey) -> Option<&Picture> {
        self.artifacts.pictures.get(key)
    }

    /// Read the shown Job's list where its thread or record moved. Called as the view draws, never from a row.
    pub(super) fn sync_artifacts(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let Some(version) = self.thread(id, cx).map(|held| held.version) else { return };
        let record = self.record(id, cx).map(|record| (record.attachments.len(), record.waiting_for.len())).unwrap_or_default();
        let stamp = (version, record.0, record.1);
        if self.artifacts.read.get(id.as_ref()).is_some_and(|read| read.stamp == stamp) {
            return;
        }
        let rows = self.artifacts.read.get(id.as_ref()).and_then(|read| read.rows.clone());
        self.artifacts.read.insert(id.to_string(), Read { stamp, rows });
        self.read_artifacts(id, window, cx);
    }

    fn read_artifacts(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            let mut rows = self.thread(id, cx).map(|held| mock_artifacts(&held.items)).unwrap_or_default();
            if id.as_ref() == crate::mock_parts::JOB {
                if let Some(initial) = crate::mock_parts::initial() {
                    if self.artifacts.mock_parts.is_empty() {
                        self.artifacts.mock_parts = initial;
                        // What he said of an older revision before this run, which the mock Fleet cannot say.
                        for (artifact, revision, note) in crate::mock_parts::earlier_verdicts() {
                            let verdict = super::parts_fold::Verdict { stand: Stand::SentBack, word: Stand::SentBack.word().into(), note: Some(note.into()) };
                            self.keep_verdict(id, artifact, revision, &verdict, cx);
                        }
                    }
                    for (at, kept) in self.artifacts.mock_parts.iter().enumerate() {
                        rows.retain(|one| one.id != kept.id);
                        rows.insert(at.min(rows.len()), kept.clone());
                    }
                }
            }
            for one in &mut rows {
                match self.artifacts.folded_here.get(&(id.to_string(), one.id.clone())) {
                    Some(true) => one.archived = Some(ArtifactArchived { by: ArtifactBy::Owner, at: Instant::carried(Local::now().to_rfc3339()) }),
                    Some(false) => one.archived = None,
                    None => {}
                }
                let key = (id.to_string(), one.id.clone());
                for _ in 0..self.artifacts.edits_here.get(&key).copied().unwrap_or(0) {
                    let number = one.revisions.last().map_or(1, |last| last.number) + 1;
                    let at = Instant::carried(Local::now().to_rfc3339());
                    one.revisions.push(ipc::ArtifactRevision { number, at, by: ArtifactBy::Owner, form: one.form });
                }
                one.seen = self.artifacts.seen_here.get(&key).copied().or(one.seen);
            }
            if let Some(read) = self.artifacts.read.get_mut(id.as_ref()) {
                read.rows = Some(rows);
            }
            self.parts_arrived(id, cx);
            self.offer_artifacts(id, cx);
            return;
        }
        let (job, path) = (id.to_string(), format!("/sessions/artifacts?session_id={}", super::wire::encoded(id)));
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                // A refusal leaves what the tab held; the next change reads again.
                if let Some(rows) = answer.ok().and_then(|bytes| ipc::decode::<Vec<JobArtifact>>("Job artifacts", &bytes).ok()) {
                    if let Some(read) = this.artifacts.read.get_mut(&job) {
                        read.rows = Some(rows);
                    }
                    let job = SharedString::from(job.clone());
                    this.parts_arrived(&job, cx);
                    this.offer_artifacts(&job, cx);
                    cx.notify();
                }
            }),
        );
    }

    /// The Job's first artifact puts its Artifacts tab beside Now, behind, once (his ask of 15 Nov 2026). It waits for the
    /// store's answer, so a layout kept from an earlier run is not lost to it.
    pub(super) fn offer_artifacts(&mut self, job: &SharedString, cx: &mut Context<Self>) {
        let any = self.artifacts.read.get(job.as_ref()).and_then(|read| read.rows.as_ref()).is_some_and(|rows| !rows.is_empty());
        if any && self.panes_answered(job) {
            self.look_in(job, |was| super::panes_place::offer_beside_now(was, "artifacts"), cx);
        }
    }

    /// An artifact he folded away or restored here is drawn so at once, before Fleet's list says it.
    pub(super) fn archived_here(&mut self, job: &str, artifact: &str, on: bool) {
        self.artifacts.folded_here.insert((job.to_string(), artifact.to_string()), on);
        if let Some(one) = self.artifacts.read.get_mut(job).and_then(|read| read.rows.as_mut()).and_then(|rows| rows.iter_mut().find(|one| one.id == artifact)) {
            one.archived = on.then(|| ArtifactArchived { by: ArtifactBy::Owner, at: Instant::carried(Local::now().to_rfc3339()) });
        }
    }

    /// He opened this revision, on mock data: it is the newest he has opened from here on, an older one moving nothing.
    pub(super) fn seen_here(&mut self, job: &str, artifact: &str, revision: u32) {
        let kept = self.artifacts.seen_here.entry((job.to_string(), artifact.to_string())).or_default();
        *kept = (*kept).max(revision);
        let kept = *kept;
        if let Some(one) = self.artifacts.read.get_mut(job).and_then(|read| read.rows.as_mut()).and_then(|rows| rows.iter_mut().find(|one| one.id == artifact)) {
            one.seen = Some(kept);
        }
    }

    /// The mock Fleet takes that he opened a revision: it keeps the post, and the revision is seen as Fleet would keep it.
    pub(super) fn artifacts_seen_post(&mut self, job: &str, artifact: &str, revision: u32) {
        self.artifacts.mock_seen.push(ipc::MarkArtifactSeen { session_id: SessionId::carried(job.to_string()), artifact: artifact.to_string(), revision });
        self.seen_here(job, artifact, revision);
    }

    /// Every `mark_artifact_seen` the mock Fleet was sent, oldest first.
    #[cfg(test)]
    pub(super) fn mock_seen_posts(&self) -> &[ipc::MarkArtifactSeen] {
        &self.artifacts.mock_seen
    }

    /// He saved an edit, on mock data: one more revision, his.
    pub(super) fn edited_here(&mut self, job: &str, artifact: &str) {
        *self.artifacts.edits_here.entry((job.to_string(), artifact.to_string())).or_default() += 1;
    }

    // ---- a page's first screen ------------------------------------------------------------

    /// The first screen of a card the thread drew: asked for once, off the draw, by a canvas that notices it was not there.
    pub(super) fn asking_picture(&self, key: &PictureKey, shape: AnyElement, cx: &Context<Self>) -> AnyElement {
        let (me, want) = (cx.entity().downgrade(), key.clone());
        div()
            .relative()
            .child(shape)
            .child(
                gpui::canvas(
                    move |_, window, cx| {
                        let (me, want) = (me.clone(), want.clone());
                        window.defer(cx, move |window, cx| {
                            let _ = me.update(cx, |this, cx| this.want_picture(want, window, cx));
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .into_any_element()
    }

    fn want_picture(&mut self, key: PictureKey, window: &mut Window, cx: &mut Context<Self>) {
        if self.artifacts.pictures.contains_key(&key) {
            return;
        }
        self.artifacts.pictures.insert(key.clone(), Picture::Asked);
        self.artifacts.waiting.push_back(key);
        self.make_pictures(window, cx);
    }

    /// Start the pictures that wait, while fewer than two are being made.
    fn make_pictures(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        while self.artifacts.making < MAKING {
            let Some(key) = self.artifacts.waiting.pop_front() else { return };
            self.artifacts.making += 1;
            self.make_picture(key, window, cx);
        }
    }

    fn make_picture(&mut self, key: PictureKey, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            let html = crate::mock::sealed_page(&key.page, key.revision, &[], &[]);
            self.paint_picture(key, html, window, cx);
            return;
        }
        let path = format!("/sessions/artifacts/page?session_id={}&artifact={}&revision={}", super::wire::encoded(&key.job), super::wire::encoded(&key.page), key.revision);
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                let html = answer.and_then(|bytes| String::from_utf8(bytes).map_err(|_| "Fleet's page is not text".to_string()));
                this.paint_picture(key, html, window, cx);
            }),
        );
    }

    /// The sealed page drawn once by the web view the app lends, which runs none of its script. It is laid out at the reading
    /// width and the whole picture is scaled to the card's width, so the well shows the first screen small and whole.
    fn paint_picture(&mut self, key: PictureKey, html: Result<String, String>, window: &mut Window, cx: &mut Context<Self>) {
        use crate::scene::mock_view::Appearance;
        let painter = crate::scene::mock_view::painter(cx);
        let (Ok(html), Some(painter)) = (html, painter) else {
            self.picture_made(key, Err(String::new()), window, cx);
            return;
        };
        let appearance = if key.dark { Appearance::Dark } else { Appearance::Light };
        let fit = fit_scale(key.width);
        let task = painter(html, READING_WIDTH, window.scale_factor() * fit, appearance, cx);
        cx.spawn_in(window, async move |this, cx| {
            // The picture's height comes in points at the reading width; it is drawn at the well's.
            let made = task.await.map(|(image, height, parts)| (image, height * fit, parts));
            let _ = this.update_in(cx, |this, window, cx| this.picture_made(key, made, window, cx));
        })
        .detach();
    }

    fn picture_made(&mut self, key: PictureKey, made: Result<crate::scene::mock_view::Picture, String>, window: &mut Window, cx: &mut Context<Self>) {
        self.artifacts.making = self.artifacts.making.saturating_sub(1);
        // An older width or theme of the same revision is let go.
        self.artifacts.pictures.retain(|held, _| held == &key || !(held.job == key.job && held.page == key.page && held.revision == key.revision));
        self.artifacts.pictures.insert(key, made.map_or(Picture::Failed, |(image, height, _)| Picture::Ready(image, height)));
        self.pages.values().for_each(|page| page.list.remeasure());
        cx.notify();
        self.make_pictures(window, cx);
    }

    // ---- the tab -------------------------------------------------------------------------

    /// The Artifacts tab: one row each, newest first, with the archived folded at the foot. None while the Job has none.
    pub(super) fn pane_artifacts(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let page = self.pages.get(id)?;
        let Some(rows) = self.artifacts.read.get(id.as_ref()) else { return None };
        let Some(rows) = rows.rows.as_ref() else { return Some(self.artifact_shapes(look)) };
        let (live, folded) = split(rows);
        if live.is_empty() && folded.is_empty() {
            return None;
        }
        let now = Local::now();
        let cursor = self.artifacts.cursor.get(id.as_ref()).cloned();
        let open = self.artifacts.unfolded.contains(id.as_ref());
        let spot = page.keys.spot("artifacts-tab", cx);
        let ids: Vec<String> = live.iter().map(|one| one.id.clone()).chain(open.then(|| folded.iter().map(|one| one.id.clone())).into_iter().flatten()).collect();
        let (job, keyed) = (id.to_string(), ids.clone());
        let list = div()
            .id("pane-artifacts")
            .track_focus(&spot.focus)
            .flex()
            .flex_col()
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                let modifiers = event.keystroke.modifiers;
                let on = this.artifacts.cursor.get(&job).cloned();
                match (event.keystroke.key.as_str(), modifiers.shift, modifiers.modified()) {
                    ("j", false, false) | ("down", false, false) => this.artifacts_cursor(&job, step(&keyed, on.as_deref(), 1), cx),
                    ("k", false, false) | ("up", false, false) => this.artifacts_cursor(&job, step(&keyed, on.as_deref(), -1), cx),
                    ("enter", false, false) => {
                        let Some(on) = on else { return };
                        this.open_artifact(&job, &on, window, cx);
                    }
                    ("r", true, true) => {
                        let Some(on) = on else { return };
                        this.restore_artifact_row(&job, &on, window, cx);
                    }
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .children(live.iter().map(|one| self.artifact_row(id, one, cursor.as_deref() == Some(one.id.as_str()), false, now, look, cx)))
            .children((!folded.is_empty()).then(|| self.archived_fold(id, open, look, cx)))
            .children(open.then(|| folded.iter().map(|one| self.artifact_row(id, one, cursor.as_deref() == Some(one.id.as_str()), true, now, look, cx)).collect::<Vec<_>>()).into_iter().flatten());
        Some(labelled(list, Role::List, "Artifacts").into_any_element())
    }

    fn artifacts_cursor(&mut self, job: &str, to: Option<String>, cx: &mut Context<Self>) {
        if let Some(to) = to {
            self.artifacts.cursor.insert(job.to_string(), to);
            cx.notify();
        }
    }

    /// One artifact: its kind's tile beside its title, what it is for, and a line of where it stands, its revision and
    /// when it was kept. That line starts at the title's edge on every row, so the rows read down the list at any width.
    #[allow(clippy::too_many_arguments)]
    fn artifact_row(&self, id: &SharedString, one: &JobArtifact, on: bool, folded: bool, now: DateTime<Local>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (key, job) = (one.id.clone(), id.to_string());
        let (_, _, word) = kind_look(one.form);
        let last = one.revisions.last();
        let number = last.map_or(1, |last| last.number);
        let said = last.map_or_else(String::new, |last| super::band_card::revision_said(last.number, last.by, last.at.as_str()));
        let line = one.for_line.as_ref().and_then(|line| line.text.clone()).filter(|text| !text.trim().is_empty());
        let meta = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(state_chip(look, format!("artifact-state:{key}"), Stand::of(one.state), one.words.as_deref(), None))
            .child(revision_chip(look, format!("artifact-revision:{key}"), number, said))
            .children(last.map(|last| self.stamp(&last.at, look, now, true, cx)));
        let row = div()
            .id(SharedString::from(format!("artifact-row:{key}")))
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .cursor_pointer()
            .when(on, |row| row.bg(look.color(token::BG_HOVER)))
            .when(folded, |row| row.opacity(0.5))
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .child(tile(look, one.form))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(
                        div()
                            .truncate()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(one.title.clone())),
                    )
                    .children(line.map(|line| {
                        div().truncate().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(line))
                    }))
                    .child(meta),
            );
        let pressed = (job, key.clone());
        let row = row.on_click(cx.listener(move |this, _, window, cx| {
            this.artifacts_cursor(&pressed.0, Some(pressed.1.clone()), cx);
            this.open_artifact(&pressed.0, &pressed.1, window, cx);
        }));
        labelled(row, Role::Button, format!("Open {} {}", word.to_lowercase(), one.title)).into_any_element()
    }

    /// The fold over the archived rows: its word, with no count, and the key that restores the row the cursor is on.
    fn archived_fold(&self, id: &SharedString, open: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let job = id.to_string();
        let row = div()
            .id("artifacts-archived")
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .bg(look.color(token::BG_SUNKEN))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .cursor_pointer()
            .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
            .child(gpui::svg().path(crate::assets::icon_path(if open { "chevron-down" } else { "chevron-right" })).flex_none().size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)))
            .child("Archived")
            .child(div().flex_1())
            .children(open.then(|| div().flex().items_center().gap(look.length(token::SPACE_1)).child(super::acts::kbd(look, "\u{21e7}R")).child("restore")))
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.artifacts.unfolded.remove(&job) {
                    this.artifacts.unfolded.insert(job.clone());
                }
                cx.notify();
            }));
        labelled(row.tooltip(tip(if open { "Fold the archived away" } else { "Show the archived" })), Role::Button, "Archived").into_any_element()
    }

    /// The tab's shapes while Fleet has not answered.
    fn artifact_shapes(&self, look: &Look) -> AnyElement {
        use gpui::AnimationExt as _;
        let bar = |width: f32| div().h(look.length(token::SPACE_3)).w(gpui::relative(width)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::FG_DEFAULT).opacity(0.07));
        labelled(
            div().id("artifacts-loading").w_full().p(look.length(token::SPACE_4)).child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .child(bar(0.5))
                    .child(bar(0.9))
                    .child(bar(0.7))
                    .with_animation(
                        "artifacts-loading-pulse",
                        gpui::Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                        |shapes, at| shapes.opacity(at),
                    ),
            ),
            Role::Status,
            "Loading",
        )
        .into_any_element()
    }

    /// Every artifact is read whole in its own tab. One that holds parts opens in the artifact's tab, with every part answerable
    /// and its drawing large; a page opens in its tab, a draft in its tab, and a sketch large over the window, where it zooms.
    fn open_artifact(&mut self, job: &str, artifact: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = self.artifact_of(job, artifact).cloned() else { return };
        if !one.parts.is_empty() {
            self.open_artifact_tab(&one.id, window, cx);
            return;
        }
        match one.form {
            ArtifactForm::Page => self.open_document_tab(&one.id, window, cx),
            ArtifactForm::Draft => {
                let row = self.thread(job, cx).and_then(|held| {
                    held.items.iter().rev().find_map(|item| match item {
                        ThreadItem::Row { row: SessionRow::Draft { id, title, .. } } if *title == one.title => Some(id.clone()),
                        _ => None,
                    })
                });
                if let Some(row) = row {
                    self.open_draft_tab(&row, window, cx);
                }
            }
            ArtifactForm::Sketch => {
                // By its ledger id, else by its newest title: a sketch retitled `against` an earlier one keeps the
                // first title's id while its newest card carries the new title's.
                let card = self.thread(job, cx).and_then(|held| {
                    let newest = |hit: &dyn Fn(&ipc::ThreadSketch) -> bool| {
                        held.items.iter().rev().find_map(|item| match item {
                            ThreadItem::Sketch(card) if hit(card) => Some(card.id.clone()),
                            _ => None,
                        })
                    };
                    newest(&|card| card.sketch == one.id).or_else(|| newest(&|card| card.title == one.title))
                });
                if let Some(card) = card {
                    self.read_sketch(&card, &card, window, cx);
                }
            }
        }
    }

    /// ⇧R on an archived row lifts the archive (`restore_artifact`); the row is drawn restored at once.
    fn restore_artifact_row(&mut self, job: &str, artifact: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.artifact_of(job, artifact).is_some_and(|one| one.archived.is_some()) {
            return;
        }
        self.archived_here(job, artifact, false);
        cx.notify();
        let body = ipc::encode(&ArtifactRef { artifact: artifact.to_string(), session_id: Some(SessionId::carried(job.to_string())) });
        let (job, artifact) = (SharedString::from(job.to_string()), artifact.to_string());
        let back = (job.clone(), artifact);
        self.post(
            "/sessions/artifacts/restore".into(),
            body,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                // A refusal puts the row back where it was, and the next read says the rest.
                if answer.is_err() {
                    this.archived_here(&back.0, &back.1, true);
                    cx.notify();
                    return;
                }
                if this.fleet.is_some() {
                    this.read_artifacts(&back.0, window, cx);
                }
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<ThreadItem> {
        let at = |when: &str| Instant::carried(format!("2026-11-14T{when}:00Z"));
        let page = |name: &str, revision: u32, when: &str, asked: bool| ThreadItem::Document(ipc::ThreadDocument { id: format!("document:{name}:r{revision}"), at: at(when), page: name.into(), title: name.into(), revision, asked, settled: None, for_line: None });
        let draft = |row: &str, title: &str, when: &str, how: Option<DraftHow>| ThreadItem::Row {
            row: SessionRow::Draft {
                id: row.into(),
                at: at(when),
                title: title.into(),
                text: "words".into(),
                before: None,
                settled: how.map(|how| ipc::DraftSettled { how, words: None, at: at(when) }),
                change: None,
                for_line: None,
            },
        };
        vec![page("design", 1, "09:00", true), draft("dra-1", "Release note", "10:00", Some(DraftHow::Fixed)), page("design", 2, "11:00", true), draft("dra-2", "Comment", "08:00", None)]
    }

    #[test]
    fn the_list_is_newest_first_with_one_row_for_each_page_at_its_newest_revision() {
        let rows = mock_artifacts(&items());
        assert_eq!(rows.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["design", "draft:Release note", "draft:Comment"]);
        assert_eq!(rows[0].revisions.len(), 2);
        assert_eq!(rows[0].state, ArtifactState::Asking);
    }

    #[test]
    fn a_draft_he_fixed_reads_approved_with_his_revision_last() {
        let rows = mock_artifacts(&items());
        let note = rows.iter().find(|one| one.id == "draft:Release note").unwrap();
        assert_eq!(note.state, ArtifactState::Approved);
        assert_eq!(note.revisions.last().map(|last| last.by), Some(ArtifactBy::Owner));
        assert_eq!(note.item, None, "nothing waits on him");
    }

    #[test]
    fn the_archived_fold_apart_and_keep_their_order() {
        let mut rows = mock_artifacts(&items());
        rows[1].archived = Some(ArtifactArchived { by: ArtifactBy::Owner, at: Instant::carried("2026-11-14T12:00:00Z") });
        let (live, folded) = split(&rows);
        assert_eq!(live.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["design", "draft:Comment"]);
        assert_eq!(folded.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["draft:Release note"]);
    }

    #[test]
    fn j_and_k_move_down_and_up_and_stop_at_the_ends() {
        let ids: Vec<String> = ["a", "b", "c"].iter().map(|id| id.to_string()).collect();
        assert_eq!(step(&ids, None, 1).as_deref(), Some("a"), "the first where none is on");
        assert_eq!(step(&ids, Some("a"), 1).as_deref(), Some("b"));
        assert_eq!(step(&ids, Some("c"), 1).as_deref(), Some("c"));
        assert_eq!(step(&ids, Some("b"), -1).as_deref(), Some("a"));
        assert_eq!(step(&ids, Some("a"), -1).as_deref(), Some("a"));
        assert_eq!(step(&[], None, 1), None);
    }

    #[test]
    fn artifacts_is_a_tab_a_pane_can_hold_and_a_zone_that_draws_it() {
        assert!(super::super::pane_tree::is_tab_kind("artifacts"));
        assert_eq!(super::super::panes_place::zone_of("artifacts").map(super::super::side::Zone::id), Some("artifacts"));
    }

    #[test]
    fn a_page_is_scaled_from_the_reading_width_to_the_wells() {
        assert_eq!(fit_scale(480), 0.5, "half as wide: every line at half size");
        assert_eq!(fit_scale(960), 1.);
        assert!(fit_scale(1100) > 1., "a wide well magnifies the whole page, not a corner of it");
        assert!(fit_scale(0) > 0., "a zero width never collapses the picture to nothing");
        assert_eq!(READING_WIDTH * fit_scale(600), 600., "the scaled page is exactly the well's width");
    }

    /// The artifact-parts Job on screen with its thread read, drawn once, and Needs you open.
    fn parts_job<'a>(cx: &'a mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("artifact-parts").expect("the fixture decodes");
        let threads = crate::mock::threads("artifact-parts").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        list.update(cx, |list, cx| list.open(crate::mock_parts::JOB.into(), cx));
        cx.run_until_parked();
        view.update_in(cx, |view, window, cx| view.set_needs(true, None, window, cx));
        cx.run_until_parked();
        (view, cx)
    }

    #[gpui::test]
    fn an_item_that_names_an_artifact_opens_it_and_an_unknown_one_opens_nothing(cx: &mut gpui::TestAppContext) {
        let (view, cx) = parts_job(cx);
        let job = crate::mock_parts::JOB;
        let (held, retired) = (crate::mock_parts::ID, "sketch:Retired step bar");
        view.read_with(cx, |view, cx| {
            assert_eq!(view.row_artifact(job, "agent-parts", cx).as_deref(), Some(held), "the item carries the id Fleet kept");
            assert!(!view.artifact_gone(job, held));
            assert!(view.artifact_gone(job, retired), "an id the Job does not hold reads as no longer held");
        });
        let open = |view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext| view.read_with(cx, |view, _| view.dock_of(job).and_then(|dock| dock.open));
        assert_eq!(open(&view, cx), Some(true), "Needs you is open");
        let opened = view.update_in(cx, |view, window, cx| view.open_named(retired, window, cx));
        assert!(!opened, "the Job holds no such artifact");
        assert_eq!(open(&view, cx), Some(true), "nothing moved");
        let opened = view.update_in(cx, |view, window, cx| view.open_named(held, window, cx));
        assert!(opened);
        assert_eq!(open(&view, cx), Some(false), "Needs you gives way to the artifact");
    }

    #[gpui::test]
    fn showing_a_revision_tells_fleet_once_however_often_it_draws(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("reader").expect("the fixture decodes");
        let threads = crate::mock::threads("reader").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        list.update(cx, |list, cx| list.open("draft-job".into(), cx));
        cx.run_until_parked();
        view.update_in(cx, |view, window, cx| view.open_document_tab("design", window, cx));
        cx.run_until_parked();
        let posts = |view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext| {
            view.read_with(cx, |view, _| view.mock_seen_posts().iter().map(|one| (one.artifact.clone(), one.revision)).collect::<Vec<_>>())
        };
        assert_eq!(posts(&view, cx), [("design".to_string(), 2)], "the newest revision shown is told at once");
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
        assert_eq!(posts(&view, cx), [("design".to_string(), 2)], "a redraw tells nothing again");
        let name = super::super::reader::reader_id(super::super::row_tabs::RowKind::Document, "design");
        view.update_in(cx, |view, window, cx| view.reader_view(&"draft-job".into(), &name, 1, window, cx));
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
        assert_eq!(posts(&view, cx), [("design".to_string(), 2), ("design".to_string(), 1)], "an older revision posts its own number, once");
    }
}
