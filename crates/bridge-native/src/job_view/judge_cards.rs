//! The Judge where the person reads: on selected words, on the cards of the thread and on an agent's reply, and the
//! Judged card at the thread's end that the same reading draws. `judge.rs` holds the press, its box and the notes; this
//! file places them.
//!
//! **One reading is drawn in two places.** A card's notes stand inside the card, on the part they name where one
//! matches and else at its foot, and the Judged card at the end of the thread draws them again. Both draw the one
//! reading, so Send to Job or Dismiss in either settles both.

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, Animation, AnimationExt, Context, FontWeight, KeyDownEvent, MouseButton, Role, SharedString, Window};
use ipc::{Instant, JudgeArtifact, JudgeKind, JudgeReading, JudgeState, SessionRow, ThreadItem};

use super::judge::{how_of, reading_of, Pinned};
use super::rows::{glyph, mono, tip, Size};
use super::{Extra, JobView, Page};
use crate::dropdown::Entry;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The kinds a person judges where he reads the thread: their notes are drawn in the thread.
pub(super) fn in_thread(kind: JudgeKind) -> bool {
    matches!(kind, JudgeKind::Excerpt | JudgeKind::Draft | JudgeKind::Branch | JudgeKind::Reply | JudgeKind::Sketch | JudgeKind::Doc)
}

/// FNV-1a over the words: the same words hash alike on every run.
fn hash_of(words: &str) -> u32 {
    words.bytes().fold(0x811c_9dc5, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193))
}

/// Words selected in the row `row`, as a judge is asked to read them: the row and eight hex digits of the words' hash.
pub(super) fn excerpt_of(row: &str, words: &str) -> JudgeArtifact {
    let words = words.trim();
    JudgeArtifact { kind: JudgeKind::Excerpt, id: format!("{row}/{:08x}", hash_of(words)), title: words.to_string() }
}

/// The press key of the selection bar's Judge: one to the words.
pub(super) fn excerpt_key(artifact: &JudgeArtifact) -> String {
    format!("selection:{}", artifact.id)
}

/// The press key of a card's Judge: one to the thing it reads.
fn card_key(artifact: &JudgeArtifact) -> String {
    format!("on:{:?}:{}", artifact.kind, artifact.id)
}

/// A menu entry's id lives as long as the app: kept once however often the menu opens.
pub(super) fn interned(text: String) -> &'static str {
    static HELD: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, &'static str>>> = std::sync::OnceLock::new();
    let mut held = HELD.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    held.entry(text.clone()).or_insert_with(|| Box::leak(text.into_boxed_str()))
}

/// What a card's Judge draws: the press for its foot or bar, and what the last reading left, for inside the card.
pub(super) struct OnCard {
    pub press: AnyElement,
    pub notes: Option<AnyElement>,
}

/// The thread's list place of what a reading read, where the thread draws it: the item, or for a build the pinned card.
fn card_place(items: &[ThreadItem], extras: &[Extra], artifact: &JudgeArtifact) -> Option<usize> {
    let item = items.iter().position(|item| match (artifact.kind, item) {
        (JudgeKind::Sketch, ThreadItem::Sketch(card)) => card.sketch.strip_prefix("sketch:") == Some(artifact.id.as_str()),
        (JudgeKind::Doc, ThreadItem::Document(card)) => card.page == artifact.id,
        (JudgeKind::Draft, ThreadItem::Row { row: SessionRow::Draft { id, .. } }) => *id == artifact.id,
        (JudgeKind::Reply, ThreadItem::Row { row: SessionRow::Message { id, .. } }) => *id == artifact.id,
        (JudgeKind::Branch, ThreadItem::ReadyToTry(card)) => card.branch == artifact.id,
        _ => false,
    });
    // An open build is drawn in the pinned card at the thread's end, not where it was offered.
    let pinned = (artifact.kind == JudgeKind::Branch).then(|| extras.iter().position(|extra| matches!(extra, Extra::Ready)).map(|at| items.len() + at)).flatten();
    pinned.or(item)
}

impl JobView {
    // ---- On selected words -----------------------------------------------------------------

    /// Judge, in the selection bar beside Ask: its box opens under it and asks for a reading of the selected words.
    pub(super) fn selection_judge(&self, row: &str, words: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let artifact = excerpt_of(row, words);
        let key = excerpt_key(&artifact);
        let job = self.shown.clone().unwrap_or_default();
        let Some(page) = self.pages.get(&job) else { return gpui::Empty.into_any_element() };
        let side = look.length(token::H_CONTROL_SM);
        let spot = page.keys.spot(&format!("judge-press:{key}"), cx);
        let edges = spot.bounds.clone();
        let open = self.judge_open_at(&key);
        let shortcut = crate::generated::actions::ACTIONS.iter().find(|one| one.id == "selection_judge").map_or("", |one| one.shortcut);
        let press = labelled(
            div()
                .id("selection-judge")
                .track_focus(&spot.focus)
                .relative()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(side)
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .focus_visible(|style| style.bg(look.color(token::BG_HOVER)))
                .when(open, |press| press.bg(look.color(token::BG_HOVER)))
                .tooltip(tip(format!("Judge ({shortcut})")))
                .child(glyph(look, "scale", Size::Mark, look.color(token::FG_MUTED)))
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                        cx.stop_propagation();
                        this.judge_selected(window, cx);
                    }
                }))
                .on_click(cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    this.judge_selected(window, cx);
                }))
                .child(gpui::canvas(move |bounds, _, _| edges.set(bounds), |_, _, _, _| {}).absolute().top_0().left_0().size_full()),
            Role::Button,
            "Judge",
        );
        let pop = open.then(|| self.judge_pop(super::acts::SHEET, &key, false, side, false, &spot.bounds, look, cx));
        div().relative().flex_none().child(press).children(pop).into_any_element()
    }

    // ---- On a card -------------------------------------------------------------------------

    /// A card's Judge: its press, and the notes of the reading of it. `card` is the card's own key, which the Judge's key
    /// finds it by.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn judge_on_card(&self, ix: usize, card: &str, artifact: JudgeArtifact, page: &Page, look: &Look, cx: &Context<Self>) -> Option<OnCard> {
        let job = self.shown.clone()?;
        let reading = self.record(&job, cx).and_then(|record| reading_of(record, artifact.kind, &artifact.id));
        let key = card_key(&artifact);
        page.cards.judgeable.borrow_mut().insert(card.to_string(), (ix, key.clone(), artifact.clone()));
        let press = self.judge_press(ix, &key, artifact, reading, &[], page, look, cx);
        let notes = self.card_notes(&key, reading, page, look, cx);
        Some(OnCard { press, notes })
    }

    /// What a reading of a card left, for inside the card: its notes, and what Fleet said where it refused. Its rows are
    /// settled from the whole list, since the Judged card draws the same notes.
    pub(super) fn card_notes(&self, key: &str, reading: Option<&JudgeReading>, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let notes = reading.and_then(|reading| Pinned::whole_job(super::acts::SHEET, key, reading)).and_then(|at| self.notes_list(&at, at.whole(), "Notes from the judge".to_string(), page, look, cx));
        let refused = page.cards.said.get(key).map(|why| super::acts::refusal(look, key, why.clone()));
        (notes.is_some() || refused.is_some()).then(|| div().flex().flex_col().min_w_0().gap(look.length(token::SPACE_2)).children(notes).children(refused).into_any_element())
    }

    /// Whether the box or the keys are on the Judge of a reply, which keeps the bar under the reply shown.
    pub(super) fn reply_judge_held(&self, id: &str, window: &Window) -> bool {
        let key = card_key(&JudgeArtifact { kind: JudgeKind::Reply, id: id.to_string(), title: String::new() });
        let focused = self.shown.as_ref().and_then(|job| self.pages.get(job)).and_then(|page| page.keys.spot_of(&format!("judge-press:{key}"))).is_some_and(|spot| spot.focus.is_focused(window));
        focused || self.judge_open_at(&key)
    }

    /// A reply's Judge and its notes, under the agent's words.
    pub(super) fn judge_on_reply(&self, id: &str, page: &Page, look: &Look, cx: &Context<Self>) -> Option<OnCard> {
        let artifact = JudgeArtifact { kind: JudgeKind::Reply, id: id.to_string(), title: "this reply".to_string() };
        self.judge_on_card(super::acts::SHEET, id, artifact, page, look, cx)
    }

    // ---- The Judged card -------------------------------------------------------------------

    /// A finished reading of words or of a card. Its head names what was read, the notes stand under it with Send to Job and
    /// Dismiss, and a press on the head of a card's reading scrolls to the card, where the same notes stand.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn judged_card(&self, id: &str, artifact: &JudgeArtifact, at: &Instant, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let job = self.shown.clone().unwrap_or_default();
        let record = self.record(&job, cx);
        let reading = record.and_then(|record| record.judge_readings.iter().find(|one| id.strip_prefix("judged:") == Some(one.id.as_str())).or_else(|| reading_of(record, artifact.kind, &artifact.id)));
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let words = artifact.kind == JudgeKind::Excerpt;
        let title = if words { "the selected words" } else { artifact.title.as_str() };
        let spot = page.keys.spot(&format!("judged:{id}"), cx);
        let judged_as = reading.map_or_else(|| "Judged".to_string(), |one| format!("Judged as {}", crate::judge_types::name_of(one)));
        let mut head = div()
            .id(SharedString::from(format!("judged-head:{id}")))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .bg(hue.opacity(0.14))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(hue.opacity(0.3))
            .child(glyph(look, "scale", Size::Mark, hue))
            .child(div().flex_none().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(hue).child(SharedString::from(judged_as.clone())))
            .children((!words).then(|| div().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(artifact.title.clone()))))
            .child(div().flex_1())
            .child(self.stamp(at, look, now, true, cx));
        if !words {
            let (go, goes) = (artifact.clone(), artifact.clone());
            head = head
                .track_focus(&spot.focus)
                .cursor_pointer()
                .hover(|style| style.bg(hue.opacity(0.2)))
                .focus_visible(|style| style.bg(hue.opacity(0.2)))
                .tooltip(tip("Go to its notes"))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                        cx.stop_propagation();
                        this.show_card_notes(&go, window, cx);
                    }
                }))
                .on_click(cx.listener(move |this, _, window, cx| this.show_card_notes(&goes, window, cx)));
        }
        let head = if words { labelled(head, Role::Group, format!("{judged_as}, the selected words")).into_any_element() } else { labelled(head, Role::Button, format!("{judged_as}, {title}")).into_any_element() };
        let quoted = words.then(|| quoted_words(look, &artifact.title));
        let body: Option<AnyElement> = match reading {
            Some(one) if one.state == JudgeState::Failed => Some(plain(look, one.failed.clone().unwrap_or_else(|| "The reading failed".to_string()), token::STATUS_COMPLETED_FAILED)),
            Some(one) if one.notes.is_empty() => Some(plain(look, "Nothing missed".to_string(), token::FG_MUTED)),
            Some(one) => Pinned::whole_job(super::acts::SHEET, &format!("judged:{id}"), one).map(Pinned::twin).and_then(|pinned| self.notes_list(&pinned, pinned.whole(), "Notes from the judge".to_string(), page, look, cx)),
            None => None,
        };
        let name = format!("{judged_as}, {title}");
        labelled(
            div()
                .id(SharedString::from(format!("judged:{id}")))
                .flex()
                .flex_col()
                .w_full()
                .min_w_0()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG) * 1.5)
                .bg(look.color(token::BG_RAISED))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(hue.opacity(0.45))
                .shadow(look.shadows(token::SHADOW_CARD))
                .child(head)
                .child(div().flex().flex_col().gap(look.length(token::SPACE_3)).min_w_0().p(look.length(token::SPACE_4)).children(quoted).children(body)),
            Role::Article,
            name,
        )
        .into_any_element()
    }

    /// A reading of selected words still going: the Judged card with the moving mark in place of the verdict.
    pub(super) fn judging_card(&self, reading: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let job = self.shown.clone().unwrap_or_default();
        let held = self.record(&job, cx).and_then(|record| record.judge_readings.iter().find(|one| one.id == reading));
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let words = held.map(|one| one.artifact.title.clone()).unwrap_or_default();
        let judging_as = held.map_or_else(|| "Judged".to_string(), |one| format!("Judged as {}", crate::judge_types::name_of(one)));
        let mark = glyph(look, "circle-dot", Size::Mark, look.color(token::STEP_RUNNING)).with_animation(
            SharedString::from(format!("judging:{reading}")),
            Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.4, 1.)),
            |mark, delta| mark.opacity(delta),
        );
        labelled(
            div()
                .id(SharedString::from(format!("judging:{reading}")))
                .flex()
                .flex_col()
                .w_full()
                .min_w_0()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG) * 1.5)
                .bg(look.color(token::BG_RAISED))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(hue.opacity(0.45))
                .shadow(look.shadows(token::SHADOW_CARD))
                .child(
                    div()
                        .id(SharedString::from(format!("judging-head:{reading}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_4))
                        .py(look.length(token::SPACE_3))
                        .bg(hue.opacity(0.14))
                        .tooltip(tip("Judge reading"))
                        .child(mark)
                        .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(hue).child(SharedString::from(judging_as))),
                )
                .child(div().p(look.length(token::SPACE_4)).child(quoted_words(look, &words))),
            Role::Article,
            "Judge reading the selected words",
        )
        .into_any_element()
    }

    /// The readings of selected words that are going, for the cards at the thread's end.
    pub(super) fn judging_words(&self, job: &str, cx: &gpui::App) -> Vec<Extra> {
        self.record(job, cx).map(|record| record.judge_readings.iter().filter(|one| one.artifact.kind == JudgeKind::Excerpt && one.state == JudgeState::Reading).map(|one| Extra::Judging(one.id.clone())).collect()).unwrap_or_default()
    }

    /// Scroll the thread to the card a reading read, and put the keys on the first note of it still waiting.
    pub(super) fn show_card_notes(&mut self, artifact: &JudgeArtifact, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if let (Some(page), Some(held)) = (self.pages.get(&job), self.thread(&job, cx)) {
            if let Some(at) = card_place(&held.items, &page.extras, artifact) {
                page.list.scroll_to_reveal_item(page.entry(at));
            }
        }
        cx.notify();
        let (view, artifact) = (cx.weak_entity(), artifact.clone());
        window.on_next_frame(move |window, cx| {
            view.update(cx, |this, cx| {
                let Some(job) = this.shown.clone() else { return };
                let Some(page) = this.pages.get(&job) else { return };
                let Some(reading) = this.record(&job, cx).and_then(|record| reading_of(record, artifact.kind, &artifact.id)) else { return };
                let first = reading.notes.iter().find(|note| how_of(page, reading, note).is_none()).or_else(|| reading.notes.first());
                if let Some(spot) = first.and_then(|note| page.keys.spot_of(&format!("finding:{}:{}", reading.id, note.id))) {
                    window.focus(&spot.focus, cx);
                    cx.notify();
                }
            })
            .ok();
        });
    }

    // ---- The keys --------------------------------------------------------------------------

    /// The Judge's acts of the registry. Whether one was answered.
    pub(super) fn judge_key_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if act.is("selection_judge") {
            return self.judge_selected(window, cx);
        }
        if act.is("card_judge") {
            let Some(page) = self.shown.as_ref().and_then(|job| self.pages.get(job)) else { return false };
            let card = page.keys.focused_card(window, cx).or_else(|| page.cards.target.clone());
            let held = card.and_then(|card| page.cards.judgeable.borrow().get(&card).cloned());
            let Some((ix, key, artifact)) = held else { return false };
            self.open_judge(ix, &key, artifact, window, cx);
            return true;
        }
        if act.is("palette_judge") {
            return self.judge_choose(window, cx);
        }
        false
    }

    /// What can be judged in the Job in focus: the Job, its plan and its Ledger artifacts.
    fn judgeable(&self, job: &str, cx: &gpui::App) -> Vec<JudgeArtifact> {
        let Some(record) = self.record(job, cx) else { return Vec::new() };
        let title = record.title.clone().filter(|title| !title.is_empty()).unwrap_or_else(|| "the Job".to_string());
        let mut found = vec![JudgeArtifact { kind: JudgeKind::Job, id: job.to_string(), title }];
        if let Some(plan) = &record.plan {
            found.push(super::judge::plan_artifact(plan.version));
        }
        for group in self.ledger_groups(&job.into(), cx) {
            found.extend(group.lines.iter().filter_map(|line| super::ledger::judged_of(&line.key, &line.title, group.id)));
        }
        found
    }

    /// A menu of what the Job holds, from the palette or a Jobs row: the one chosen opens the box of judge types.
    pub(super) fn judge_choose(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(job) = self.shown.clone() else { return false };
        let choices = self.judgeable(&job, cx);
        let entries: Vec<Entry> = choices
            .iter()
            .filter_map(|one| Some(Entry::Act { id: interned(ipc::encode(one).ok()?), label: SharedString::from(one.title.clone()), icon: Some("scale"), danger: false }))
            .collect();
        let Some(page) = self.pages.get(&job) else { return false };
        let view = page.list.viewport_bounds();
        let at = gpui::point(view.origin.x + view.size.width / 2., view.origin.y + gpui::px(96.));
        let back = self.focus.clone();
        self.open_pop(
            "Judge",
            entries,
            at,
            true,
            Some(back),
            |this, id, window, cx| {
                if let Ok(artifact) = ipc::decode::<JudgeArtifact>("a judge choice", id.as_bytes()) {
                    this.open_judge_sheet(artifact, window, cx);
                }
            },
            window,
            cx,
        );
        true
    }
}

/// The words a reading was asked to read, quoted in a sunken box.
fn quoted_words(look: &Look, words: &str) -> AnyElement {
    div()
        .min_w_0()
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .bg(look.color(token::BG_SUNKEN))
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .font_family(mono(look))
        .text_color(look.color(token::FG_MUTED))
        .child(SharedString::from(words.to_string()))
        .into_any_element()
}

/// One quiet line in a card's body.
fn plain(look: &Look, said: String, ink: crate::look::Token) -> AnyElement {
    div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(ink)).child(SharedString::from(said)).into_any_element()
}

/// The Job whose Judge tab is asked to come forward when it comes on screen: a press on a Jobs row's dot or check.
#[derive(Default)]
struct JudgeTabAsked(Option<String>);

impl gpui::Global for JudgeTabAsked {}

/// The Job whose list of judgeable parts is asked for when it comes on screen: a press on a Jobs row's Judge.
#[derive(Default)]
struct JudgeChoiceAsked(Option<String>);

impl gpui::Global for JudgeChoiceAsked {}

/// Open the list of what the Job holds to judge, now where the Job is on screen and else when it comes to be.
pub fn ask_judge_choice(job: &str, window: &mut Window, cx: &mut gpui::App) {
    cx.default_global::<JudgeChoiceAsked>().0 = Some(job.to_string());
    let Some(view) = cx.try_global::<super::view_tabs::LiveJobView>().and_then(|live| live.0.upgrade()) else { return };
    let shown = view.read(cx).shown.as_ref().is_some_and(|shown| shown.as_ref() == job);
    if shown {
        view.update(cx, |view, cx| view.front_judge_choice(window, cx));
    }
}

/// Open the Job's Judge tab in front, now where the Job is on screen and else when it comes to be.
pub fn ask_judge_tab(job: &str, window: &mut Window, cx: &mut gpui::App) {
    cx.default_global::<JudgeTabAsked>().0 = Some(job.to_string());
    let Some(view) = cx.try_global::<super::view_tabs::LiveJobView>().and_then(|live| live.0.upgrade()) else { return };
    let shown = view.read(cx).shown.as_ref().is_some_and(|shown| shown.as_ref() == job);
    if shown {
        view.update(cx, |view, cx| view.front_judge_tab(window, cx));
    }
}

impl JobView {
    /// The list of what the Job holds opens on the Job shown when a Jobs row asked for it, once the thread has drawn.
    pub(super) fn front_judge_choice(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(shown) = self.shown.clone() else { return };
        let asked = cx.try_global::<JudgeChoiceAsked>().and_then(|asked| asked.0.clone());
        if asked.as_deref() != Some(shown.as_ref()) {
            return;
        }
        cx.default_global::<JudgeChoiceAsked>().0 = None;
        let view = cx.weak_entity();
        window.on_next_frame(move |window, cx| {
            view.update(cx, |this, cx| {
                this.judge_choose(window, cx);
            })
            .ok();
        });
    }

    /// The Judge tab comes forward on the Job shown when a press asked for it.
    pub(super) fn front_judge_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(shown) = self.shown.clone() else { return };
        let asked = cx.try_global::<JudgeTabAsked>().and_then(|asked| asked.0.clone());
        if asked.as_deref() != Some(shown.as_ref()) {
            return;
        }
        cx.default_global::<JudgeTabAsked>().0 = None;
        self.open_judge_pane(window, cx);
    }
}

/// The command palette's Judge…, on the Job on screen: a menu of what can be judged.
pub fn choose_to_judge(window: &mut Window, cx: &mut gpui::App) {
    let Some(view) = cx.try_global::<super::view_tabs::LiveJobView>().and_then(|live| live.0.upgrade()) else { return };
    view.update(cx, |view, cx| {
        view.judge_choose(window, cx);
    });
}
