//! Settings → Workflows, the workflow picked, beside the list: its id and what it is for, its acts, then its steps,
//! each a card. A port of Classic's `Detail`, `Act`, `Said`, `Step` and `Compared`
//! (`packages/components/src/compositions/WorkflowManager/WorkflowManager.tsx`), the acts wired as
//! `apps/desktop/src/renderer/src/workflow-manager.tsx` wires them. The lanes and the box that asks an agent for a
//! change come after; `step_card` takes `StepRows` so the lanes' inspector draws the same card.

use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{div, prelude::*, svg, AnyElement, App, AsyncWindowContext, Context, Div, Entity, FocusHandle, Focusable, FontWeight, HighlightStyle, Hsla, Role, SharedString, StyledText, Subscription, Task, WeakEntity, Window};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use ipc::{ProposeToArmada, SaveWorkflow, WorkflowCopy, WorkflowId, WorkflowScope, WorkflowSummary};

use super::definition::with_words;
use super::overlay::SettingsOverlay;
use super::view::{bone, mono, ringed, tip};
use super::wire::Refused;
use super::workflows::{Listed, Place};
use crate::assets::icon_path;
use crate::job_view::OpenJobFrom;
use crate::labels::labelled;
use crate::look::{token, Look};

/// Fleet's word for a place (`WIRE_OF`).
pub fn wire_of(place: Place) -> &'static str {
    match place {
        Place::Armada => "armada",
        Place::Repository => "repository",
        Place::Yours => "kit",
    }
}

/// One step as the page draws it (`ManagedStep`).
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub id: String,
    pub label: String,
    pub glyph: Option<&'static str>,
    /// What ends it, finishing "Ends when"; empty where nothing Fleet observes does.
    pub ends: String,
    /// Each thing that ends it where any one of several does, drawn as a list in place of `ends`.
    pub ends_any: Vec<String>,
    pub passed_over: Option<String>,
    pub words: String,
}

/// The glyph of the steps the carried workflows name; any other id draws none (`glyphOf`, `STEP_GLYPHS`).
pub fn glyph_of(step: &str) -> Option<&'static str> {
    Some(match step {
        "propose" => "message-square-text",
        "discuss" => "message-circle-question-mark",
        "approve_plan" => "stamp",
        "walk" | "show" => "footprints",
        "fix" | "build" | "refine" | "wire" => "cable",
        "deliver" => "git-pull-request",
        "find_the_cause" => "microscope",
        "tests" => "test-tube-diagonal",
        "draft" => "notebook-pen",
        "sketch" => "pen-tool",
        "record_plan" => "clipboard-list",
        _ => return None,
    })
}

/// A step Fleet read, as the page draws it (`stepOf`).
pub fn step_of(one: &ipc::StepReading) -> Step {
    Step {
        id: one.id.as_str().to_string(),
        label: one.label.clone(),
        glyph: glyph_of(one.id.as_str()),
        ends: one.ends_when.clone().unwrap_or_default(),
        ends_any: one.ends_any.clone(),
        passed_over: one.passed_over_when.clone(),
        words: one.instructions.clone().unwrap_or_default(),
    }
}

/// Whether a copy's step says anything Armada's of the same id does not.
pub fn differs(one: &Step, armada: &Step) -> bool {
    one.words != armada.words || one.ends != armada.ends || one.ends_any != armada.ends_any || one.passed_over != armada.passed_over
}

/// Words cut after each sentence's stop, as the comparison marks them (`sentencesOf`).
pub fn sentences_of(words: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut now = String::new();
    let mut chars = words.chars().peekable();
    while let Some(one) = chars.next() {
        if one.is_whitespace() && now.ends_with(['.', '!', '?']) {
            while chars.peek().is_some_and(|next| next.is_whitespace()) {
                chars.next();
            }
            out.push(std::mem::take(&mut now));
        } else {
            now.push(one);
        }
    }
    out.push(now);
    out.retain(|one| !one.is_empty());
    out
}

/// An act of the header's toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    CopyInto,
    MakeCopy,
    Compare,
    Reset,
    Propose,
    Delete,
}

impl Act {
    fn id(self) -> &'static str {
        match self {
            Act::CopyInto => "copy-into",
            Act::MakeCopy => "make-copy",
            Act::Compare => "compare",
            Act::Reset => "reset",
            Act::Propose => "propose",
            Act::Delete => "delete",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Act::CopyInto | Act::MakeCopy => "copy-plus",
            Act::Compare => "git-compare",
            Act::Reset => "undo-2",
            Act::Propose => "git-pull-request-arrow",
            Act::Delete => "trash",
        }
    }

    const ALL: [Act; 6] = [Act::CopyInto, Act::MakeCopy, Act::Compare, Act::Reset, Act::Propose, Act::Delete];
}

/// The acts a workflow offers, in their groups: what runs in a repository that keeps no copy is copied into it;
/// Armada's own is copied as Yours and proposed; a copy is compared with Armada's and reset to it; a copy is deleted.
pub fn acts_of(one: &Listed, armada: Option<&Listed>) -> Vec<(&'static str, Vec<Act>)> {
    let editable = one.place != Place::Armada && one.runs_in.is_none();
    let comparable = editable && armada.is_some();
    let first = one.runs_in.is_none() && (one.place == Place::Armada || comparable);
    let mut groups = Vec::new();
    if one.runs_in.is_some() {
        groups.push(("This repository's", vec![Act::CopyInto]));
    }
    if first {
        let mut acts = Vec::new();
        if one.place == Place::Armada {
            acts.push(Act::MakeCopy);
        }
        if comparable {
            acts.extend([Act::Compare, Act::Reset]);
        }
        if one.place == Place::Armada {
            acts.push(Act::Propose);
        }
        groups.push(("Armada's", acts));
    }
    if editable {
        groups.push(("Delete", vec![Act::Delete]));
    }
    groups
}

/// Armada's own row of an id, which a copy is compared with and reset to.
pub fn armada_of(rows: &[Listed], id: &str) -> Option<Listed> {
    rows.iter().find(|one| one.place == Place::Armada && one.runs_in.is_none() && one.id == id).cloned()
}

/// What the lanes' inspector puts in a card in place of what the column draws (`StepRows`): the name and what ends
/// the step as fields, rows under the words, its own save, and whether the card stays read only there.
#[derive(Default)]
pub struct StepRows {
    pub name: Option<AnyElement>,
    pub ends: Option<AnyElement>,
    pub passed_over: Option<AnyElement>,
    pub after: Vec<AnyElement>,
    pub fixed: bool,
    pub save: Option<Rc<dyn Fn(String, &mut Window, &mut Context<SettingsOverlay>)>>,
}

/// One card to draw: the workflow's row, the step and its place, whether its words are edited, whether they save
/// as they are left (the lanes) rather than on Save, and, while comparing, Armada's step of the same id or none
/// where the copy added it.
pub struct Card<'a> {
    pub row: &'a Listed,
    pub step: &'a Step,
    pub n: usize,
    pub editable: bool,
    pub live: bool,
    pub versus: Option<Option<&'a Step>>,
    pub rows: Option<StepRows>,
}

pub enum StepsRead {
    Loading,
    Refused(String),
    Read(Vec<Step>),
}

/// What the last act said beside the acts: done, or Fleet's sentence for why not, with the Job it started.
pub struct Said {
    pub words: String,
    pub refused: bool,
    pub job: Option<String>,
}

struct WordsBox {
    state: Entity<TextareaState>,
    _changed: Subscription,
}

/// The detail's own state: each copy's steps as read, by row key; the copy compared; what was said and on which
/// row; the words typed into each step, kept until saved; and the key of each control.
#[derive(Default)]
pub struct Detail {
    reads: BTreeMap<String, StepsRead>,
    comparing: Option<String>,
    said: Option<(String, Said)>,
    busy: bool,
    words: BTreeMap<String, WordsBox>,
    keys: BTreeMap<String, FocusHandle>,
    /// The list read again after an act, and the list the connection held when it was: the connection's wins
    /// again once it has moved on.
    fresh: Option<(Option<Vec<WorkflowSummary>>, Vec<WorkflowSummary>)>,
}

impl Detail {
    pub fn in_words(&self, window: &Window, cx: &App) -> bool {
        self.words.values().any(|one| one.state.read(cx).focus_handle(cx).is_focused(window))
    }

    /// The list to draw: what an act read last, while the connection still holds what it held then.
    pub fn listed(&self, held: Option<Vec<WorkflowSummary>>) -> Option<Vec<WorkflowSummary>> {
        match &self.fresh {
            Some((then, fresh)) if *then == held => Some(fresh.clone()),
            _ => held,
        }
    }

    fn key(&self, name: &str) -> Option<&FocusHandle> {
        self.keys.get(name)
    }

    fn typed(&self, at: &str, cx: &App) -> Option<String> {
        self.words.get(at).map(|one| one.state.read(cx).value().to_string())
    }
}

/// What an act did, for the page to follow.
#[derive(Default)]
struct After {
    said: Option<String>,
    job: Option<String>,
    /// The row to pick once the list is read again.
    pick: Option<String>,
    list: bool,
    steps: bool,
    /// The row's typed words are let go, as a reset takes them back.
    drop_words: bool,
    /// The step whose words were saved, so the keys go back to its card.
    saved: Option<String>,
    /// The id a deleted copy had, so the same id elsewhere is picked.
    deleted: Option<String>,
}

/// One request made on the overlay's wire and awaited off it.
async fn ask<T: 'static>(this: &WeakEntity<SettingsOverlay>, cx: &mut AsyncWindowContext, make: impl FnOnce(&mut SettingsOverlay, &mut Context<SettingsOverlay>) -> Task<Result<T, Refused>>) -> Result<T, Refused> {
    let task = this.update(cx, make).map_err(|_| Refused::said("Settings closed"))?;
    task.await
}

fn copy_of(one: &Listed) -> WorkflowCopy {
    WorkflowCopy { workflow_id: WorkflowId::carried(&one.id), source: wire_of(one.place).into(), file: None }
}

impl SettingsOverlay {
    /// The picked copy's steps read, and Armada's beside a copy of it; the keys of the acts, the cards and their
    /// words made before they are drawn.
    pub(super) fn detail_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (_, Some(one)) = self.shown_workflows(cx) else { return };
        let rows = self.workflow_rows(cx).unwrap_or_default();
        if !self.workflows.detail.reads.contains_key(&one.key()) {
            self.read_steps(one.clone(), window, cx);
        }
        if let Some(armada) = armada_of(&rows, &one.id).filter(|_| one.place != Place::Armada) {
            if !self.workflows.detail.reads.contains_key(&armada.key()) {
                self.read_steps(armada, window, cx);
            }
        }
        let mut names: Vec<String> = Act::ALL.iter().map(|act| format!("act:{}", act.id())).collect();
        names.push("open-job".into());
        let key = one.key();
        let steps = match self.workflows.detail.reads.get(&key) {
            Some(StepsRead::Read(steps)) => steps.clone(),
            _ => Vec::new(),
        };
        for step in &steps {
            let at = format!("{key}/{}", step.id);
            names.extend([format!("card:{at}"), format!("save:{at}"), format!("discard:{at}")]);
        }
        for name in names {
            self.workflows.detail.keys.entry(name).or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
        if one.place != Place::Armada && one.runs_in.is_none() {
            for step in &steps {
                self.words_box(&key, step, window, cx);
            }
        }
    }

    /// A step's words, as typed: what was kept from before, else what Fleet read.
    fn words_box(&mut self, row: &str, step: &Step, window: &mut Window, cx: &mut Context<Self>) {
        let at = format!("{row}/{}", step.id);
        if self.workflows.detail.words.contains_key(&at) {
            return;
        }
        let kept = crate::drafts::kept(&format!("workflows:words:{at}"), cx);
        let state = cx.new(|cx| TextareaState::new(window, cx).auto_grow(2, 24));
        state.update(cx, |state, cx| state.set_value(kept.unwrap_or_else(|| step.words.clone()), window, cx));
        let held = at.clone();
        let changed = cx.subscribe_in(&state, window, move |this, _, event: &InputEvent, _, cx| this.words_changed(&held, event, cx));
        self.workflows.detail.words.insert(at, WordsBox { state, _changed: changed });
    }

    fn step_at(&self, at: &str) -> Option<&Step> {
        let (row, step) = at.rsplit_once('/')?;
        match self.workflows.detail.reads.get(row)? {
            StepsRead::Read(steps) => steps.iter().find(|one| one.id == step),
            _ => None,
        }
    }

    fn words_changed(&mut self, at: &str, event: &InputEvent, cx: &mut Context<Self>) {
        if !matches!(event, InputEvent::Change) {
            return;
        }
        let Some(text) = self.workflows.detail.typed(at, cx) else { return };
        let keep = format!("workflows:words:{at}");
        if self.step_at(at).is_some_and(|step| step.words == text) {
            crate::drafts::clear(&keep, cx);
        } else {
            crate::drafts::keep(&keep, &text, cx);
        }
        cx.notify();
    }

    /// Whether a step's words differ from what Fleet holds.
    fn words_changed_from(&self, at: &str, step: &Step, cx: &App) -> bool {
        self.workflows.detail.typed(at, cx).is_some_and(|text| text != step.words)
    }

    fn read_steps(&mut self, one: Listed, window: &mut Window, cx: &mut Context<Self>) {
        let key = one.key();
        self.workflows.detail.reads.entry(key.clone()).or_insert(StepsRead::Loading);
        let task = self.wire.workflow_steps(&one.id, wire_of(one.place), &one.manifest, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, window, cx| this.steps_read(&key, answer, window, cx));
        })
        .detach();
    }

    /// Steps read: the words nobody changed by hand follow them.
    fn steps_read(&mut self, key: &str, answer: Result<ipc::WorkflowSteps, Refused>, window: &mut Window, cx: &mut Context<Self>) {
        let read = match answer {
            Ok(read) => read.steps.iter().map(step_of).collect::<Vec<_>>(),
            Err(why) => {
                self.workflows.detail.reads.insert(key.to_string(), StepsRead::Refused(why.said));
                cx.notify();
                return;
            }
        };
        let was = match self.workflows.detail.reads.get(key) {
            Some(StepsRead::Read(steps)) => steps.clone(),
            _ => Vec::new(),
        };
        for step in &read {
            let at = format!("{key}/{}", step.id);
            let (Some(text), Some(old)) = (self.workflows.detail.typed(&at, cx), was.iter().find(|one| one.id == step.id)) else { continue };
            if text == old.words && text != step.words {
                if let Some(one) = self.workflows.detail.words.get(&at) {
                    one.state.update(cx, |state, cx| state.set_value(step.words.clone(), window, cx));
                }
            }
        }
        self.workflows.detail.reads.insert(key.to_string(), StepsRead::Read(read));
        cx.notify();
    }

    /// An act pressed on the picked workflow. Compare only draws; the rest ask Fleet, one at a time.
    fn press_act(&mut self, act: Act, window: &mut Window, cx: &mut Context<Self>) {
        let (_, Some(one)) = self.shown_workflows(cx) else { return };
        if act == Act::Compare {
            let key = one.key();
            let detail = &mut self.workflows.detail;
            detail.comparing = if detail.comparing.as_deref() == Some(key.as_str()) { None } else { Some(key) };
            cx.notify();
            return;
        }
        if self.workflows.detail.busy || !self.live {
            return;
        }
        if act == Act::Propose && self.workflows.detail.said.as_ref().is_some_and(|(row, said)| *row == one.key() && !said.refused) {
            return;
        }
        self.workflows.detail.busy = true;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let answer = run_act(act, &one, &this, cx).await;
            let _ = this.update_in(cx, |this, window, cx| this.act_done(&one, answer, window, cx));
        })
        .detach();
    }

    /// Save: the step's words written into the definition the copy is read from.
    fn save_words(&mut self, at: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (_, Some(one)) = self.shown_workflows(cx) else { return };
        let Some(words) = self.workflows.detail.typed(at, cx) else { return };
        let Some((_, step)) = at.rsplit_once('/') else { return };
        if self.workflows.detail.busy || !self.live {
            return;
        }
        self.workflows.detail.busy = true;
        cx.notify();
        let step = step.to_string();
        cx.spawn_in(window, async move |this, cx| {
            let answer = save_words(&one, &step, &words, &this, cx).await;
            let _ = this.update_in(cx, |this, window, cx| this.act_done(&one, answer, window, cx));
        })
        .detach();
    }

    /// Discard: the words typed let go, back to what Fleet holds.
    fn discard_words(&mut self, at: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(words) = self.step_at(at).map(|step| step.words.clone()) else { return };
        if let Some(one) = self.workflows.detail.words.get(at) {
            one.state.update(cx, |state, cx| state.set_value(words, window, cx));
        }
        crate::drafts::clear(&format!("workflows:words:{at}"), cx);
        if let Some(card) = self.workflows.detail.key(&format!("card:{at}")).cloned() {
            window.focus(&card, cx);
        }
        cx.notify();
    }

    fn act_done(&mut self, one: &Listed, answer: Result<After, Refused>, window: &mut Window, cx: &mut Context<Self>) {
        let key = one.key();
        self.workflows.detail.busy = false;
        let after = match answer {
            Ok(after) => after,
            Err(why) => {
                self.workflows.detail.said = Some((key, Said { words: why.said, refused: true, job: None }));
                cx.notify();
                return;
            }
        };
        match after.said {
            Some(words) => self.workflows.detail.said = Some((key.clone(), Said { words, refused: false, job: after.job })),
            None => {
                if self.workflows.detail.said.as_ref().is_some_and(|(row, said)| *row == key && said.refused) {
                    self.workflows.detail.said = None;
                }
            }
        }
        if after.drop_words {
            let mine = format!("{key}/");
            let dropped: Vec<String> = self.workflows.detail.words.keys().filter(|at| at.starts_with(&mine)).cloned().collect();
            for at in dropped {
                self.workflows.detail.words.remove(&at);
                crate::drafts::clear(&format!("workflows:words:{at}"), cx);
            }
        }
        if let Some(step) = &after.saved {
            let at = format!("{key}/{step}");
            crate::drafts::clear(&format!("workflows:words:{at}"), cx);
            if let Some(card) = self.workflows.detail.key(&format!("card:{at}")).cloned() {
                window.focus(&card, cx);
            }
        }
        if after.steps {
            self.read_steps(one.clone(), window, cx);
        }
        if after.list {
            self.read_list(after.pick, after.deleted.map(|id| (id, key)), window, cx);
        }
        cx.notify();
    }

    /// `GET /workflows` again after an act, then the row it named picked: the copy made, or after a delete the
    /// same id where it is still listed, else the first.
    fn read_list(&mut self, pick: Option<String>, deleted: Option<(String, String)>, window: &mut Window, cx: &mut Context<Self>) {
        let task = self.wire.list_workflows(cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if let Ok(list) = answer {
                    let held = this.wire.workflows(cx);
                    this.workflows.detail.fresh = Some((held, list));
                }
                let rows = this.workflow_rows(cx).unwrap_or_default();
                let next = match (pick, deleted) {
                    (Some(pick), _) => Some(pick),
                    (None, Some((id, gone))) => rows.iter().find(|row| row.id == id && row.key() != gone).or(rows.first()).map(Listed::key),
                    _ => None,
                };
                if let Some(next) = next.filter(|next| rows.iter().any(|row| row.key() == *next)) {
                    this.pick_workflow(&next, window, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn open_job(&mut self, job: String, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(open) = cx.try_global::<OpenJobFrom>().cloned() {
            cx.defer(move |cx| (open.0)(job, None, cx));
        }
        self.close(window, cx);
    }

    /// The detail's stops, after the list's: each act, Open Job where an act started one, then the first card or
    /// the one the keys are on, each card's Discard and Save while its words are changed, and its words.
    pub(super) fn detail_stops_for(&self, window: &Window, cx: &App) -> Vec<FocusHandle> {
        let (_, Some(one)) = self.shown_workflows(cx) else { return Vec::new() };
        let detail = &self.workflows.detail;
        let key = one.key();
        let rows = self.workflow_rows(cx).unwrap_or_default();
        let mut stops = Vec::new();
        let said = detail.said.as_ref().filter(|(row, _)| *row == key).map(|(_, said)| said);
        for (_, acts) in acts_of(&one, armada_of(&rows, &one.id).as_ref()) {
            for act in acts {
                if act == Act::Propose && said.is_some_and(|said| !said.refused) {
                    continue;
                }
                stops.extend(detail.key(&format!("act:{}", act.id())).cloned());
            }
        }
        if said.is_some_and(|said| said.job.is_some()) {
            stops.extend(detail.key("open-job").cloned());
        }
        let Some(StepsRead::Read(steps)) = detail.reads.get(&key) else { return stops };
        let on = steps.iter().position(|step| detail.key(&format!("card:{key}/{}", step.id)).is_some_and(|card| card.is_focused(window)));
        for (n, step) in steps.iter().enumerate() {
            let at = format!("{key}/{}", step.id);
            if n == 0 || on == Some(n) {
                stops.extend(detail.key(&format!("card:{at}")).cloned());
            }
            if self.words_changed_from(&at, step, cx) {
                stops.extend(detail.key(&format!("discard:{at}")).cloned());
                stops.extend(detail.key(&format!("save:{at}")).cloned());
            }
            if let Some(words) = detail.words.get(&at) {
                stops.push(words.state.read(cx).focus_handle(cx));
            }
        }
        stops
    }

    /// The detail's keys: ↑ and ↓ between cards, E into a card's words and Esc back out, Enter and Space on an
    /// act, Save, Discard and Open Job.
    pub(super) fn detail_key(&mut self, word: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let (_, Some(one)) = self.shown_workflows(cx) else { return false };
        let key = one.key();
        let steps = match self.workflows.detail.reads.get(&key) {
            Some(StepsRead::Read(steps)) => steps.clone(),
            _ => Vec::new(),
        };
        let press = matches!(word, "enter" | "space");
        for (n, step) in steps.iter().enumerate() {
            let at = format!("{key}/{}", step.id);
            let card = self.workflows.detail.key(&format!("card:{at}")).cloned();
            if self.workflows.detail.words.get(&at).is_some_and(|one| one.state.read(cx).focus_handle(cx).is_focused(window)) {
                if word == "escape" {
                    if let Some(card) = card {
                        window.focus(&card, cx);
                        cx.notify();
                    }
                    return true;
                }
                return false;
            }
            if card.as_ref().is_some_and(|card| card.is_focused(window)) {
                match word {
                    "up" | "down" => {
                        let next = if word == "up" { n.saturating_sub(1) } else { (n + 1).min(steps.len() - 1) };
                        if let Some(next) = self.workflows.detail.key(&format!("card:{key}/{}", steps[next].id)).cloned() {
                            window.focus(&next, cx);
                            cx.notify();
                        }
                        return true;
                    }
                    "e" => {
                        if let Some(words) = self.workflows.detail.words.get(&at) {
                            let focus = words.state.read(cx).focus_handle(cx);
                            window.focus(&focus, cx);
                            cx.notify();
                        }
                        return true;
                    }
                    _ => return false,
                }
            }
            if press && self.workflows.detail.key(&format!("save:{at}")).is_some_and(|save| save.is_focused(window)) {
                self.save_words(&at, window, cx);
                return true;
            }
            if press && self.workflows.detail.key(&format!("discard:{at}")).is_some_and(|discard| discard.is_focused(window)) {
                self.discard_words(&at, window, cx);
                return true;
            }
        }
        if !press {
            return false;
        }
        if let Some(act) = Act::ALL.into_iter().find(|act| self.workflows.detail.key(&format!("act:{}", act.id())).is_some_and(|focus| focus.is_focused(window))) {
            self.press_act(act, window, cx);
            return true;
        }
        if self.workflows.detail.key("open-job").is_some_and(|focus| focus.is_focused(window)) {
            if let Some(job) = self.workflows.detail.said.as_ref().and_then(|(_, said)| said.job.clone()) {
                self.open_job(job, window, cx);
            }
            return true;
        }
        false
    }

    // ---- drawn --------------------------------------------------------------------------------

    /// The picked workflow: its head with the acts, then its steps.
    pub(super) fn render_workflow_detail(&self, one: &Listed, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = one.key();
        let rows = self.workflow_rows(cx).unwrap_or_default();
        let armada = armada_of(&rows, &one.id);
        let editable = one.place != Place::Armada && one.runs_in.is_none();
        let comparing = editable && armada.is_some() && self.workflows.detail.comparing.as_deref() == Some(key.as_str());
        let armada_steps = match armada.as_ref().and_then(|armada| self.workflows.detail.reads.get(&armada.key())) {
            Some(StepsRead::Read(steps)) => Some(steps.clone()),
            _ => None,
        };
        let steps: AnyElement = match self.workflows.detail.reads.get(&key) {
            None | Some(StepsRead::Loading) => self.render_steps_pending(look),
            Some(StepsRead::Refused(said)) => labelled(div().id("workflow-steps-refused").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(SharedString::from(said.clone())), Role::Alert, said.clone()).into_any_element(),
            Some(StepsRead::Read(steps)) => {
                let mut column = div().flex().flex_col().gap(look.length(token::SPACE_4));
                for (n, step) in steps.iter().enumerate() {
                    let versus = (comparing && armada_steps.is_some()).then(|| armada_steps.as_ref().and_then(|all| all.iter().find(|one| one.id == step.id)));
                    column = column.child(self.step_card(Card { row: one, step, n: n + 1, editable, live: false, versus, rows: None }, look, window, cx));
                }
                column.into_any_element()
            }
        };
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::GAP_SECTION))
            .pb(look.length(token::SPACE_4))
            .child(self.render_workflow_head(one, armada.as_ref(), comparing, look, window, cx))
            .child(div().px(look.length(token::SPACE_4)).child(steps))
            .into_any_element()
    }

    /// The head: the id and the line it is for, the acts, and what the last act said.
    fn render_workflow_head(&self, one: &Listed, armada: Option<&Listed>, comparing: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = one.key();
        let said = self.workflows.detail.said.as_ref().filter(|(row, _)| *row == key).map(|(_, said)| said);
        let off = self.workflows.detail.busy || !self.live;
        let repository = one.runs_in.as_ref().map(|manifest| self.wire.repositories(cx).unwrap_or_default().into_iter().find(|(id, _, _)| id == manifest).map_or_else(|| manifest.clone(), |(_, name, _)| name));
        let title = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .min_w_0()
            .child(labelled(div().id("workflow-title").text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(one.id.clone())), Role::Heading, one.id.clone()))
            .children(one.for_requests.clone().filter(|line| !line.is_empty()).map(|line| {
                div()
                    .flex()
                    .flex_wrap()
                    .items_baseline()
                    .gap_x(look.length(token::SPACE_3))
                    .child(div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_MUTED)).child("For requests"))
                    .child(div().min_w_0().max_w_full().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(line)))
            }));
        let mut toolbar = div().id("workflow-acts").flex().flex_wrap().items_center().gap_y(look.length(token::SPACE_2)).gap_x(look.length(token::SPACE_6));
        for (group, acts) in acts_of(one, armada) {
            let mut row = div().flex().items_center().gap(look.length(token::SPACE_1));
            for act in acts {
                let label = match act {
                    Act::CopyInto => format!("Copy into {}", repository.clone().unwrap_or_default()),
                    Act::MakeCopy => "Make my copy".into(),
                    Act::Compare => "Compare with Armada's".into(),
                    Act::Reset => "Reset to Armada's".into(),
                    Act::Propose => "Propose to Armada".into(),
                    Act::Delete => "Delete".into(),
                };
                let held_off = off || (act == Act::Propose && said.is_some_and(|said| !said.refused));
                row = row.child(self.act_button(act, label, act == Act::Compare && comparing, if act == Act::Compare { false } else { held_off }, look, window, cx));
            }
            let row = labelled(row.id(SharedString::from(format!("workflow-acts-{group}"))), Role::Group, group);
            toolbar = toolbar.child(if group == "Delete" { div().ml(look.length(token::SPACE_4)).child(row).into_any_element() } else { row.into_any_element() });
        }
        let said_line = said.map(|said| {
            let ink = if said.refused { look.color(token::STATUS_COMPLETED_FAILED) } else { look.color(token::FG_MUTED) };
            let line = div()
                .id("workflow-said")
                .w_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_XS))
                .text_color(ink)
                .when(!said.refused, |line| line.child(svg().flex_none().path(icon_path("check")).size(look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH) * 2.).text_color(look.color(token::STATUS_COMPLETED_SUCCESS))))
                .child(SharedString::from(said.words.clone()))
                .children(said.job.clone().and_then(|job| {
                    let focus = self.workflows.detail.key("open-job")?.clone();
                    let button = ringed(look, super::view::ghost(look, "workflow-open-job", "Open Job", false), &focus, window)
                        .track_focus(&focus)
                        .on_click(cx.listener(move |this, _, window, cx| this.open_job(job.clone(), window, cx)));
                    Some(labelled(button, Role::Button, "Open Job"))
                }));
            labelled(line, if said.refused { Role::Alert } else { Role::Status }, said.words.clone())
        });
        labelled(
            div()
                .id("workflow-head")
                .flex()
                .flex_wrap()
                .items_start()
                .justify_between()
                .gap_y(look.length(token::SPACE_2))
                .gap_x(look.length(token::SPACE_4))
                .p(look.length(token::SPACE_4))
                .child(title)
                .child(labelled(toolbar, Role::Toolbar, "Acts"))
                .children(said_line),
            Role::Article,
            one.id.clone(),
        )
        .into_any_element()
    }

    /// One act: its glyph, its name in the tooltip and for a reader who cannot see it (`Act`).
    #[allow(clippy::too_many_arguments)]
    fn act_button(&self, act: Act, label: String, pressed: bool, off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.workflows.detail.key(&format!("act:{}", act.id())).cloned() else { return div().into_any_element() };
        let failed = look.color(token::STATUS_COMPLETED_FAILED);
        let (ground, edge, ink, hover): (Hsla, Hsla, Hsla, Hsla) = match act {
            _ if pressed => (look.color(token::ACCENT_MUTED), look.color(token::ACCENT), look.color(token::FG_DEFAULT), look.color(token::ACCENT_MUTED)),
            Act::Compare | Act::Reset => (look.color(token::BG_SUNKEN), look.color(token::BORDER_STRONG), look.color(token::FG_DEFAULT), look.color(token::BG_HOVER)),
            Act::Delete => (gpui::transparent_black(), failed, failed, look.color(token::STATUS_COMPLETED_FAILED_BG)),
            Act::CopyInto | Act::MakeCopy | Act::Propose => (gpui::transparent_black(), gpui::transparent_black(), look.color(token::FG_MUTED), look.color(token::BG_HOVER)),
        };
        let button = div()
            .id(SharedString::from(format!("workflow-act-{}", act.id())))
            .track_focus(&focus)
            .flex_none()
            .size(look.length(token::H_CONTROL_SM))
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(ground)
            .tooltip(tip(label.clone()))
            .child(svg().path(icon_path(act.glyph())).size(look.length(token::SPACE_4)).text_color(ink))
            .when(off, |button| button.opacity(0.5))
            .when(!off, |button| button.cursor_pointer().hover(move |style| style.bg(hover)).on_click(cx.listener(move |this, _, window, cx| this.press_act(act, window, cx))));
        labelled(ringed(look, button, &focus, window), Role::Button, label).into_any_element()
    }

    /// Three cards drawn as shapes while Fleet reads the steps.
    fn render_steps_pending(&self, look: &Look) -> AnyElement {
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_4));
        for (n, wide) in [96., 72., 112.].into_iter().enumerate() {
            let tall = look.length(token::LEADING_XS);
            column = column.child(
                card_frame(look, false)
                    .id(SharedString::from(format!("workflow-step-bone-{n}")))
                    .child(band(look).child(bone(look, gpui::px(wide), tall)))
                    .child(div().flex().flex_col().gap(look.length(token::SPACE_2)).p(look.length(token::SPACE_4)).child(bone(look, gpui::px(160.), tall)).child(bone(look, gpui::px(320.), tall))),
            );
        }
        labelled(column.id("workflow-steps-pending"), Role::Image, "Loading").into_any_element()
    }

    /// One step as its card (`Step`): a band with its place, name and glyph, the marks a comparison makes and Save
    /// while its words are changed; then what ends it, when it is passed over, and its words.
    pub fn step_card(&self, card: Card, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Card { row, step, n, editable, live, versus, rows } = card;
        let at = format!("{}/{}", row.key(), step.id);
        let detail = &self.workflows.detail;
        let Some(focus) = detail.key(&format!("card:{at}")).cloned() else { return div().into_any_element() };
        let mut rows = rows;
        let editable = editable && !rows.as_ref().is_some_and(|rows| rows.fixed);
        let changed = editable && self.words_changed_from(&at, step, cx);
        let armada = versus.flatten();
        let added = matches!(versus, Some(None));
        let differs = armada.is_some_and(|armada| differs(step, armada));
        let off = detail.busy || !self.live;
        let in_rows = rows.is_some();

        let band_drawn = !in_rows || added || differs || (changed && !live);
        let band_row = band_drawn.then(|| {
            let mark = if added { Some(("list-plus", "Not in Armada's")) } else if differs { Some(("git-compare", "Differs from Armada's")) } else { None };
            let marks = div().flex_none().w(look.length(token::SPACE_6)).flex().items_center().justify_center().children(mark.map(|(glyph, said)| {
                labelled(div().id(SharedString::from(format!("workflow-mark-{at}"))).tooltip(tip(said)).child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH) * 2.).text_color(look.color(token::ACCENT))), Role::Image, said)
            }));
            let mut save = div().flex_none().flex().items_center().justify_end().gap(look.length(token::SPACE_1));
            if changed && !live {
                if let (Some(discard_key), Some(save_key)) = (detail.key(&format!("discard:{at}")).cloned(), detail.key(&format!("save:{at}")).cloned()) {
                    let press = at.clone();
                    let discard = ringed(look, super::view::ghost(look, SharedString::from(format!("workflow-discard-{at}")), "Discard", false), &discard_key, window)
                        .track_focus(&discard_key)
                        .on_click(cx.listener(move |this, _, window, cx| this.discard_words(&press, window, cx)));
                    let press = at.clone();
                    let own = rows.as_mut().and_then(|rows| rows.save.take());
                    let saving = ringed(look, sunken_button(look, SharedString::from(format!("workflow-save-{at}")), "Save", off), &save_key, window).track_focus(&save_key).when(!off, |button| {
                        button.on_click(cx.listener(move |this, _, window, cx| match &own {
                            Some(own) => {
                                let words = this.workflows.detail.typed(&press, cx).unwrap_or_default();
                                own(words, window, cx);
                            }
                            None => this.save_words(&press, window, cx),
                        }))
                    });
                    save = save.child(labelled(discard, Role::Button, format!("Discard {}", step.label))).child(labelled(saving, Role::Button, format!("Save {}", step.label)));
                }
            }
            band(look)
                .when(!in_rows, |band| {
                    band.child(div().text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(n.to_string())))
                        .child(div().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(step.label.clone())))
                        .children(step.glyph.map(|glyph| svg().flex_none().path(icon_path(glyph)).size(look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH) * 2.).text_color(look.color(token::FG_MUTED))))
                })
                // Save and the marks keep to the band's right, together, the marks last so they line up from card to card.
                .child(div().flex_none().ml_auto().flex().items_center().gap(look.length(token::SPACE_3)).child(save).child(marks))
        });

        let mut facts = div().flex().flex_col().gap(look.length(token::SPACE_2)).p(look.length(token::SPACE_4)).text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT));
        let (name, ends_field, passed_field, after) = match rows {
            Some(rows) => (rows.name, rows.ends, rows.passed_over, rows.after),
            None => (None, None, None, Vec::new()),
        };
        if let Some(name) = name {
            facts = facts.child(fact(look, "Name", name));
        }
        if step.ends_any.len() > 1 {
            let list = div().flex().flex_col().gap(look.length(token::SPACE_1)).children(step.ends_any.iter().map(|one| SharedString::from(one.clone())));
            facts = facts.child(fact(look, "Ends when", labelled(list.id(SharedString::from(format!("workflow-ends-{at}"))), Role::List, "Ends when").into_any_element()));
        } else if let Some(ends) = ends_field {
            facts = facts.child(fact(look, "Ends when", ends));
        } else if !step.ends.is_empty() {
            facts = facts.child(fact(look, "Ends when", div().child(SharedString::from(step.ends.clone())).into_any_element()));
        }
        if let Some(passed) = passed_field {
            facts = facts.child(fact(look, "Passed over when", passed));
        } else if let Some(passed) = &step.passed_over {
            facts = facts.child(fact(look, "Passed over when", div().child(SharedString::from(passed.clone())).into_any_element()));
        }
        let words: AnyElement = match (armada.filter(|_| differs && !changed), detail.words.get(&at).filter(|_| editable)) {
            (Some(armada), _) => compared(look, &at, &armada.words, &step.words),
            (None, Some(words)) => labelled(
                div().id(SharedString::from(format!("workflow-words-{at}"))).key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).child(Textarea::new(&words.state).disabled(off)),
                Role::TextInput,
                format!("Words of {}", step.label),
            )
            .into_any_element(),
            (None, None) => div().min_w_0().child(SharedString::from(step.words.clone())).into_any_element(),
        };
        facts = facts.child(fact(look, "Words", words)).children(after);

        let frame = card_frame(look, changed).id(SharedString::from(format!("workflow-step-{at}"))).track_focus(&focus).children(band_row).child(facts);
        labelled(ringed(look, frame, &focus, window), Role::Group, step.label.clone()).into_any_element()
    }
}

/// The ask behind each act (`onCopyInto`, `onMakeCopy`, `onReset`, `onPropose`, `onDelete`).
async fn run_act(act: Act, one: &Listed, this: &WeakEntity<SettingsOverlay>, cx: &mut AsyncWindowContext) -> Result<After, Refused> {
    let (id, manifest) = (one.id.clone(), one.manifest.clone());
    match act {
        Act::MakeCopy | Act::CopyInto => {
            let (from, into, scope) = match act {
                Act::MakeCopy => ("armada", manifest.clone(), WorkflowScope::Kit),
                _ => (wire_of(one.place), one.runs_in.clone().unwrap_or_else(|| manifest.clone()), WorkflowScope::Repository),
            };
            let held = ask(this, cx, |this, cx| this.wire.workflow_definition(&id, from, &into, cx)).await?;
            let save = SaveWorkflow { scope, definition: held.definition, overwrite: false };
            // A copy already there is the one to open.
            match ask(this, cx, |this, cx| this.wire.save_workflow(&into, save, cx)).await {
                Err(why) if !why.is("fleet.workflow_exists") => return Err(why),
                _ => {}
            }
            let pick = if scope == WorkflowScope::Kit { format!("yours:{id}") } else { format!("repository@{into}:{id}") };
            Ok(After { pick: Some(pick), list: true, ..After::default() })
        }
        Act::Reset => {
            let copy = copy_of(one);
            ask(this, cx, |this, cx| this.wire.reset_workflow(&manifest, copy, cx)).await?;
            Ok(After { said: Some("Matches Armada's".into()), list: true, steps: true, drop_words: true, ..After::default() })
        }
        Act::Propose => {
            let held = ask(this, cx, |this, cx| this.wire.workflow_definition(&id, "armada", &manifest, cx)).await?;
            let proposing = ProposeToArmada { workflow_id: WorkflowId::carried(&id), definition: held.definition };
            let proposed = ask(this, cx, |this, cx| this.wire.propose_workflow(proposing, cx)).await?;
            let said = proposed.send_failed.map_or_else(|| "Sent. A Job started on the Armada repository".to_string(), |why| format!("Job started on the Armada repository. Brief not sent: {why}"));
            Ok(After { said: Some(said), job: Some(proposed.session_id.as_str().to_string()), ..After::default() })
        }
        Act::Delete => {
            let copy = copy_of(one);
            ask(this, cx, |this, cx| this.wire.delete_workflow(&manifest, copy, cx)).await?;
            Ok(After { list: true, drop_words: true, deleted: Some(id), ..After::default() })
        }
        Act::Compare => Ok(After::default()),
    }
}

/// A step's words written into the definition the copy is read from (`onSaveWords`).
async fn save_words(one: &Listed, step: &str, words: &str, this: &WeakEntity<SettingsOverlay>, cx: &mut AsyncWindowContext) -> Result<After, Refused> {
    let (id, manifest, source) = (one.id.clone(), one.manifest.clone(), wire_of(one.place));
    let held = ask(this, cx, |this, cx| this.wire.workflow_definition(&id, source, &manifest, cx)).await?;
    let definition = with_words(&held.definition, step, words).ok_or_else(|| Refused::said(format!("{step} is not a step of {}", held.file)))?;
    let scope = if one.place == Place::Repository { WorkflowScope::Repository } else { WorkflowScope::Kit };
    ask(this, cx, |this, cx| this.wire.save_workflow(&manifest, SaveWorkflow { scope, definition, overwrite: true }, cx)).await?;
    Ok(After { steps: true, list: true, saved: Some(step.to_string()), ..After::default() })
}

/// A card's frame: the sunken ground and its edge, the accent while its words are changed.
fn card_frame(look: &Look, changed: bool) -> Div {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if changed { look.color(token::ACCENT) } else { look.color(token::BORDER_DEFAULT) })
        .bg(look.color(token::BG_SUNKEN))
        .overflow_hidden()
}

/// A card's band, over its facts.
fn band(look: &Look) -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_x(look.length(token::SPACE_3))
        .gap_y(look.length(token::SPACE_1))
        .min_h(look.length(token::H_ROW) * 1.25)
        .pt(look.length(token::SPACE_1))
        .pb(look.length(token::SPACE_1))
        .pl(look.length(token::SPACE_4))
        .pr(look.length(token::SPACE_3))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_RAISED))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_MUTED))
}

/// A secondary button on a sunken ground, as the band's Save is (`variant="secondary" ground="sunken"`).
fn sunken_button(look: &Look, id: SharedString, said: &'static str, off: bool) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_STRONG))
        .bg(look.color(token::BG_RAISED))
        .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
        .text_size(look.length(token::TEXT_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(look.color(token::FG_DEFAULT))
        .when(off, |button| button.opacity(0.5))
        .when(!off, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
        .child(said)
}

/// One fact of a card: its name in the narrow column, and what it says beside it, or under it where the card is
/// too narrow to hold both.
fn fact(look: &Look, label: &'static str, value: AnyElement) -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_start()
        .gap_x(look.length(token::SPACE_3))
        .gap_y(look.length(token::SPACE_1))
        .child(
            div()
                .flex_none()
                .w(look.length(token::TEXT_SM) * 9.)
                // On the line of the value's first words, as a baseline grid sets it.
                .line_height(look.length(token::LEADING_SM))
                .whitespace_nowrap()
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_SUBTLE))
                .child(label),
        )
        .child(div().flex_1().min_w(look.length(token::SPACE_12) * 3.).child(value))
}

/// A step's words beside Armada's, each sentence the other side lacks marked (`Compared`).
fn compared(look: &Look, at: &str, against: &str, words: &str) -> AnyElement {
    let theirs = sentences_of(against);
    let ours = sentences_of(words);
    let side = |label: &'static str, said: &[String], other: &[String], mark: Hsla| {
        let mut text = String::new();
        let mut marks = Vec::new();
        for one in said {
            let from = text.len();
            text.push_str(one);
            if !other.contains(one) {
                marks.push((from..text.len(), HighlightStyle { background_color: Some(mark), ..HighlightStyle::default() }));
            }
            text.push(' ');
        }
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .py(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(div().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child(label))
            .child(div().min_w_0().child(StyledText::new(SharedString::from(text.trim_end().to_string())).with_highlights(marks)))
    };
    labelled(
        div()
            .id(SharedString::from(format!("workflow-compared-{at}")))
            .flex()
            .gap(look.length(token::SPACE_3))
            .child(side("Armada's", &theirs, &ours, look.color(token::STATUS_COMPLETED_FAILED_BG)))
            .child(side("This copy", &ours, &theirs, look.color(token::STATUS_COMPLETED_SUCCESS_BG))),
        Role::Group,
        "Compared with Armada's",
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(place: Place, runs_in: Option<&str>) -> Listed {
        Listed { id: "bug".into(), place, manifest: "armada".into(), runs_in: runs_in.map(String::from), for_requests: None }
    }

    #[test]
    fn each_place_offers_its_own_acts() {
        let armada = row(Place::Armada, None);
        let said = |one: &Listed, armada: Option<&Listed>| acts_of(one, armada).into_iter().map(|(group, acts)| (group, acts)).collect::<Vec<_>>();
        assert_eq!(said(&armada, Some(&armada)), [("Armada's", vec![Act::MakeCopy, Act::Propose])]);
        assert_eq!(said(&row(Place::Repository, None), Some(&armada)), [("Armada's", vec![Act::Compare, Act::Reset]), ("Delete", vec![Act::Delete])]);
        assert_eq!(said(&row(Place::Yours, None), None), [("Delete", vec![Act::Delete])]);
        assert_eq!(said(&row(Place::Armada, Some("sample-web")), Some(&armada)), [("This repository's", vec![Act::CopyInto])]);
    }

    #[test]
    fn sentences_part_after_each_stop() {
        assert_eq!(sentences_of("Wire it.  Write the tests now! Then?  done"), ["Wire it.", "Write the tests now!", "Then?", "done"]);
        assert_eq!(sentences_of(""), Vec::<String>::new());
    }

    #[test]
    fn a_step_differs_on_its_words_or_what_ends_it() {
        let step = Step { id: "fix".into(), label: "Fix".into(), glyph: glyph_of("fix"), ends: "it is pushed".into(), ends_any: Vec::new(), passed_over: None, words: "Fix it.".into() };
        assert!(!differs(&step, &step.clone()));
        assert!(differs(&Step { words: "Fix it now.".into(), ..step.clone() }, &step));
        assert!(differs(&Step { passed_over: Some("nothing would be seen".into()), ..step.clone() }, &step));
        assert_eq!(step.glyph, Some("cable"));
    }
}
