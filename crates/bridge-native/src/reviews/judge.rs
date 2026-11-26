//! Judge on a pull request a Job holds: a press in the page's head asks that Job's Judge to read it, the button and the
//! inbox row carry the reading's mark, and its notes stand under Review comments, each with Send to job and Dismiss.
//!
//! **Only a press starts a reading, and a note reaches the Job only if he sends it.** The reading rides on the holding Job's
//! record, which the Jobs list keeps current, so nothing here holds a copy of it past the frame that drew it. A pull
//! request no Job holds has no Judge: the Job is the one Fleet asks.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui::{anchored, deferred, div, point, prelude::*, px, Anchor, AnyElement, App, Context, Entity, FocusHandle, FontWeight, KeyDownEvent, MouseButton, MouseDownEvent, Role, SharedString, Subscription, Task, Window};
use ipc::{JudgeArtifact, JudgeKind, JudgeNote, JudgeNoteHow, JudgeReading, JudgeState};

use super::model::Page;
use super::{Boxed, ReviewsScreen};
use crate::generated::actions::ACTIONS;
use crate::jobs::judging::{mark_face, mark_of_reading, Mark};
use crate::jobs::JobsList;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::{glyph, mono, tip, Btn, Weight};

/// The registry's act for the Judge press on a page.
const JUDGE: &str = "review_judge";

/// The Job a walk's mock page names as its holder.
const MOCK_JOB: &str = "mock-holder";

/// How long the walk's mock takes to read, when the press is made.
const MOCK_READ: Duration = Duration::from_millis(1200);

/// The Job holding a pull request: who the Judge is asked through, and what the page calls it.
#[derive(Clone, Debug)]
pub(super) struct Holder {
    pub id: String,
    pub title: String,
}

/// The box a press opened, over the page.
pub(super) struct Open {
    pub number: u64,
    /// Where the words in the box are kept, as every text box on the page keeps them.
    pub keep: String,
    pub field: Boxed,
    /// The row of judge types, one stop for Tab where ← and → move the choice.
    pub chips: FocusHandle,
    pub stop: FocusHandle,
    /// The id of the judge type chosen.
    pub chosen: String,
    /// Fleet's words where the ask was refused.
    pub said: Option<String>,
    pub sending: bool,
}

/// What the page knows of the Judge beyond what the holding Job's record says.
pub(super) struct Judging {
    pub jobs: Option<Entity<JobsList>>,
    /// The walk's readings, by pull request: the mock has no Job to keep them.
    pub mock: HashMap<u64, JudgeReading>,
    pub types: crate::judge_types::Held,
    pub open: Option<Open>,
    /// Pull requests asked and not yet answered by a new reading: the id of the reading held at the press.
    pub asked: HashMap<u64, Option<String>>,
    /// Notes settled here, by reading and note, until Fleet's record says so.
    pub settled: HashMap<String, JudgeNoteHow>,
    /// Why Send or Dismiss was refused, by pull request.
    pub refused: HashMap<u64, String>,
    /// Readings whose notes were shown, so Fleet is told once.
    pub marked: HashSet<String>,
    pub mock_runs: HashMap<u64, Task<()>>,
    pub watch: Option<Subscription>,
}

impl Judging {
    pub fn new(mock: HashMap<u64, JudgeReading>) -> Self {
        Judging { jobs: None, mock, types: Default::default(), open: None, asked: HashMap::new(), settled: HashMap::new(), refused: HashMap::new(), marked: HashSet::new(), mock_runs: HashMap::new(), watch: None }
    }
}

/// The key the registry gives an act, as a tooltip says it.
fn shortcut_of(id: &str) -> &'static str {
    ACTIONS.iter().find(|row| row.id == id).map_or("", |row| row.shortcut)
}

/// `path` of `path:12` or `path:12-20`: where a note's place names a file.
fn path_of(place: &str) -> &str {
    let place = place.trim();
    match place.rsplit_once(':') {
        Some((path, line)) if !line.is_empty() && line.chars().all(|ch| ch.is_ascii_digit() || ch == '-') => path,
        _ => place,
    }
}

impl ReviewsScreen {
    /// The Jobs list the holding Jobs' records are read from. Called where the screen is built beside it.
    pub fn watch_jobs(&mut self, list: Entity<JobsList>, cx: &mut Context<Self>) {
        self.judging.watch = Some(cx.observe(&list, |this, _, cx| {
            this.judge_answered(cx);
            cx.notify();
        }));
        self.judging.jobs = Some(list);
    }

    /// A press waits on a new reading; once the record holds one, the wait is over.
    fn judge_answered(&mut self, cx: &mut Context<Self>) {
        let done: Vec<u64> = self.judging.asked.iter().filter(|(number, held)| self.judge_reading(**number, cx).is_some_and(|one| Some(&one.id) != held.as_ref())).map(|(number, _)| *number).collect();
        for number in done {
            self.judging.asked.remove(&number);
        }
    }

    /// The Job holding a pull request, as the list's row or the walk's page names it.
    pub(super) fn holder_of(&self, number: u64) -> Option<Holder> {
        match &self.live {
            Some(live) => live.pulls.get(&number)?.row.holder.as_ref().map(|holder| Holder { id: holder.id.clone(), title: holder.title.clone() }),
            None => self.pages.get(&number)?.holder.clone().map(|title| Holder { id: MOCK_JOB.to_string(), title }),
        }
    }

    /// The newest reading of a pull request by the Job holding it.
    pub(super) fn judge_reading<'a>(&'a self, number: u64, cx: &'a App) -> Option<&'a JudgeReading> {
        if !self.is_live() {
            return self.judging.mock.get(&number);
        }
        let holder = self.holder_of(number)?;
        let id = number.to_string();
        let record = self.judging.jobs.as_ref()?.read(cx).record(&holder.id)?;
        record.judge_readings.iter().filter(|one| one.artifact.kind == JudgeKind::PullRequest && one.artifact.id == id).max_by(|a, b| a.asked_at.as_str().cmp(b.asked_at.as_str()))
    }

    /// What the page's mark says: reading, notes waiting or clean, and the words for its tooltip.
    pub(super) fn judge_mark(&self, number: u64, cx: &App) -> Option<(Mark, String)> {
        let reading = self.judge_reading(number, cx);
        if let Some(held) = self.judging.asked.get(&number) {
            if reading.map(|one| &one.id) == held.as_ref() {
                return Some((Mark::Reading, Mark::Reading.said(None)));
            }
        }
        let reading = reading?;
        let mark = mark_of_reading(reading)?;
        Some((mark, mark.said(Some(&crate::judge_types::name_of(reading)))))
    }

    /// Where the button was last drawn, which the box hangs from and whose own press is not an outside one.
    fn judge_button_at(&self, window: &Window, cx: &App) -> Option<gpui::Bounds<gpui::Pixels>> {
        let number = self.judging.open.as_ref()?.number;
        let name = self.judge_name(number, cx);
        cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), Role::Button, &name).map(|drawn| drawn.bounds))
    }

    /// The press's name, which says where the reading stands.
    fn judge_name(&self, number: u64, cx: &App) -> String {
        match self.judge_mark(number, cx) {
            Some((_, said)) => format!("Judge, {said}"),
            None => "Judge".to_string(),
        }
    }

    /// The Judge's press in the head, where a Job holds the pull request: its mark, its Job in the tooltip, and the box under it.
    pub(super) fn judge_button(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let holder = self.holder_of(page.number)?;
        let mark = self.judge_mark(page.number, cx);
        let key = shortcut_of(JUDGE);
        let said = match (&mark, self.judge_reading(page.number, cx).and_then(|one| one.failed.clone())) {
            (Some((_, said)), _) => format!("{said}. Job {}", holder.title),
            (None, Some(why)) => format!("{why}. Job {}", holder.title),
            (None, None) => format!("Ask the Judge of {} to read it", holder.title),
        };
        let said = if key.is_empty() { said } else { format!("{said} ({key})") };
        let open = self.judging.open.as_ref().is_some_and(|open| open.number == page.number);
        let number = page.number;
        let button = Btn::new("page-judge", "Judge", Weight::Secondary)
            .icon("scale")
            .draw(look, cx)
            .when(open, |button| button.bg(look.color(token::BG_HOVER)))
            .tooltip(tip(said))
            .on_click(cx.listener(|this, _, window, cx| this.toggle_judge(window, cx)))
            .children(mark.map(|(mark, _)| mark_face(mark, &format!("page-{number}"), look)));
        Some(labelled(button, Role::Button, self.judge_name(number, cx)).into_any_element())
    }

    /// An inbox row's mark of the same reading, in a column of its own so the titles line up.
    pub(super) fn judge_row_mark(&self, number: u64, look: &Look, cx: &App) -> Option<AnyElement> {
        let (mark, said) = self.judge_mark(number, cx)?;
        let face = div().id(SharedString::from(format!("judge-mark-{number}"))).flex().items_center().justify_center().tooltip(tip(said.clone())).child(mark_face(mark, &format!("row-{number}"), look));
        Some(labelled(face, Role::Image, said).into_any_element())
    }

    // ---- The box -----------------------------------------------------------------------------------------

    /// The press, or its key: the box opens under the button, or shuts.
    pub(super) fn toggle_judge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open.filter(|number| self.holder_of(*number).is_some()) else { return };
        if self.judging.open.as_ref().is_some_and(|open| open.number == number) {
            self.close_judge(true, window, cx);
        } else {
            self.open_judge(number, window, cx);
        }
    }

    fn open_judge(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repository) = self.pages.get(&number).map(|page| page.repository.clone()) else { return };
        let keep = format!("judge:{repository}#{number}");
        let field = self.boxed(&keep, None, 3, 8, window, cx);
        let area = field.state.clone();
        self.judging.open = Some(Open {
            number,
            keep,
            field,
            chips: cx.focus_handle().tab_stop(true),
            stop: cx.focus_handle(),
            chosen: crate::judge_types::DEFAULT.to_string(),
            said: None,
            sending: false,
        });
        self.read_judge_types(number, window, cx);
        area.update(cx, |area, cx| area.focus(window, cx));
        cx.notify();
    }

    pub(super) fn close_judge(&mut self, refocus: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.judging.open = None;
        if refocus {
            window.focus(&self.within, cx);
        }
        cx.notify();
    }

    /// The key the types of a pull request are held under, and where they are read for.
    pub(super) fn judge_place(&self, number: u64, cx: &App) -> Option<(String, Option<String>)> {
        let Some(live) = &self.live else { return Some(("mock".to_string(), None)) };
        let holder = self.holder_of(number)?;
        let record = self.judging.jobs.as_ref().and_then(|list| list.read(cx).record(&holder.id));
        let repository = record.and_then(|record| record.manifest_id.as_ref()).map(|id| id.as_str().to_string()).or_else(|| live.pulls.get(&number).map(|pull| pull.row.manifest.clone()))?;
        let project = record.and_then(|record| record.hosted.as_ref()).and_then(|hosted| hosted.project.clone()).filter(|project| !project.trim().is_empty());
        Some((repository, project))
    }

    /// The types read for the pull request's Job: none yet, or what Fleet said.
    fn judge_types_read(&self, number: u64, cx: &App) -> crate::judge_chips::Read<'_> {
        let (repository, project) = self.judge_place(number, cx)?;
        self.judging.types.read.get(&crate::judge_types::key(&repository, project.as_deref()))
    }

    fn read_judge_types(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_live() {
            self.live_judge_types(number, window, cx);
        } else if let Some((repository, project)) = self.judge_place(number, cx) {
            self.judging.types.read.insert(crate::judge_types::key(&repository, project.as_deref()), Ok(crate::mock::judge_types(project.as_deref())));
        }
    }

    fn choose_judge_type(&mut self, id: String, cx: &mut Context<Self>) {
        if let Some(open) = self.judging.open.as_mut() {
            open.chosen = id;
        }
        cx.notify();
    }

    /// ← and → move the choice one type along the row; the ends hold.
    fn step_judge_type(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(open) = self.judging.open.as_ref() else { return };
        let Some(Ok(list)) = self.judge_types_read(open.number, cx) else { return };
        if let Some(id) = crate::judge_chips::stepped(list, &open.chosen, delta) {
            crate::ui_sounds::moved();
            self.choose_judge_type(id, cx);
        }
    }

    /// The box under the button: the judge types, what to look at in particular, and Judge. Drawn over the page, where
    /// nothing clips it.
    pub(super) fn render_judge_box(&self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let open = self.judging.open.as_ref().filter(|open| self.open == Some(open.number))?;
        let page = self.pages.get(&open.number)?;
        self.holder_of(open.number)?;
        let key = format!("pr-{}", open.number);
        let read = self.judge_types_read(open.number, cx);
        let shown = !matches!(read, Some(Ok(list)) if list.is_empty());
        let chips = shown.then(|| {
            crate::judge_chips::chips(
                &key,
                read,
                &open.chosen,
                &open.chips,
                look,
                cx,
                |this: &mut Self, id, window, cx| {
                    this.choose_judge_type(id, cx);
                    if let Some(open) = this.judging.open.as_ref() {
                        window.focus(&open.chips, cx);
                    }
                },
                |this: &mut Self, delta, _, cx| this.step_judge_type(delta, cx),
            )
        });
        let wide = look.length(token::SIDEBAR_DEFAULT) * 1.6;
        let at = self.judge_button_at(window, cx).map_or(point(px(0.), px(0.)), |bounds| point(bounds.origin.x, bounds.origin.y + bounds.size.height));
        let margin = look.length(token::SPACE_2);
        let area = open.field.state.clone();
        let stops = (open.chips.clone(), open.stop.clone(), chips.is_some());
        let field = crate::judge_chips::prompt_field(&key, &open.field.state, look);
        let go = labelled(
            Btn::new("judge-run", "Judge", Weight::Primary)
                .pending(open.sending)
                .draw(look, cx)
                .track_focus(&open.stop)
                .on_click(cx.listener(|this, _, window, cx| this.run_judge(window, cx))),
            Role::Button,
            "Judge",
        );
        let title = format!("Judge #{} {}", page.number, page.title);
        let panel = labelled(
            div()
                .id("judge-pop")
                .occlude()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .w(wide)
                .max_w_full()
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    // A key made in the box is not heard by the page under it, except a letter, which a Mac hands to the
                    // box's input only once no listener has stopped it.
                    let typed = &event.keystroke;
                    let letter = typed.key_char.is_some() && !(typed.modifiers.control || typed.modifiers.platform || typed.modifiers.function);
                    if !letter || matches!(typed.key.as_str(), "escape" | "tab") {
                        cx.stop_propagation();
                    }
                    if typed.key == "escape" {
                        this.close_judge(true, window, cx);
                    } else if typed.key == "tab" {
                        // Tab goes round the box: its field, the judge types, then Judge.
                        let mut order = vec![gpui::Focusable::focus_handle(area.read(cx), cx)];
                        order.extend(stops.2.then(|| stops.0.clone()));
                        order.push(stops.1.clone());
                        trap_tab(&order, typed.modifiers.shift, window, cx);
                    }
                }))
                .on_mouse_down_out(cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    // A press on the button itself is its own toggle, not an outside press.
                    if !this.judge_button_at(window, cx).is_some_and(|bounds| bounds.contains(&event.position)) {
                        this.close_judge(false, window, cx);
                    }
                }))
                .child(div().min_w_0().truncate().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(title)))
                .children(chips)
                .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Anything in particular?"))
                .child(field)
                .children(open.said.clone().map(|why| crate::judge_chips::refusal(look, &key, why.into())))
                .child(div().flex().justify_end().child(go)),
            Role::Dialog,
            "Judge",
        );
        let at = point(at.x, at.y + look.length(token::SPACE_1));
        Some(deferred(anchored().anchor(Anchor::TopLeft).position(at).snap_to_window_with_margin(margin).child(panel)).with_priority(1).into_any_element())
    }

    /// Ask the Judge: the button reads as reading in this frame, and Fleet's reading replaces it.
    pub(super) fn run_judge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.judging.open.as_ref().filter(|open| !open.sending) else { return };
        let number = open.number;
        let (Some(holder), Some(page)) = (self.holder_of(number), self.pages.get(&number)) else { return };
        let artifact = JudgeArtifact { kind: JudgeKind::PullRequest, id: number.to_string(), title: page.title.clone() };
        let prompt = open.field.state.read(cx).value().trim().to_string();
        let judge = match self.judge_types_read(number, cx) {
            Some(Ok(list)) => crate::judge_chips::chosen_in(list, &open.chosen).map(|one| one.id.clone()),
            _ => None,
        };
        let held = self.judge_reading(number, cx).map(|one| one.id.clone());
        self.judging.asked.insert(number, held);
        if let Some(open) = self.judging.open.as_mut() {
            open.sending = true;
            open.said = None;
        }
        if self.is_live() {
            self.live_judge_ask(holder, artifact, prompt, judge, window, cx);
        } else {
            let run = cx.spawn(async move |this, cx| {
                cx.background_executor().timer(MOCK_READ).await;
                let _ = this.update(cx, |this, cx| {
                    this.judging.asked.remove(&number);
                    this.judge_asked_done(number, cx);
                });
            });
            self.judging.mock_runs.insert(number, run);
        }
        cx.notify();
    }

    /// Fleet took the ask: the box goes, with the words in it.
    pub(super) fn judge_asked_done(&mut self, number: u64, cx: &mut Context<Self>) {
        if let Some(open) = self.judging.open.take_if(|open| open.number == number) {
            crate::drafts::clear(&open.keep, cx);
        }
        cx.notify();
    }

    /// Fleet refused: the box stays with its words, and Fleet's reason stands in it.
    pub(super) fn judge_asked_refused(&mut self, number: u64, why: String, cx: &mut Context<Self>) {
        self.judging.asked.remove(&number);
        if let Some(open) = self.judging.open.as_mut().filter(|open| open.number == number) {
            open.sending = false;
            open.said = Some(why);
        }
        cx.notify();
    }

    /// A key of the registry made on the page.
    pub(super) fn on_review_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if act.is(JUDGE) && self.open.is_some_and(|number| self.holder_of(number).is_some()) {
            self.toggle_judge(window, cx);
        } else {
            cx.propagate();
        }
    }

    // ---- The notes ---------------------------------------------------------------------------------------

    /// Judged as the type: the reading's notes under Review comments, a header band in the Judge's hue over them. A reading
    /// going says so, a failed one says why, and one with nothing to say says that.
    pub(super) fn judge_section(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let holder = self.holder_of(page.number)?;
        let reading = self.judge_reading(page.number, cx).cloned();
        let going = self.judging.asked.get(&page.number).is_some_and(|held| reading.as_ref().map(|one| &one.id) == held.as_ref());
        if reading.is_none() && !going {
            return None;
        }
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let name = reading.as_ref().filter(|_| !going).map(crate::judge_types::name_of);
        let head = match &name {
            Some(name) => format!("Judged as {name}"),
            None => "Judge".to_string(),
        };
        if let Some(reading) = reading.as_ref().filter(|one| !going && one.state == JudgeState::Done && !one.notes.is_empty()) {
            self.judge_read(&holder, reading, cx);
        }
        let mark = reading.as_ref().filter(|_| !going).and_then(mark_of_reading).or(going.then_some(Mark::Reading));
        let body: Vec<AnyElement> = match reading.as_ref().filter(|_| !going) {
            None => vec![self.judge_line("judge-reading", "Judge reading", false, look)],
            Some(one) => match one.state {
                JudgeState::Reading => vec![self.judge_line("judge-reading", "Judge reading", false, look)],
                JudgeState::Failed => vec![self.judge_line("judge-failed", one.failed.as_deref().unwrap_or("The Judge could not read it"), true, look)],
                JudgeState::Done if one.notes.is_empty() => vec![self.judge_line("judge-clean", "Nothing missed", false, look)],
                JudgeState::Done => one.notes.iter().map(|note| self.judge_note(&holder, one, note, page, look, cx)).collect(),
            },
        };
        let refused = self.judging.refused.get(&page.number).cloned().map(|why| crate::judge_chips::refusal(look, "judge-note", why.into()));
        let band = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .bg(hue.opacity(0.14))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(hue.opacity(0.3))
            .rounded_t(super::card_inner_radius(look))
            .child(glyph("scale", look.length(token::SPACE_4), hue))
            .child(labelled(
                div().id("judge-head").flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(hue).child(SharedString::from(head.clone())),
                Role::Heading,
                head.clone(),
            ))
            .children(mark.map(|mark| div().flex_none().child(mark_face(mark, &format!("section-{}", page.number), look))));
        Some(
            labelled(
                div()
                    .id("judge-notes")
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(hue.opacity(0.45))
                    .bg(look.color(token::STATUS_AWAITING_REVIEW_BG))
                    .child(band)
                    .child(div().flex().flex_col().gap(look.length(token::SPACE_2)).p(look.length(token::SPACE_3)).children(body).children(refused)),
                Role::Group,
                head,
            )
            .into_any_element(),
        )
    }

    /// One line of the section: a state, in words.
    fn judge_line(&self, id: &'static str, said: &str, failed: bool, look: &Look) -> AnyElement {
        labelled(
            div()
                .id(id)
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(if failed { token::STATUS_COMPLETED_FAILED } else { token::FG_MUTED }))
                .child(SharedString::from(said.to_string())),
            Role::Status,
            said.to_string(),
        )
        .into_any_element()
    }

    /// How a note stands: Fleet's record says, else what was pressed here.
    fn judge_how(&self, reading: &JudgeReading, note: &JudgeNote) -> Option<JudgeNoteHow> {
        note.settled.as_ref().map(|settled| settled.how).or_else(|| self.judging.settled.get(&format!("{}/{}", reading.id, note.id)).copied())
    }

    /// One note: its claim and where it is, with Send to job and Dismiss, or, once settled, one muted line.
    fn judge_note(&self, holder: &Holder, reading: &JudgeReading, note: &JudgeNote, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let mark = format!("{}-{}", reading.id, note.id);
        if let Some(how) = self.judge_how(reading, note) {
            let (icon, said) = match how {
                JudgeNoteHow::Sent => ("arrow-right", "Sent"),
                JudgeNoteHow::Dismissed => ("ban", "Dismissed"),
            };
            return labelled(
                div()
                    .id(SharedString::from(format!("judge-settled-{mark}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(div().id(SharedString::from(format!("judge-settled-mark-{mark}"))).flex_none().tooltip(tip(said)).child(glyph(icon, look.length(token::SPACE_4), look.color(token::FG_MUTED))))
                    .child(div().min_w_0().truncate().child(SharedString::from(note.claim.clone()))),
                Role::Group,
                format!("Note {}: {}", said.to_lowercase(), note.claim),
            )
            .into_any_element();
        }
        // The file the note is about, where the page holds it: a press opens Files there.
        let path = note.cites.iter().map(String::as_str).chain(note.r#where.as_deref()).map(path_of).find(|path| page.files.iter().any(|file| file.path == *path)).map(str::to_string);
        let place = note.r#where.clone().or_else(|| note.cites.first().cloned());
        let place = place.map(|place| match path {
            Some(path) => {
                let said = format!("Open {path} in Files");
                labelled(
                    div()
                        .id(SharedString::from(format!("judge-where-{mark}")))
                        .min_w_0()
                        .truncate()
                        .cursor_pointer()
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::ACCENT))
                        .hover(|style| style.underline())
                        .tooltip(tip(said.clone()))
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .on_click(cx.listener(move |this, _, window, cx| this.show_file(&path, window, cx)))
                        .child(SharedString::from(place)),
                    Role::Button,
                    said,
                )
                .into_any_element()
            }
            None => labelled(
                div()
                    .id(SharedString::from(format!("judge-where-{mark}")))
                    .min_w_0()
                    .truncate()
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .tooltip(tip(place.clone()))
                    .child(SharedString::from(place.clone())),
                Role::Label,
                format!("Where: {place}"),
            )
            .into_any_element(),
        });
        let act = |id: &'static str, said: &'static str, name: String, hint: String, weight: Weight, how: JudgeNoteHow| {
            let (holder, reading, note) = (holder.id.clone(), reading.id.clone(), note.id.clone());
            let number = page.number;
            labelled(
                Btn::new(SharedString::from(format!("{id}-{mark}")), said, weight)
                    .draw(look, cx)
                    .tooltip(tip(hint))
                    .on_click(cx.listener(move |this, _, window, cx| this.settle_judge_note(number, &holder, &reading, &note, how, window, cx))),
                Role::Button,
                name,
            )
        };
        labelled(
            div()
                .id(SharedString::from(format!("judge-note-{mark}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .min_w_0()
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(hue.opacity(0.45))
                .bg(look.color(token::BG_RAISED))
                .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(note.claim.clone())))
                .children(place)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .flex_wrap()
                        .gap(look.length(token::SPACE_1))
                        .child(act("judge-send", "Send to job", format!("Send to {}", holder.title), format!("Goes to {} as your message", holder.title), Weight::Secondary, JudgeNoteHow::Sent))
                        .child(act("judge-dismiss", "Dismiss", format!("Dismiss: {}", note.claim), "Leaves it unsent".to_string(), Weight::Ghost, JudgeNoteHow::Dismissed)),
                ),
            Role::Group,
            format!("Note: {}", note.claim),
        )
        .into_any_element()
    }

    /// Send a note to the Job or put it away. It reads as settled in this frame; a refusal takes it back.
    #[allow(clippy::too_many_arguments)]
    fn settle_judge_note(&mut self, number: u64, holder: &str, reading: &str, note: &str, how: JudgeNoteHow, window: &mut Window, cx: &mut Context<Self>) {
        self.judging.settled.insert(format!("{reading}/{note}"), how);
        self.judging.refused.remove(&number);
        if self.is_live() {
            self.live_judge_settle(number, holder, reading, note, how, window, cx);
        }
        cx.notify();
    }

    /// The notes are shown, so they are read: Fleet is told once per reading.
    fn judge_read(&mut self, holder: &Holder, reading: &JudgeReading, cx: &mut Context<Self>) {
        if reading.read || !self.judging.marked.insert(reading.id.clone()) || !self.is_live() {
            return;
        }
        let (holder, reading) = (holder.id.clone(), reading.id.clone());
        let me = cx.weak_entity();
        cx.defer(move |cx| {
            let _ = me.update(cx, |this, cx| this.live_judge_read(&holder, &reading, cx));
        });
    }
}

/// Tab inside the box goes round its own stops, forward or back, and never out to what is behind it.
fn trap_tab(order: &[FocusHandle], back: bool, window: &mut Window, cx: &mut App) {
    let Some(last) = order.len().checked_sub(1) else { return };
    let at = order.iter().position(|stop| stop.contains_focused(window, cx));
    let next = match (at, back) {
        (Some(at), false) => (at + 1) % (last + 1),
        (Some(at), true) => (at + last) % (last + 1),
        (None, false) => 0,
        (None, true) => last,
    };
    window.focus(&order[next], cx);
    cx.stop_propagation();
}

#[cfg(test)]
mod tests {
    use super::path_of;

    #[test]
    fn a_place_names_its_file_with_or_without_its_line() {
        assert_eq!(path_of("src/webhooks/retry.rs:21"), "src/webhooks/retry.rs");
        assert_eq!(path_of("src/webhooks/retry.rs:21-30"), "src/webhooks/retry.rs");
        assert_eq!(path_of("src/webhooks/retry.rs"), "src/webhooks/retry.rs");
        assert_eq!(path_of("Plan, task Check the token"), "Plan, task Check the token");
    }
}
