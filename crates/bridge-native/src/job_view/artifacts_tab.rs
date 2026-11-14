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
                });
            }
            ThreadItem::Row { row: SessionRow::Draft { id, at, title, settled, change: None, for_line, .. } } => {
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
                    revisions: revisions(if mine { 2 } else { 1 }, at, ArtifactForm::Draft, if mine { ArtifactBy::Owner } else { ArtifactBy::Agent }),
                    state,
                    words,
                    item: (state == ArtifactState::Asking).then(|| format!("draft:{id}")),
                    in_plan: None,
                    archived: None,
                    for_line: for_line.clone(),
                    presses: Vec::new(),
                    votes: Vec::new(),
                    seen: None,
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
            for one in &mut rows {
                match self.artifacts.folded_here.get(&(id.to_string(), one.id.clone())) {
                    Some(true) => one.archived = Some(ArtifactArchived { by: ArtifactBy::Owner, at: Instant::carried(Local::now().to_rfc3339()) }),
                    Some(false) => one.archived = None,
                    None => {}
                }
            }
            if let Some(read) = self.artifacts.read.get_mut(id.as_ref()) {
                read.rows = Some(rows);
            }
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
                    cx.notify();
                }
            }),
        );
    }

    /// An artifact he folded away or restored here is drawn so at once, before Fleet's list says it.
    pub(super) fn archived_here(&mut self, job: &str, artifact: &str, on: bool) {
        self.artifacts.folded_here.insert((job.to_string(), artifact.to_string()), on);
        if let Some(one) = self.artifacts.read.get_mut(job).and_then(|read| read.rows.as_mut()).and_then(|rows| rows.iter_mut().find(|one| one.id == artifact)) {
            one.archived = on.then(|| ArtifactArchived { by: ArtifactBy::Owner, at: Instant::carried(Local::now().to_rfc3339()) });
        }
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

    /// The sealed page drawn once at the card's width by the web view the app lends, which runs none of its script.
    fn paint_picture(&mut self, key: PictureKey, html: Result<String, String>, window: &mut Window, cx: &mut Context<Self>) {
        use crate::scene::mock_view::Appearance;
        let painter = crate::scene::mock_view::painter(cx);
        let (Ok(html), Some(painter)) = (html, painter) else {
            self.picture_made(key, Err(String::new()), window, cx);
            return;
        };
        let appearance = if key.dark { Appearance::Dark } else { Appearance::Light };
        let task = painter(html, key.width as f32, window.scale_factor(), appearance, cx);
        cx.spawn_in(window, async move |this, cx| {
            let made = task.await;
            let _ = this.update_in(cx, |this, window, cx| this.picture_made(key, made, window, cx));
        })
        .detach();
    }

    fn picture_made(&mut self, key: PictureKey, made: Result<(Arc<RenderImage>, f32), String>, window: &mut Window, cx: &mut Context<Self>) {
        self.artifacts.making = self.artifacts.making.saturating_sub(1);
        // An older width or theme of the same revision is let go.
        self.artifacts.pictures.retain(|held, _| held == &key || !(held.job == key.job && held.page == key.page && held.revision == key.revision));
        self.artifacts.pictures.insert(key, made.map_or(Picture::Failed, |(image, height)| Picture::Ready(image, height)));
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

    /// One artifact: its kind's tile, its title over what it is for, its revision, where it stands and when it was kept,
    /// each in a column of its own so the rows read down the list.
    #[allow(clippy::too_many_arguments)]
    fn artifact_row(&self, id: &SharedString, one: &JobArtifact, on: bool, folded: bool, now: DateTime<Local>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (key, job) = (one.id.clone(), id.to_string());
        let (_, _, word) = kind_look(one.form);
        let last = one.revisions.last();
        let number = last.map_or(1, |last| last.number);
        let said = last.map_or_else(String::new, |last| super::band_card::revision_said(last.number, last.by, last.at.as_str()));
        let line = one.for_line.as_ref().and_then(|line| line.text.clone()).filter(|text| !text.trim().is_empty());
        let column = |width: gpui::Pixels| div().flex_none().w(width).flex().items_center();
        let (revision_w, state_w, time_w) = (look.length(token::SPACE_12) + look.length(token::SPACE_3), look.length(token::SPACE_12) * 2. + look.length(token::SPACE_4), look.length(token::SPACE_12) + look.length(token::SPACE_3));
        let row = div()
            .id(SharedString::from(format!("artifact-row:{key}")))
            .flex()
            .items_center()
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
                    })),
            )
            .child(column(revision_w).child(revision_chip(look, format!("artifact-revision:{key}"), number, said)))
            .child(column(state_w).child(state_chip(look, format!("artifact-state:{key}"), Stand::of(one.state), one.words.as_deref(), None)))
            .child(column(time_w).justify_end().children(last.map(|last| self.stamp(&last.at, look, now, true, cx))));
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

    /// A press on a row opens the artifact as its own card opens it: a page in its tab, a draft in its tab, a sketch over the window.
    fn open_artifact(&mut self, job: &str, artifact: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = self.artifact_of(job, artifact).cloned() else { return };
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
                let card = self.thread(job, cx).and_then(|held| {
                    held.items.iter().rev().find_map(|item| match item {
                        ThreadItem::Sketch(card) if card.sketch == one.id => Some(card.id.clone()),
                        _ => None,
                    })
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
}
