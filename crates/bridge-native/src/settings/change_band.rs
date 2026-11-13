//! Settings → Workflows, under a copy's steps: the box that tells an agent what to change, and the change it comes
//! back with for him to approve, send back or dismiss. A port of Classic's `Agent`, `StepChangeCard` and
//! `FieldValue` (`packages/components/src/compositions/WorkflowManager/WorkflowManager.tsx`), `stepChangesOf`
//! (`diff.ts`) and `agentOf` (`apps/desktop/src/renderer/src/workflow-manager-wire.ts`), the acts wired as
//! `workflow-manager.tsx` wires them. Fleet's routes are `crates/ipc/operations/*_workflow_change*.toml`.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use gpui::{canvas, div, point, prelude::*, svg, AnyElement, App, Bounds, Context, Entity, FocusHandle, Focusable, FontWeight, HighlightStyle, Hsla, Keystroke, Pixels, Role, ScrollHandle, SharedString, StyledText, Subscription, Task, Window};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use ipc::{AskWorkflowChange, ChangeOrigin, ChangeState, ChangeStep, SendBackWorkflowChange, WorkflowChange, WorkflowId};

use super::definition::{tree_of, Json};
use super::overlay::SettingsOverlay;
use super::view::{mono, ringed, tip};
use super::wire::Refused;
use super::workflow_detail::{ask, band, glyph_of, make_copy, sentences_of, sunken_button, wire_of, Step, StepsRead};
use super::workflows::{Listed, Place};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// How often the band reads Fleet again while a change is drafting (`POLL_MS`).
const POLL: std::time::Duration = std::time::Duration::from_millis(1000);

// ---- the diff ----------------------------------------------------------------------------------

/// The four facts a step carries, in the order a card's rows draw them (`STEP_FIELDS`). Several ends are read as
/// all of them, one a line, so a change to the list is a change.
const FIELDS: [&str; 4] = ["Name", "Ends when", "Passed over when", "Words"];

fn field_of(step: &Step, field: &str) -> String {
    match field {
        "Name" => step.label.clone(),
        "Ends when" if step.ends_any.len() > 1 => step.ends_any.join("\n"),
        "Ends when" => step.ends.clone(),
        "Passed over when" => step.passed_over.clone().unwrap_or_default(),
        _ => step.words.clone(),
    }
}

fn same(one: &Step, other: &Step) -> bool {
    FIELDS.iter().all(|field| field_of(one, field) == field_of(other, field))
}

/// One step a change touches. `n` is its place after the change, or before it where removed; `was` a changed
/// step's place before (`StepChange`).
#[derive(Clone, Debug, PartialEq)]
pub enum StepChange {
    Added { n: usize, after: Step },
    Removed { n: usize, before: Step },
    Changed { n: usize, was: usize, before: Step, after: Step },
}

/// The ids of steps on both sides that moved: those outside the longest run that keeps its order. At a tie the run
/// keeping the steps that stood earlier stays put, so the later step is the one said to move (`movedIds`).
fn moved_ids(before: &[Step], after: &[Step]) -> BTreeSet<String> {
    let common: Vec<&str> = before.iter().filter(|one| after.iter().any(|each| each.id == one.id)).map(|one| one.id.as_str()).collect();
    let stood = |id: &str| common.iter().position(|one| *one == id);
    let seq: Vec<usize> = after.iter().filter_map(|one| stood(&one.id)).collect();
    let mut run = vec![0usize; seq.len()];
    let mut prev: Vec<Option<usize>> = vec![None; seq.len()];
    let stronger = |run: &[usize], one: usize, over: Option<usize>| match over {
        None => true,
        Some(over) => run[one] > run[over] || (run[one] == run[over] && seq[one] < seq[over]),
    };
    for at in 0..seq.len() {
        let mut from = None;
        for each in 0..at {
            if seq[each] < seq[at] && stronger(&run, each, from) {
                from = Some(each);
            }
        }
        run[at] = from.map_or(1, |from| run[from] + 1);
        prev[at] = from;
    }
    let mut end = None;
    for at in 0..seq.len() {
        if stronger(&run, at, end) {
            end = Some(at);
        }
    }
    let mut kept = BTreeSet::new();
    while let Some(at) = end {
        kept.insert(seq[at]);
        end = prev[at];
    }
    common.iter().enumerate().filter(|(at, _)| !kept.contains(at)).map(|(_, id)| id.to_string()).collect()
}

/// The steps a change touches, in the order the workflow reads after it, a removed step where it stood; and the
/// labels of the rest (`stepChangesOf`).
pub fn step_changes(before: &[Step], after: &[Step]) -> (Vec<StepChange>, Vec<String>) {
    let moved = moved_ids(before, after);
    // A removed step sits after the last step before it that stays, or first where none does.
    let mut removed_after: BTreeMap<Option<String>, Vec<StepChange>> = BTreeMap::new();
    let mut anchor: Option<String> = None;
    for (at, one) in before.iter().enumerate() {
        if after.iter().any(|each| each.id == one.id) {
            anchor = Some(one.id.clone());
        } else {
            removed_after.entry(anchor.clone()).or_default().push(StepChange::Removed { n: at + 1, before: one.clone() });
        }
    }
    let mut changes = removed_after.remove(&None).unwrap_or_default();
    let mut unchanged = Vec::new();
    for (at, one) in after.iter().enumerate() {
        match before.iter().position(|each| each.id == one.id) {
            None => changes.push(StepChange::Added { n: at + 1, after: one.clone() }),
            Some(was) if moved.contains(&one.id) || !same(&before[was], one) => changes.push(StepChange::Changed { n: at + 1, was: was + 1, before: before[was].clone(), after: one.clone() }),
            Some(_) => unchanged.push(one.label.clone()),
        }
        changes.extend(removed_after.remove(&Some(one.id.clone())).unwrap_or_default());
    }
    (changes, unchanged)
}

// ---- the phase ---------------------------------------------------------------------------------

/// What the band draws, from the open changes (`AgentState`).
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Idle,
    Working { asked: Option<String> },
    Failed { reason: String, asked: Option<String> },
    Cannot { asked: Option<String>, missing: Vec<String> },
    /// `fresh`: a workflow nothing held before, approved or dismissed but never sent back.
    Proposed { before: Vec<Step>, after: Vec<Step>, asked: Option<String>, fresh: bool },
}

fn step_of_change(one: &ChangeStep) -> Step {
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

/// A definition's text as steps, where Fleet sent no reading of it: a step the workflow already has keeps what was
/// read of it, one it lacks has no end to say (`stepsOfText`).
fn steps_of_text(text: &str, known: &[Step]) -> Vec<Step> {
    let Some(Json::List(steps)) = tree_of(text).and_then(|tree| tree.get("steps").cloned()) else { return Vec::new() };
    steps
        .iter()
        .map(|one| {
            let id = one.get("id").and_then(Json::text).unwrap_or_default().to_string();
            let had = known.iter().find(|each| each.id == id).cloned();
            let label = one.get("label").and_then(Json::text).unwrap_or(&id).to_string();
            let words = one.get("instructions").and_then(Json::text).unwrap_or_default().to_string();
            match had {
                Some(had) => Step { label, words, ..had },
                None => Step { glyph: glyph_of(&id), id, label, ends: String::new(), ends_any: Vec::new(), passed_over: None, words },
            }
        })
        .collect()
}

/// The change he asked for, if one is open: the oldest, since nothing asks again while one stands (`askedChange`).
fn asked_change(changes: &[WorkflowChange]) -> Option<&WorkflowChange> {
    changes.iter().find(|one| one.step_id.is_none())
}

/// How many lines unchanged words are held to (`<Clamped lines={3}>`).
const CLAMP: usize = 3;

/// The words a proposed change holds to three lines, in the order they are drawn: each card's Words on each side,
/// where they are there and do not differ. Words that differ are drawn whole, so a marked sentence is never behind
/// View more.
fn passages(row: &str, phase: &Phase) -> Vec<String> {
    let Phase::Proposed { before, after, .. } = phase else { return Vec::new() };
    let (changes, _) = step_changes(before, after);
    let mut found = Vec::new();
    for change in &changes {
        let (label, sides): (&str, Vec<(&str, &Step)>) = match change {
            StepChange::Added { after, .. } => (&after.label, vec![("After", after)]),
            StepChange::Removed { before, .. } => (&before.label, vec![("Before", before)]),
            StepChange::Changed { before, after, .. } => {
                if field_of(before, "Words") != field_of(after, "Words") {
                    continue;
                }
                (&after.label, vec![("Before", before), ("After", after)])
            }
        };
        for (side, step) in sides {
            if !field_of(step, "Words").is_empty() {
                found.push(passage_key(row, label, side));
            }
        }
    }
    found
}

fn passage_key(row: &str, label: &str, side: &str) -> String {
    format!("{row}|{label}|{side}")
}

/// The band's phase from the open changes (`agentOf`).
pub fn phase_of(changes: &[WorkflowChange], known: &[Step]) -> Phase {
    let Some(one) = asked_change(changes) else { return Phase::Idle };
    let asked = match &one.origin {
        ChangeOrigin::Asked { text } => Some(text.clone()),
        _ => None,
    };
    match one.state {
        ChangeState::Drafting => Phase::Working { asked },
        ChangeState::Failed => Phase::Failed { reason: one.reason.clone().unwrap_or_default(), asked },
        ChangeState::Cannot => Phase::Cannot { asked, missing: one.missing.clone() },
        ChangeState::Ready if one.after.is_some() => {
            // Fleet's own reading of each side says what ends a step it adds; the text alone cannot.
            let before = one.before_steps.as_ref().map_or_else(|| steps_of_text(&one.before, known), |steps| steps.iter().map(step_of_change).collect());
            let after = one.after_steps.as_ref().map_or_else(|| steps_of_text(one.after.as_deref().unwrap_or_default(), known), |steps| steps.iter().map(step_of_change).collect());
            Phase::Proposed { before, after, asked, fresh: matches!(one.origin, ChangeOrigin::New) }
        }
        _ => Phase::Idle,
    }
}

// ---- the state ---------------------------------------------------------------------------------

struct Boxed {
    state: Entity<TextareaState>,
    _changed: Subscription,
}

/// The band's own state: each copy's open changes as read, by row key; the box that asks and the notes box while
/// one is being written, by workflow id, as the page keeps them across the copy a first ask makes; what the last
/// act said; the Job a press started; and the read waiting while a change drafts.
#[derive(Default)]
pub struct Band {
    changes: BTreeMap<String, Vec<WorkflowChange>>,
    asks: BTreeMap<String, Boxed>,
    notes: Option<(String, Boxed)>,
    /// What he asked last on a row, said over the change until Fleet's reading of it carries his words.
    asked: BTreeMap<String, String>,
    refused: Option<(String, String)>,
    /// The Job Start a job to build it started, and the line said beside it.
    started: Option<(String, String, Option<String>)>,
    busy: bool,
    poll: Option<Task<()>>,
    /// Set when a change turns proposed or cannot, and cleared once it has been brought into view.
    reveal: Rc<Cell<bool>>,
    /// Each clamped passage's key, and whether its words run past the clamp at the width last drawn.
    overflows: Rc<RefCell<BTreeMap<String, bool>>>,
    /// The passages opened with View more.
    open: BTreeSet<String>,
    /// Each passage's View more.
    more: BTreeMap<String, FocusHandle>,
}

impl Band {
    pub fn in_text(&self, window: &Window, cx: &App) -> bool {
        self.asks.values().chain(self.notes.as_ref().map(|(_, notes)| notes)).any(|one| one.state.read(cx).focus_handle(cx).is_focused(window))
    }

    fn typed(one: &Boxed, cx: &App) -> String {
        one.state.read(cx).value().to_string()
    }
}

/// The band's acts, each a key of the detail's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BandAct {
    Send,
    Dismiss,
    SendBack,
    SendNotes,
    Approve,
    Build,
    OpenJob,
}

impl BandAct {
    const ALL: [BandAct; 7] = [BandAct::Send, BandAct::Dismiss, BandAct::SendBack, BandAct::SendNotes, BandAct::Approve, BandAct::Build, BandAct::OpenJob];

    fn key(self) -> &'static str {
        match self {
            BandAct::Send => "band:send",
            BandAct::Dismiss => "band:dismiss",
            BandAct::SendBack => "band:send-back",
            BandAct::SendNotes => "band:send-notes",
            BandAct::Approve => "band:approve",
            BandAct::Build => "band:build",
            BandAct::OpenJob => "band:open-job",
        }
    }
}

/// Where the band's keys rest once the control they were on has gone: the band itself.
const AT_REST: &str = "band";

fn editable(one: &Listed) -> bool {
    one.place != Place::Armada && one.runs_in.is_none()
}

impl SettingsOverlay {
    fn known_steps(&self, key: &str) -> Vec<Step> {
        match self.workflows.detail.reads.get(key) {
            Some(StepsRead::Read(steps)) => steps.clone(),
            _ => Vec::new(),
        }
    }

    fn band_phase(&self, one: &Listed) -> Phase {
        let key = one.key();
        phase_of(self.workflows.detail.band.changes.get(&key).map_or(&[], Vec::as_slice), &self.known_steps(&key))
    }

    /// The band's keys and its box made before they are drawn, and the copy's changes read once.
    pub(super) fn band_keys(&mut self, one: &Listed, window: &mut Window, cx: &mut Context<Self>) {
        for name in BandAct::ALL.iter().map(|act| act.key()).chain([AT_REST]) {
            self.workflows.detail.keys.entry(name.to_string()).or_insert_with(|| cx.focus_handle().tab_stop(name != AT_REST));
        }
        let key = one.key();
        if !self.workflows.detail.band.asks.contains_key(&one.id) {
            let boxed = self.band_box(&format!("workflows:agent:{}", one.id), 2, 12, window, cx);
            self.workflows.detail.band.asks.insert(one.id.clone(), boxed);
        }
        // Every row's changes are read: an agent's change on a shipped row lands in its draft, where nothing is copied until Publish.
        if !self.workflows.detail.band.changes.contains_key(&key) {
            self.workflows.detail.band.changes.insert(key.clone(), Vec::new());
            self.read_changes(one.clone(), window, cx);
        }
        for passage in passages(&key, &self.band_phase(one)) {
            self.workflows.detail.band.more.entry(passage).or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
    }

    /// A box that keeps what is typed in it under `keep` until it is sent.
    fn band_box(&mut self, keep: &str, rows: usize, most: usize, window: &mut Window, cx: &mut Context<Self>) -> Boxed {
        let state = cx.new(|cx| TextareaState::new(window, cx).auto_grow(rows, most));
        if let Some(kept) = crate::drafts::kept(keep, cx) {
            state.update(cx, |state, cx| state.set_value(kept, window, cx));
        }
        let keep = keep.to_string();
        let changed = cx.subscribe_in(&state, window, move |_, state, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                let text = state.read(cx).value().to_string();
                if text.is_empty() {
                    crate::drafts::clear(&keep, cx);
                } else {
                    crate::drafts::keep(&keep, &text, cx);
                }
                cx.notify();
            }
        });
        Boxed { state, _changed: changed }
    }

    /// `GET /workflows/changes` for a copy; read again each beat while one drafts and the copy is still shown.
    fn read_changes(&mut self, one: Listed, window: &mut Window, cx: &mut Context<Self>) {
        let task = self.wire.workflow_changes(&one.id, wire_of(one.place), &one.manifest, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, window, cx| this.changes_read(one, answer, window, cx));
        })
        .detach();
    }

    fn changes_read(&mut self, one: Listed, answer: Result<ipc::WorkflowChanges, Refused>, window: &mut Window, cx: &mut Context<Self>) {
        let key = one.key();
        let Ok(read) = answer else { return };
        let drafting = read.changes.iter().any(|change| change.state == ChangeState::Drafting);
        // An asked change that is ready goes into the row's draft, marked as the agent's, and is put aside on Fleet: the
        // draft holds it now.
        let mut changes = read.changes;
        let ready = changes.iter().position(|change| change.step_id.is_none() && change.state == ChangeState::Ready && change.after.is_some() && matches!(change.origin, ChangeOrigin::Asked { .. }));
        let landed = match (ready, self.editor_of(&one)) {
            (Some(at), Some(editor)) => {
                let change = changes.remove(at);
                if let (ChangeOrigin::Asked { text }, Some(after)) = (change.origin, change.after) {
                    editor.update(cx, |editor, cx| editor.agent_edit(text, change.before, after, cx));
                }
                Some(change.id)
            }
            _ => None,
        };
        if let Some(change) = landed {
            let dismissed = self.wire.dismiss_change(change, cx);
            cx.spawn(async move |_, _| {
                let _ = dismissed.await;
            })
            .detach();
        }
        let band = &mut self.workflows.detail.band;
        if !matches!(phase_of(&changes, &[]), Phase::Proposed { .. }) && band.notes.as_ref().is_some_and(|(id, _)| *id == one.id) {
            band.notes = None;
        }
        let shows = |changes: &[WorkflowChange]| matches!(phase_of(changes, &[]), Phase::Proposed { .. } | Phase::Cannot { .. });
        // Brought into view as it arrives, the way Classic's `scrollIntoView({ block: "nearest" })` does.
        if shows(&changes) && !band.changes.get(&key).is_some_and(|was| shows(was)) {
            band.reveal.set(true);
        }
        band.changes.insert(key.clone(), changes);
        let band = &mut self.workflows.detail.band;
        if drafting {
            band.poll = Some(cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(POLL).await;
                let _ = this.update_in(cx, |this, window, cx| {
                    if this.shown_workflows(cx).1.is_some_and(|shown| shown.key() == one.key()) {
                        this.read_changes(one, window, cx);
                    }
                });
            }));
        }
        cx.notify();
    }

    /// A press on one of the band's acts.
    fn press_band(&mut self, act: BandAct, window: &mut Window, cx: &mut Context<Self>) {
        let (_, Some(one)) = self.shown_workflows(cx) else { return };
        let key = one.key();
        match act {
            BandAct::SendBack => {
                let boxed = self.band_box(&format!("workflows:notes:{}", one.id), 2, 8, window, cx);
                let focus = boxed.state.read(cx).focus_handle(cx);
                self.workflows.detail.band.notes = Some((one.id.clone(), boxed));
                window.focus(&focus, cx);
                cx.notify();
                return;
            }
            BandAct::OpenJob => {
                if let Some(job) = self.workflows.detail.band.started.as_ref().and_then(|(_, _, job)| job.clone()) {
                    self.open_job(job, window, cx);
                }
                return;
            }
            _ => {}
        }
        if self.workflows.detail.band.busy || !self.live {
            return;
        }
        let change = self.workflows.detail.band.changes.get(&key).and_then(|changes| asked_change(changes)).map(|change| change.id);
        let manifest = one.manifest.clone();
        let task: Task<Result<Option<String>, Refused>> = match act {
            BandAct::Send => {
                let Some(text) = self.workflows.detail.band.asks.get(&one.id).map(|one| Band::typed(one, cx).trim().to_string()).filter(|text| !text.is_empty()) else { return };
                self.workflows.detail.band.asked.insert(key.clone(), text.clone());
                let asking = AskWorkflowChange { workflow_id: WorkflowId::carried(&one.id), source: wire_of(one.place).into(), text, ..Default::default() };
                let task = self.wire.ask_change(&manifest, asking, cx);
                cx.background_spawn(async move { task.await.map(|_| None) })
            }
            BandAct::SendNotes => {
                let (Some(change), Some(notes)) = (change, self.workflows.detail.band.notes.as_ref().filter(|(id, _)| *id == one.id).map(|(_, notes)| Band::typed(notes, cx).trim().to_string()).filter(|notes| !notes.is_empty())) else { return };
                let task = self.wire.send_back_change(SendBackWorkflowChange { change, notes }, cx);
                cx.background_spawn(async move { task.await.map(|_| None) })
            }
            BandAct::Approve | BandAct::Dismiss | BandAct::Build => {
                let Some(change) = change else { return };
                match act {
                    BandAct::Approve => {
                        let task = self.wire.approve_change(change, cx);
                        cx.background_spawn(async move { task.await.map(|_| None) })
                    }
                    BandAct::Dismiss => {
                        let task = self.wire.dismiss_change(change, cx);
                        cx.background_spawn(async move { task.await.map(|_| None) })
                    }
                    _ => {
                        let task = self.wire.build_support(change, cx);
                        cx.background_spawn(async move { task.await.map(|building| Some(building.job_id.as_str().to_string())) })
                    }
                }
            }
            BandAct::SendBack | BandAct::OpenJob => return,
        };
        let band = &mut self.workflows.detail.band;
        band.busy = true;
        band.refused = None;
        // The control pressed goes with the phase it was in; the keys wait on the band until the next is drawn.
        if let Some(rest) = self.workflows.detail.keys.get(AT_REST).cloned() {
            window.focus(&rest, cx);
        }
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, window, cx| this.band_done(&one, act, answer, window, cx));
        })
        .detach();
    }

    /// Send on a workflow that is not a copy: the copy is made first, the change asked on it, and the page then
    /// picks the copy (`inCopy`). A refusal at the copy shows on the row it was asked on, one at the ask on the copy.
    fn send_on_copy(&mut self, one: Listed, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let band = &mut self.workflows.detail.band;
        band.busy = true;
        band.refused = None;
        if let Some(rest) = self.workflows.detail.keys.get(AT_REST).cloned() {
            window.focus(&rest, cx);
        }
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let (copy, answer) = match make_copy(&one, &this, cx).await {
                Err(why) => (None, Err(why)),
                Ok(copy) => {
                    let _ = this.update(cx, |this, _| {
                        this.workflows.detail.band.asked.insert(copy.key(), text.clone());
                    });
                    let asking = AskWorkflowChange { workflow_id: WorkflowId::carried(&copy.id), source: wire_of(copy.place).into(), text, ..Default::default() };
                    let answer = ask(&this, cx, |this, cx| this.wire.ask_change(&copy.manifest, asking, cx)).await.map(|_| ());
                    (Some(copy), answer)
                }
            };
            let _ = this.update_in(cx, |this, window, cx| this.sent_on_copy(&one, copy, answer, window, cx));
        })
        .detach();
    }

    fn sent_on_copy(&mut self, one: &Listed, copy: Option<Listed>, answer: Result<(), Refused>, window: &mut Window, cx: &mut Context<Self>) {
        let band = &mut self.workflows.detail.band;
        band.busy = false;
        let at = copy.as_ref().map_or_else(|| one.key(), Listed::key);
        match answer {
            Err(why) => band.refused = Some((at.clone(), why.said)),
            Ok(()) => {
                if let Some(ask) = band.asks.get(&one.id) {
                    ask.state.update(cx, |state, cx| state.set_value(String::new(), window, cx));
                }
                crate::drafts::clear(&format!("workflows:agent:{}", one.id), cx);
            }
        }
        if copy.is_some() {
            // The copy's changes are read when it is drawn, so the one just asked is found drafting.
            self.workflows.detail.band.changes.remove(&at);
            self.read_list(Some(at), None, window, cx);
        }
        cx.notify();
    }

    fn band_done(&mut self, one: &Listed, act: BandAct, answer: Result<Option<String>, Refused>, window: &mut Window, cx: &mut Context<Self>) {
        let key = one.key();
        // A Fleet that draws no change on Armada's own workflow: the copy is made first, as it was before the draft.
        if let (BandAct::Send, Err(why)) = (act, &answer) {
            let typed = self.workflows.detail.band.asks.get(&one.id).map(|ask| Band::typed(ask, cx).trim().to_string()).filter(|text| !text.is_empty());
            if let (true, true, Some(text)) = (why.is("fleet.workflow_is_armadas"), !editable(one), typed) {
                self.workflows.detail.band.asked.remove(&key);
                self.send_on_copy(one.clone(), text, window, cx);
                return;
            }
        }
        let band = &mut self.workflows.detail.band;
        band.busy = false;
        match &answer {
            Err(why) => band.refused = Some((key.clone(), why.said.clone())),
            Ok(job) => match act {
                BandAct::Send => {
                    band.started = None;
                    if let Some(ask) = band.asks.get(&one.id) {
                        ask.state.update(cx, |state, cx| state.set_value(String::new(), window, cx));
                    }
                    crate::drafts::clear(&format!("workflows:agent:{}", one.id), cx);
                }
                BandAct::SendNotes => {
                    band.notes = None;
                    crate::drafts::clear(&format!("workflows:notes:{}", one.id), cx);
                }
                BandAct::Build => band.started = Some((key.clone(), "A job started to build it".into(), job.clone())),
                _ => {}
            },
        }
        // Approve and Dismiss read the copy again, as Classic's `refresh` does, whether or not Fleet took them.
        if matches!(act, BandAct::Approve | BandAct::Dismiss) {
            self.read_steps(one.clone(), window, cx);
            self.read_list(None, None, window, cx);
        }
        if answer.is_ok() || matches!(act, BandAct::Approve | BandAct::Dismiss | BandAct::SendNotes | BandAct::Build) {
            self.read_changes(one.clone(), window, cx);
        }
        cx.notify();
    }

    /// The band's stops, after the cards': what its phase draws, in the order it draws them.
    pub(super) fn band_stops(&self, one: &Listed, cx: &App) -> Vec<FocusHandle> {
        let key = one.key();
        let detail = &self.workflows.detail;
        let band = &detail.band;
        let act = |act: BandAct| detail.keys.get(act.key()).cloned();
        let mut stops = Vec::new();
        match self.band_phase(one) {
            Phase::Idle => {
                if band.started.as_ref().is_some_and(|(row, _, job)| *row == key && job.is_some()) {
                    stops.extend(act(BandAct::OpenJob));
                }
                stops.extend(band.asks.get(&one.id).map(|ask| ask.state.read(cx).focus_handle(cx)));
                stops.extend(act(BandAct::Send));
            }
            Phase::Working { .. } => {}
            Phase::Failed { .. } => stops.extend(act(BandAct::Dismiss)),
            Phase::Cannot { .. } => stops.extend([act(BandAct::Dismiss), act(BandAct::Build)].into_iter().flatten()),
            phase @ Phase::Proposed { .. } => {
                let fresh = matches!(phase, Phase::Proposed { fresh: true, .. });
                let overflows = band.overflows.borrow();
                for passage in passages(&key, &phase) {
                    if band.open.contains(&passage) || overflows.get(&passage).copied().unwrap_or(false) {
                        stops.extend(band.more.get(&passage).cloned());
                    }
                }
                stops.extend(act(BandAct::Dismiss));
                match band.notes.as_ref().filter(|(id, _)| *id == one.id) {
                    Some((_, notes)) => {
                        stops.push(notes.state.read(cx).focus_handle(cx));
                        stops.extend(act(BandAct::SendNotes));
                    }
                    None if !fresh => stops.extend(act(BandAct::SendBack)),
                    None => {}
                }
                stops.extend(act(BandAct::Approve));
            }
        }
        stops
    }

    /// The band's keys: Enter sends what the box holds and Shift+Enter breaks the line; ⌘Enter sends the notes and
    /// Esc puts them away; Enter and Space press an act.
    pub(super) fn band_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let (_, Some(one)) = self.shown_workflows(cx) else { return false };
        let row = one.id.clone();
        let mods = &key.modifiers;
        let word = key.key.as_str();
        let band = &self.workflows.detail.band;
        if band.asks.get(&row).is_some_and(|ask| ask.state.read(cx).focus_handle(cx).is_focused(window)) {
            if word == "enter" && !mods.shift && !mods.platform && !mods.control && !mods.alt {
                self.press_band(BandAct::Send, window, cx);
                return true;
            }
            return false;
        }
        if let Some((_, notes)) = band.notes.as_ref().filter(|(at, _)| *at == row) {
            if notes.state.read(cx).focus_handle(cx).is_focused(window) {
                if word == "escape" {
                    self.workflows.detail.band.notes = None;
                    if let Some(back) = self.workflows.detail.keys.get(BandAct::SendBack.key()).cloned() {
                        window.focus(&back, cx);
                    }
                    cx.notify();
                    return true;
                }
                if word == "enter" && (mods.platform || mods.control) {
                    self.press_band(BandAct::SendNotes, window, cx);
                    return true;
                }
                return false;
            }
        }
        if mods.platform || mods.control || mods.alt || !matches!(word, "enter" | "space") {
            return false;
        }
        if let Some(passage) = self.workflows.detail.band.more.iter().find(|(_, focus)| focus.is_focused(window)).map(|(passage, _)| passage.clone()) {
            self.toggle_passage(passage, cx);
            return true;
        }
        let Some(act) = BandAct::ALL.into_iter().find(|act| self.workflows.detail.keys.get(act.key()).is_some_and(|focus| focus.is_focused(window))) else { return false };
        self.press_band(act, window, cx);
        true
    }

    // ---- drawn --------------------------------------------------------------------------------

    /// The band over the lanes, on every workflow: the first ask on Armada's own, or on what runs in a repository
    /// that keeps no copy, makes the copy.
    pub(super) fn render_change_band(&self, one: &Listed, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let key = one.key();
        let detail = &self.workflows.detail;
        let band = &detail.band;
        let rest = detail.keys.get(AT_REST)?.clone();
        let refused = band.refused.as_ref().filter(|(row, _)| *row == key).map(|(_, said)| alert(look, "workflow-band-refused", said));
        let phase = self.band_phase(one);
        let request = match &phase {
            Phase::Idle => None,
            Phase::Working { asked } | Phase::Failed { asked, .. } | Phase::Cannot { asked, .. } | Phase::Proposed { asked, .. } => asked.clone(),
        }
        .or_else(|| band.asked.get(&key).cloned())
        .unwrap_or_default();
        let off = band.busy || !self.live;
        let drawn: AnyElement = match phase {
            Phase::Working { .. } => div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(labelled(div().id("workflow-band-working").flex_none().tooltip(tip("The agent is working")).child(crate::titlebar::parts::spinner("workflow-band-spinner", look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH) * 2., look.color(token::FG_MUTED), cx)), Role::Image, "The agent is working"))
                .child(div().min_w_0().child(SharedString::from(request)))
                .into_any_element(),
            Phase::Failed { reason, .. } => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(alert(look, "workflow-band-failed", &reason))
                .children(refused)
                .child(div().flex().justify_end().child(self.band_button(BandAct::Dismiss, "Dismiss", Weight::Secondary, off, look, window, cx)))
                .into_any_element(),
            Phase::Cannot { missing, .. } => {
                let rows = missing.iter().enumerate().map(|(n, one)| {
                    div()
                        .flex()
                        .items_start()
                        .gap(look.length(token::SPACE_2))
                        .child(
                            div()
                                .flex_none()
                                .w(look.length(token::SPACE_6))
                                .h(look.length(token::LEADING_SM))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(labelled(div().id(SharedString::from(format!("workflow-missing-{n}"))).tooltip(tip("Not built yet")).child(svg().path(icon_path("circle-dashed")).size(look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH) * 2.).text_color(look.color(token::FG_MUTED))), Role::Image, "Not built yet")),
                        )
                        .child(div().min_w_0().child(SharedString::from(one.clone())))
                });
                let list = labelled(div().id("workflow-missing").flex().flex_col().gap(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).children(rows), Role::List, "Missing");
                region(proposed_frame(look)
                    .child(self.reveal())
                    .child(asked_line(look, &request))
                    .child(div().px(look.length(token::SPACE_4)).pt(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child("Armada can't do this yet"))
                    .child(div().flex().flex_col().gap(look.length(token::SPACE_1)).px(look.length(token::SPACE_4)).py(look.length(token::SPACE_3)).child(caption(look, "Missing")).child(list))
                    .children(refused.map(|said| div().px(look.length(token::SPACE_4)).child(said)))
                    .child(acts_row(look).child(self.band_button(BandAct::Dismiss, "Dismiss", Weight::Ghost, off, look, window, cx)).child(self.band_button(BandAct::Build, "Start a job to build it", Weight::Primary, off, look, window, cx)))
                    )
            }
            Phase::Proposed { before, after, fresh, .. } => {
                let (changes, unchanged) = step_changes(&before, &after);
                let notes = band.notes.as_ref().filter(|(id, _)| *id == one.id).map(|(_, notes)| notes);
                let typed = notes.map(|notes| Band::typed(notes, cx).trim().to_string()).unwrap_or_default();
                let mut acts = acts_row(look).child(self.band_button(BandAct::Dismiss, "Dismiss", Weight::Ghost, off, look, window, cx));
                if !fresh {
                    acts = acts.child(match notes {
                        None => self.band_button(BandAct::SendBack, "Send back", Weight::Secondary, off, look, window, cx),
                        Some(_) => self.band_button(BandAct::SendNotes, "Send notes", Weight::Secondary, off || typed.is_empty(), look, window, cx),
                    });
                }
                acts = acts.child(self.band_button(BandAct::Approve, "Approve", Weight::Primary, off, look, window, cx));
                let held = |label: &str, side: &str, text: String, words: gpui::Div| self.held(&key, label, side, text, words, look, window, cx);
                region(proposed_frame(look)
                    .child(self.reveal())
                    .child(asked_line(look, &request))
                    .child(div().flex().flex_col().gap(look.length(token::SPACE_3)).px(look.length(token::SPACE_4)).py(look.length(token::SPACE_3)).children(changes.iter().map(|change| change_card(look, change, &held))))
                    .children((!unchanged.is_empty()).then(|| {
                        div()
                            .flex()
                            .flex_wrap()
                            .items_baseline()
                            .gap_x(look.length(token::SPACE_3))
                            .px(look.length(token::SPACE_4))
                            .pb(look.length(token::SPACE_3))
                            .text_size(look.length(token::TEXT_SM))
                            .text_color(look.color(token::FG_MUTED))
                            .child(caption(look, "Unchanged"))
                            .child(SharedString::from(unchanged.join(", ")))
                    }))
                    .children(notes.map(|notes| {
                        div()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .px(look.length(token::SPACE_4))
                            .pb(look.length(token::SPACE_3))
                            .child(caption(look, "Notes for the agent"))
                            .child(labelled(div().id("workflow-band-notes").key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).child(Textarea::new(&notes.state)), Role::TextInput, "Notes for the agent"))
                    }))
                    .children(refused.map(|said| div().px(look.length(token::SPACE_4)).child(said)))
                    .child(acts)
                    )
            }
            Phase::Idle => {
                let ask = band.asks.get(&one.id)?;
                let empty = Band::typed(ask, cx).trim().is_empty();
                let started = band.started.as_ref().filter(|(row, _, _)| *row == key).map(|(_, words, job)| {
                    let line = div()
                        .id("workflow-band-started")
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_XS))
                        .text_color(look.color(token::FG_MUTED))
                        .child(svg().flex_none().path(icon_path("check")).size(look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH) * 2.).text_color(look.color(token::STATUS_COMPLETED_SUCCESS)))
                        .child(SharedString::from(words.clone()))
                        .children(job.as_ref().map(|_| self.band_button(BandAct::OpenJob, "Open Job", Weight::Ghost, false, look, window, cx)));
                    labelled(line, Role::Status, words.clone())
                });
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .children(started)
                    .children(refused)
                    .child(caption(look, "Tell an agent what to change"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_2))
                            .p(look.length(token::SPACE_2))
                            .rounded(look.length(token::RADIUS_LG))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::BORDER_DEFAULT))
                            .bg(look.color(token::BG_SUNKEN))
                            .child(labelled(div().id("workflow-band-ask").key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).child(Textarea::new(&ask.state).disabled(off)), Role::TextInput, "Tell an agent what to change"))
                            .child(div().flex().justify_end().child(self.band_button(BandAct::Send, "Send to agent", Weight::Secondary, off || empty, look, window, cx))),
                    )
                    .into_any_element()
            }
        };
        Some(labelled(div().id("workflow-band").track_focus(&rest).child(drawn), Role::Group, "Tell an agent what to change").into_any_element())
    }

    fn toggle_passage(&mut self, passage: String, cx: &mut Context<Self>) {
        let open = &mut self.workflows.detail.band.open;
        if !open.remove(&passage) {
            open.insert(passage);
        }
        cx.notify();
    }

    /// Unchanged words held to three lines, with View more only where they run past them (`Clamped`). The lines
    /// are counted from the words shaped at the width drawn, so a narrower window finds the overflow it makes.
    #[allow(clippy::too_many_arguments)]
    fn held(&self, row: &str, label: &str, side: &str, text: String, words: gpui::Div, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let passage = passage_key(row, label, side);
        let band = &self.workflows.detail.band;
        let open = band.open.contains(&passage);
        let overflows = band.overflows.clone();
        let measured = passage.clone();
        let measure = canvas(
            move |bounds: Bounds<Pixels>, window, _| {
                let style = window.text_style();
                let size = style.font_size.to_pixels(window.rem_size());
                let lines = window
                    .text_system()
                    .shape_text(SharedString::from(text.clone()), size, &[style.to_run(text.len())], Some(bounds.size.width), None)
                    .map_or(0, |lines| lines.iter().map(|line| line.wrap_boundaries().len() + 1).sum::<usize>());
                let over = lines > CLAMP;
                if overflows.borrow_mut().insert(measured.clone(), over) != Some(over) {
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        let words = words.relative().when(!open, |words| words.line_clamp(CLAMP).text_ellipsis()).child(measure);
        let shown = open || band.overflows.borrow().get(&passage).copied().unwrap_or(false);
        let more = band.more.get(&passage).cloned().filter(|_| shown).map(|focus| {
            let said = if open { "View less" } else { "View more" };
            let button = div()
                .id(SharedString::from(format!("workflow-more-{passage}")))
                .flex_none()
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(look.color(token::ACCENT))
                .cursor_pointer()
                .hover(|style| style.underline())
                .child(said);
            let toggled = passage.clone();
            let button = ringed(look, button, &focus, window).track_focus(&focus).on_click(cx.listener(move |this, _, _, cx| this.toggle_passage(toggled.clone(), cx)));
            labelled(button, Role::Button, said).into_any_element()
        });
        div().flex().flex_col().items_start().gap(look.length(token::SPACE_1)).min_w_0().child(words).children(more).into_any_element()
    }

    /// Measures the proposed change where it is drawn and, the first time it is, scrolls the detail the least that
    /// shows it whole, or its top where it is taller than the panel.
    fn reveal(&self) -> impl IntoElement {
        let reveal = self.workflows.detail.band.reveal.clone();
        let scroll: ScrollHandle = self.workflows.detail.scroll.clone();
        canvas(
            move |at: Bounds<Pixels>, window, _| {
                if !reveal.replace(false) {
                    return;
                }
                let view = scroll.bounds();
                let mut by = Pixels::ZERO;
                if at.bottom() > view.bottom() {
                    by = at.bottom() - view.bottom();
                }
                if at.top() - by < view.top() {
                    by = at.top() - view.top();
                }
                if by == Pixels::ZERO {
                    return;
                }
                let offset = scroll.offset();
                let y = (offset.y - by).max(-scroll.max_offset().y).min(Pixels::ZERO);
                scroll.set_offset(point(offset.x, y));
                window.refresh();
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full()
    }

    #[allow(clippy::too_many_arguments)]
    fn band_button(&self, act: BandAct, said: &'static str, weight: Weight, off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.workflows.detail.keys.get(act.key()).cloned() else { return div().into_any_element() };
        let id = SharedString::from(format!("workflow-{}", act.key().replace(':', "-")));
        let button = match weight {
            Weight::Ghost => super::view::ghost(look, id, said, off),
            Weight::Secondary => sunken_button(look, id, said, off),
            Weight::Primary => filled(look, id, said, off),
        };
        let button = ringed(look, button, &focus, window).track_focus(&focus).when(!off, |button| button.on_click(cx.listener(move |this, _, window, cx| this.press_band(act, window, cx))));
        labelled(button, Role::Button, said).into_any_element()
    }
}

#[derive(Clone, Copy)]
enum Weight {
    Ghost,
    Secondary,
    Primary,
}

/// The primary button: the accent's fill (`Button variant="primary" size="sm"`).
pub(crate) fn filled(look: &Look, id: SharedString, said: &'static str, off: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::ACCENT))
        .bg(look.color(token::ACCENT))
        .text_size(look.length(token::TEXT_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(look.color(token::FG_INVERSE))
        .when(off, |button| button.opacity(0.5))
        .when(!off, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::ACCENT_HOVER))))
        .child(said)
}

fn caption(look: &Look, said: &'static str) -> gpui::Div {
    div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_MUTED)).child(said)
}

fn alert(look: &Look, id: &'static str, said: &str) -> AnyElement {
    labelled(div().id(id).text_size(look.length(token::TEXT_SM)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(SharedString::from(said.to_string())), Role::Alert, said.to_string()).into_any_element()
}

/// The proposed change's frame and band; `region` labels it once it is whole.
fn proposed_frame(look: &Look) -> gpui::Stateful<gpui::Div> {
    let frame = div()
        .id("workflow-proposed")
        .flex()
        .flex_col()
        .min_w_0()
        .relative()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_SUNKEN))
        .overflow_hidden()
        .child(band(look).child("Proposed change"));
    frame
}

fn region(frame: gpui::Stateful<gpui::Div>) -> AnyElement {
    labelled(frame, Role::Region, "Proposed change").into_any_element()
}

fn asked_line(look: &Look, request: &str) -> gpui::Div {
    div().px(look.length(token::SPACE_4)).pt(look.length(token::SPACE_3)).text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(request.to_string()))
}

fn acts_row(look: &Look) -> gpui::Div {
    div().flex().flex_wrap().items_center().justify_end().gap(look.length(token::SPACE_2)).px(look.length(token::SPACE_4)).pb(look.length(token::SPACE_3))
}

/// One step a change touches: its place and name, how it changed, and each fact before and after (`StepChangeCard`).
fn change_card(look: &Look, change: &StepChange, held: &dyn Fn(&str, &str, String, gpui::Div) -> AnyElement) -> AnyElement {
    let (n, kind, before, after) = match change {
        StepChange::Added { n, after } => (*n, "Added", None, Some(after)),
        StepChange::Removed { n, before } => (*n, "Removed", Some(before), None),
        StepChange::Changed { n, before, after, .. } => (*n, "Changed", Some(before), Some(after)),
    };
    let label = after.or(before).map(|one| one.label.clone()).unwrap_or_default();
    let (glyph, sign) = match change {
        StepChange::Added { .. } => ("list-plus", look.color(token::DIFF_ADD_FG)),
        StepChange::Removed { .. } => ("list-minus", look.color(token::DIFF_DEL_FG)),
        StepChange::Changed { .. } => ("git-compare", look.color(token::ACCENT)),
    };
    let sides: &[&str] = match change {
        StepChange::Changed { .. } => &["Before", "After"],
        StepChange::Added { .. } => &["After"],
        StepChange::Removed { .. } => &["Before"],
    };
    // A changed step also sets its place first, so a step that only moved shows what moved.
    let mut rows: Vec<(&str, String, String)> = Vec::new();
    if let StepChange::Changed { n, was, .. } = change {
        rows.push(("Place", was.to_string(), n.to_string()));
    }
    for field in FIELDS {
        rows.push((field, before.map(|one| field_of(one, field)).unwrap_or_default(), after.map(|one| field_of(one, field)).unwrap_or_default()));
    }
    let changed = matches!(change, StepChange::Changed { .. });
    let removed = matches!(change, StepChange::Removed { .. });
    let name_col = look.length(token::TEXT_SM) * 9.;
    let head = div()
        .flex()
        .gap(look.length(token::SPACE_3))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_1))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_SUBTLE))
        .child(div().flex_none().w(name_col).child("Field"))
        .children(sides.iter().map(|side| div().flex_1().min_w_0().child(*side)));
    let body = rows.into_iter().map(|(field, was, is)| {
        let differs = changed && was != is;
        let ink = if removed || (changed && !differs) { look.color(token::FG_MUTED) } else { look.color(token::FG_DEFAULT) };
        div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .child(
                div()
                    .flex_none()
                    .w(name_col)
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .font_weight(FontWeight(look.number(if differs { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
                    .text_color(if differs { look.color(token::ACCENT) } else { look.color(token::FG_SUBTLE) })
                    .child(field),
            )
            .children(sides.iter().map(|side| {
                let (value, against, mark) = if *side == "Before" { (&was, &is, look.color(token::STATUS_COMPLETED_FAILED_BG)) } else { (&is, &was, look.color(token::STATUS_COMPLETED_SUCCESS_BG)) };
                let cell = div().flex_1().min_w_0().text_color(ink);
                // Words that differ are drawn whole, so the marked sentence is never behind View more. The strike is
                // on the words alone; View more is not struck.
                if field == "Words" && !differs && !value.is_empty() {
                    let (text, words) = words_of(value, changed.then_some(against.as_str()), mark);
                    return cell.child(held(&label, side, text, words.when(removed, |words| words.line_through())));
                }
                cell.when(removed, |cell| cell.line_through()).child(field_value(look, value, field == "Words", changed.then_some(against.as_str()), mark))
            }))
    });
    labelled(
        div()
            .id(SharedString::from(format!("workflow-change-{kind}-{label}")))
            .flex()
            .flex_col()
            .min_w_0()
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .relative()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .min_h(look.length(token::H_ROW))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_1))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_RAISED))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(div().text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(n.to_string())))
                    .child(div().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(label.clone())))
                    .child(
                        div()
                            .flex_none()
                            .ml_auto()
                            .w(look.length(token::SPACE_6))
                            .flex()
                            .justify_center()
                            .child(labelled(div().id(SharedString::from(format!("workflow-change-sign-{label}"))).tooltip(tip(kind)).child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH) * 2.).text_color(sign)), Role::Image, kind)),
                    ),
            )
            .child(head)
            .children(body)
            // An added or removed step's edge in the diff's hue for the kind, over the whole card.
            .when(!changed, |card| card.child(div().absolute().left_0().top_0().bottom_0().w(look.length(token::BORDER_WIDTH) * 3.).bg(sign))),
        Role::Group,
        label,
    )
    .into_any_element()
}

/// One fact on one side: `nothing` where the step has none; several ends a line each; words with each sentence the
/// other side lacks marked (`FieldValue`).
fn field_value(look: &Look, value: &str, words: bool, against: Option<&str>, mark: Hsla) -> AnyElement {
    if value.is_empty() {
        return div().text_color(look.color(token::FG_SUBTLE)).child("nothing").into_any_element();
    }
    if !words && value.contains('\n') {
        return div().flex().flex_col().gap(look.length(token::SPACE_1)).children(value.split('\n').map(|one| SharedString::from(one.to_string()))).into_any_element();
    }
    if !words {
        return div().child(SharedString::from(value.to_string())).into_any_element();
    }
    words_of(value, against, mark).1.into_any_element()
}

/// Words with each sentence the other side lacks marked, and the text they draw.
fn words_of(value: &str, against: Option<&str>, mark: Hsla) -> (String, gpui::Div) {
    let other = against.map(sentences_of);
    let mut text = String::new();
    let mut marks = Vec::new();
    for one in sentences_of(value) {
        let from = text.len();
        text.push_str(&one);
        if other.as_ref().is_some_and(|other| !other.contains(&one)) {
            marks.push((from..text.len(), HighlightStyle { background_color: Some(mark), ..HighlightStyle::default() }));
        }
        text.push(' ');
    }
    let text = text.trim_end().to_string();
    (text.clone(), div().min_w_0().child(StyledText::new(SharedString::from(text)).with_highlights(marks)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(id: &str, words: &str) -> Step {
        Step { id: id.into(), label: id.to_uppercase(), glyph: None, ends: String::new(), ends_any: Vec::new(), passed_over: None, words: words.into() }
    }

    #[test]
    fn unchanged_words_are_held_and_words_that_differ_are_drawn_whole() {
        let before = vec![step("a", "kept"), step("b", "was"), step("r", "gone")];
        let after = vec![step("a", "kept"), step("b", "is"), step("c", "new"), step("e", "")];
        let held: BTreeSet<String> = passages("row", &Phase::Proposed { before, after, asked: None, fresh: false }).into_iter().collect();
        assert_eq!(held, BTreeSet::from(["row|R|Before".to_string(), "row|C|After".to_string()]));
        assert!(passages("row", &Phase::Idle).is_empty());
    }

    fn kinds(changes: &[StepChange]) -> Vec<String> {
        changes
            .iter()
            .map(|one| match one {
                StepChange::Added { n, after } => format!("+{}@{n}", after.id),
                StepChange::Removed { n, before } => format!("-{}@{n}", before.id),
                StepChange::Changed { n, was, after, .. } => format!("~{}@{was}>{n}", after.id),
            })
            .collect()
    }

    #[test]
    fn a_split_step_is_removed_where_it_stood_and_the_new_ones_follow() {
        let before = [step("propose", "a"), step("walk", "b"), step("refine", "c"), step("deliver", "d")];
        let after = [step("propose", "a"), step("walk", "b"), step("wire", "e"), step("tests", "f"), step("deliver", "d")];
        let (changes, unchanged) = step_changes(&before, &after);
        assert_eq!(kinds(&changes), ["-refine@3", "+wire@3", "+tests@4"]);
        assert_eq!(unchanged, ["PROPOSE", "WALK", "DELIVER"]);
    }

    #[test]
    fn a_step_that_moves_is_changed_and_the_ones_it_passed_are_not() {
        let before = [step("a", "1"), step("b", "2"), step("c", "3")];
        let after = [step("b", "2"), step("c", "3"), step("a", "1")];
        assert_eq!(kinds(&step_changes(&before, &after).0), ["~a@1>3"]);
        let reworded = [step("a", "1"), step("b", "two"), step("c", "3")];
        assert_eq!(kinds(&step_changes(&before, &reworded).0), ["~b@2>2"]);
    }

    #[test]
    fn the_oldest_asked_change_sets_the_phase() {
        let change = |state: ChangeState| WorkflowChange {
            id: 1,
            workflow_id: WorkflowId::carried("feature"),
            source: "repository".into(),
            project: None,
            step_id: None,
            origin: ChangeOrigin::Asked { text: "Split Refine".into() },
            before: r#"{"workflow_id":"feature","steps":[{"id":"refine","label":"Refine"}]}"#.into(),
            after: Some(r#"{"workflow_id":"feature","steps":[{"id":"wire","label":"Wire"}]}"#.into()),
            state,
            reason: None,
            missing: vec!["A step ends on one thing".into()],
            before_steps: None,
            after_steps: None,
            after_name: None,
            after_for_requests: None,
            notes: None,
            drafted: false,
            at: ipc::Instant::carried("2026-11-12T09:00:00Z"),
        };
        assert_eq!(phase_of(&[], &[]), Phase::Idle);
        assert_eq!(phase_of(&[change(ChangeState::Drafting)], &[]), Phase::Working { asked: Some("Split Refine".into()) });
        assert!(matches!(phase_of(&[change(ChangeState::Cannot)], &[]), Phase::Cannot { missing, .. } if missing.len() == 1));
        let Phase::Proposed { before, after, fresh, .. } = phase_of(&[change(ChangeState::Ready)], &[step("refine", "kept")]) else { panic!("proposed") };
        assert_eq!((before[0].words.as_str(), after[0].label.as_str(), fresh), ("", "Wire", false));
    }
}
