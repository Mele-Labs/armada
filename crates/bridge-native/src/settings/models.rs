//! Settings → Models: the models the harness offers, which of them Jobs may use, and the model and effort each
//! kind of run starts on. A port of Classic's Kit › Models (`packages/components/src/compositions/KitModels/`,
//! `apps/desktop/src/renderer/src/kit-models-wire.ts`): read through `GET /models`, saved into settings.json.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    canvas, div, prelude::*, px, svg, AnyElement, App, Bounds, Context, Entity, FocusHandle, Focusable, FontWeight, Keystroke, MouseButton, MouseDownEvent,
    MouseMoveEvent, Pixels, Role, SharedString, Subscription, Task, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{ModelChoices, ModelsRead, SettingValue, SettingsList};

use super::overlay::{Picking, SettingsOverlay};
use super::view::{bone, bounds_of, mono, ring_colour, ringed, small, switch_track, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// Fleet's first effort: passing none, the harness picking.
pub const HARNESS_DEFAULT: &str = "harness default";

/// A choice the page makes, by the setting it writes (`CHOICE_KEY`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Jobs,
    Drones,
    Reviews,
    Effort,
    Judge,
    JudgeEffort,
    Proposer,
    ProposerEffort,
    Retro,
    SecondOpinion,
    Downshift,
}

impl Slot {
    pub fn key(self) -> &'static str {
        match self {
            Slot::Jobs => "models.session",
            Slot::Drones => "models.default",
            Slot::Reviews => "models.review",
            Slot::Effort => "effort.default",
            Slot::Judge => "models.judge",
            Slot::JudgeEffort => "effort.judge",
            Slot::Proposer => "models.proposer",
            Slot::ProposerEffort => "effort.proposer",
            Slot::Retro => "models.retro",
            Slot::SecondOpinion => "models.secondOpinion",
            Slot::Downshift => "plan.downshiftTo",
        }
    }

    fn effort(self) -> bool {
        matches!(self, Slot::Effort | Slot::JudgeEffort | Slot::ProposerEffort)
    }
}

/// One block per run: its name, its model, then its effort where it has one (`RUNS`).
pub struct Run {
    pub label: &'static str,
    pub model: Slot,
    pub effort: Option<Slot>,
}

pub const RUNS: [Run; 8] = [
    Run { label: "Jobs", model: Slot::Jobs, effort: None },
    Run { label: "Drones", model: Slot::Drones, effort: Some(Slot::Effort) },
    Run { label: "Reviews", model: Slot::Reviews, effort: None },
    Run { label: "Judge", model: Slot::Judge, effort: Some(Slot::JudgeEffort) },
    Run { label: "Proposer", model: Slot::Proposer, effort: Some(Slot::ProposerEffort) },
    Run { label: "Retro", model: Slot::Retro, effort: None },
    Run { label: "Second opinion", model: Slot::SecondOpinion, effort: None },
    Run { label: "Step down to", model: Slot::Downshift, effort: None },
];

/// Every setting that names a model, and whether it falls to the first model still on or the last when the one
/// it names is switched off (`FALLS_TO`). Fleet refuses a save that leaves one naming an off model.
const FALLS_TO: [(&str, bool); 10] = [
    ("models.session", true),
    ("models.default", true),
    ("models.review", true),
    ("models.retro", true),
    ("models.operator", true),
    ("models.secondOpinion", true),
    ("plan.downshiftTo", true),
    ("models.judge", false),
    ("models.proposer", false),
    ("models.account", false),
];

/// One model the harness offers, as the page draws it (`KitModel`).
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub name: String,
    /// The family without its version: `Opus`.
    pub display: String,
    /// The version a Job runs: `opus-5-5`.
    pub resolved: String,
    pub says: String,
    pub efforts: Vec<String>,
    pub on: bool,
}

/// A Kit column: a raised panel that scrolls itself (`.armada-kit-browser__list`).
pub(super) fn panel(look: &Look, id: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_3))
        .min_w_0()
        .min_h_0()
        .h_full()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_RAISED))
        .overflow_y_scroll()
}

/// What the page draws (`KitModelsRead`).
#[derive(Clone, Debug, PartialEq)]
pub enum Read {
    Loading,
    NotRead(String),
    Read(Vec<Model>),
}

/// `opus-5-5` is `5.5` (`versionOf`).
pub fn version_of(resolved: &str) -> String {
    let bytes = resolved.as_bytes();
    let mut at = resolved.len();
    // Back over each `-digits` group at the end.
    loop {
        let digits = bytes[..at].iter().rev().take_while(|b| b.is_ascii_digit()).count();
        if digits == 0 || at == digits || bytes[at - digits - 1] != b'-' {
            break;
        }
        at -= digits + 1;
    }
    resolved[at..].trim_start_matches('-').replace('-', ".")
}

/// `Opus 5.5` is `Opus`: the page draws the version beside it (`familyOf`).
fn family_of(offered: &ipc::OfferedModel) -> String {
    let display = offered.display.trim_end();
    let cut = display.rsplit_once(char::is_whitespace).filter(|(_, last)| !last.is_empty() && last.split('.').all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit())));
    let family = cut.map_or(display, |(head, _)| head.trim_end());
    if !family.is_empty() {
        return family.to_string();
    }
    let mut chars = offered.name.chars();
    chars.next().map(|first| first.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

fn text_of(list: Option<&SettingsList>, key: &str) -> Option<String> {
    match &list?.settings.iter().find(|one| one.key == key)?.value {
        SettingValue::Text(text) => Some(text.clone()),
        _ => None,
    }
}

/// The models Jobs may use as settings.json says it: `models.roster` holds it, or holds nothing.
fn roster_of(list: Option<&SettingsList>) -> Option<Vec<String>> {
    match &list?.settings.iter().find(|one| one.key == "models.roster")?.value {
        SettingValue::List(items) => Some(items.iter().filter_map(|item| if let SettingValue::Text(text) = item { Some(text.clone()) } else { None }).collect()),
        _ => None,
    }
}

/// The offered models as the page draws them (`readKitModels`). Whether each is on follows settings.json once it
/// is read, so a switch reads as switched as soon as the save is answered.
pub fn models_of(held: Option<&Result<ModelChoices, String>>, list: Option<&SettingsList>) -> Read {
    let held = match held {
        None => return Read::Loading,
        Some(Err(_)) => return Read::NotRead("Fleet did not answer".into()),
        Some(Ok(held)) => held,
    };
    if let ModelsRead::NotRead { why } = &held.read {
        return Read::NotRead(if why.is_empty() { "Fleet does not list models yet".into() } else { why.clone() });
    }
    let roster = roster_of(list);
    Read::Read(
        held.offered
            .iter()
            .map(|one| Model {
                name: one.name.clone(),
                display: family_of(one),
                resolved: one.version.clone(),
                says: one.description.clone(),
                efforts: one.efforts.clone(),
                on: roster.as_ref().map_or(one.on, |roster| roster.is_empty() || roster.contains(&one.name)),
            })
            .collect(),
    )
}

/// The model or effort a slot holds now: settings.json's word, else the model a run that names none is given,
/// else the harness picking.
pub fn choice_of(slot: Slot, held: Option<&ModelChoices>, list: Option<&SettingsList>) -> String {
    if let Some(word) = text_of(list, slot.key()).filter(|word| !word.is_empty()) {
        return word;
    }
    match (slot, held) {
        (Slot::Jobs, Some(held)) => held.session.clone(),
        (slot, _) if slot.effort() => HARNESS_DEFAULT.into(),
        (_, Some(held)) => held.default.clone(),
        (_, None) => String::new(),
    }
}

/// The save for switching one model on or off: the roster, and every setting that names the model switched off
/// moved to one still on. `None` where it would leave none on (`saveForSwitch`).
pub fn save_for_switch(models: &[Model], list: Option<&SettingsList>, name: &str, on: bool) -> Option<BTreeMap<String, Option<SettingValue>>> {
    let roster: Vec<String> = models.iter().filter(|one| if one.name == name { on } else { one.on }).map(|one| one.name.clone()).collect();
    let (first, last) = (roster.first()?.clone(), roster.last()?.clone());
    let mut changes: BTreeMap<String, Option<SettingValue>> = BTreeMap::new();
    changes.insert("models.roster".into(), Some(SettingValue::List(roster.iter().cloned().map(SettingValue::Text).collect())));
    for (key, to_first) in FALLS_TO {
        if let Some(named) = text_of(list, key) {
            if !roster.contains(&named) {
                changes.insert(key.into(), Some(SettingValue::Text(if to_first { first.clone() } else { last.clone() })));
            }
        }
    }
    Some(changes)
}

/// The efforts a slider steps through: the harness picking, those the models on take, and any in force.
pub fn efforts_of(models: &[Model], in_force: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut steps = vec![HARNESS_DEFAULT.to_string()];
    for one in models.iter().filter(|one| one.on).flat_map(|one| one.efforts.iter().cloned()).chain(in_force) {
        if !steps.contains(&one) {
            steps.push(one);
        }
    }
    steps
}

/// An effort as the slider says it.
pub fn effort_said(word: &str) -> String {
    if word == HARNESS_DEFAULT { "default".into() } else { word.into() }
}

/// The page's own state: what Fleet answered, the row picked, the filter, and each control's keys.
pub struct ModelsPage {
    pub held: Option<Result<ModelChoices, String>>,
    asked: bool,
    pub picked: Option<String>,
    pub filter: Entity<InputState>,
    /// Each model's row and switch.
    rows: BTreeMap<String, [FocusHandle; 2]>,
    /// Each run's select and slider, in `RUNS` order.
    selects: Vec<FocusHandle>,
    sliders: Vec<FocusHandle>,
    /// What a control was moved to, drawn until settings.json answers with it.
    want: BTreeMap<String, String>,
    /// The one change out, by key and word.
    sent: Option<(String, String)>,
    tracks: Rc<RefCell<BTreeMap<usize, Bounds<Pixels>>>>,
    _read: Option<Task<()>>,
    _filter: Subscription,
}

impl ModelsPage {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsOverlay>) -> ModelsPage {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter"));
        let sub = cx.subscribe_in(&filter, window, |_, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        ModelsPage {
            held: None,
            asked: false,
            picked: None,
            filter,
            rows: BTreeMap::new(),
            selects: RUNS.iter().map(|_| cx.focus_handle().tab_stop(true)).collect(),
            sliders: RUNS.iter().map(|_| cx.focus_handle().tab_stop(true)).collect(),
            want: BTreeMap::new(),
            sent: None,
            tracks: Rc::default(),
            _read: None,
            _filter: sub,
        }
    }

    /// Settings opened again: the models are asked again the next time the page is drawn.
    pub fn reopen(&mut self) {
        self.asked = false;
    }

    pub fn filter_focused(&self, window: &Window, cx: &App) -> bool {
        self.filter.read(cx).focus_handle(cx).is_focused(window)
    }
}

impl SettingsOverlay {
    fn held_models(&self) -> Option<&ModelChoices> {
        self.models.held.as_ref().and_then(|held| held.as_ref().ok())
    }

    /// The models as drawn now.
    pub(super) fn kit_models(&self) -> Read {
        models_of(self.models.held.as_ref(), self.list.as_ref())
    }

    /// A slot's word as drawn: what it was moved to, else what settings.json holds.
    pub(super) fn slot_word(&self, slot: Slot) -> String {
        self.models.want.get(slot.key()).cloned().unwrap_or_else(|| choice_of(slot, self.held_models(), self.list.as_ref()))
    }

    pub(super) fn effort_steps(&self, models: &[Model]) -> Vec<String> {
        efforts_of(models, RUNS.iter().filter_map(|run| run.effort).map(|slot| self.slot_word(slot)))
    }

    /// Ask Fleet for the models once each time Settings opens, while the page is drawn.
    pub(super) fn read_models(&mut self, cx: &mut Context<Self>) {
        if self.models.asked {
            return;
        }
        self.models.asked = true;
        let read = self.wire.models(cx);
        self.models._read = Some(cx.spawn(async move |this, cx| {
            let answer = read.await;
            let _ = this.update(cx, |this, cx| {
                // A read that failed keeps what was read before, where there was one.
                match answer {
                    Ok(held) => this.models.held = Some(Ok(held)),
                    Err(why) if !matches!(this.models.held, Some(Ok(_))) => this.models.held = Some(Err(why.said)),
                    Err(_) => {}
                }
                cx.notify();
            });
        }));
    }

    /// Keys for each model's row and switch, made the first time it is listed.
    pub(super) fn model_keys(&mut self, cx: &mut Context<Self>) {
        let Read::Read(models) = self.kit_models() else { return };
        for one in models {
            self.models.rows.entry(one.name).or_insert_with(|| [cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true)]);
        }
    }

    /// The rows the filter keeps, and the one the detail draws.
    fn filtered(&self, cx: &App) -> (Vec<Model>, Option<Model>) {
        let Read::Read(models) = self.kit_models() else { return (Vec::new(), None) };
        let query = self.models.filter.read(cx).value().to_string();
        let rows: Vec<Model> = models.into_iter().filter(|one| super::entries::holds(&one.display, &query)).collect();
        let row = rows.iter().find(|one| Some(&one.name) == self.models.picked.as_ref()).or_else(|| rows.first()).cloned();
        (rows, row)
    }

    /// The page's stops in the order Tab reaches them: the filter, the row picked and each switch that can move,
    /// then each run's model and effort.
    pub(super) fn model_stops(&self, cx: &App) -> Vec<FocusHandle> {
        let mut stops = vec![self.models.filter.read(cx).focus_handle(cx)];
        let (rows, row) = self.filtered(cx);
        let on = rows.iter().filter(|one| one.on).count();
        let all_on = match self.kit_models() {
            Read::Read(models) => models.iter().filter(|one| one.on).count(),
            _ => on,
        };
        for one in &rows {
            let Some([row_key, switch]) = self.models.rows.get(&one.name) else { continue };
            if row.as_ref().is_some_and(|row| row.name == one.name) {
                stops.push(row_key.clone());
            }
            if !(one.on && all_on == 1) {
                stops.push(switch.clone());
            }
        }
        if let Read::Read(_) = self.kit_models() {
            for (at, run) in RUNS.iter().enumerate() {
                stops.push(self.models.selects[at].clone());
                if run.effort.is_some() {
                    stops.push(self.models.sliders[at].clone());
                }
            }
        }
        stops
    }

    /// Move a slot to a word: drawn at once, sent now or after the change already out.
    pub fn choose_slot(&mut self, slot: Slot, word: String, window: &mut Window, cx: &mut Context<Self>) {
        self.models.want.insert(slot.key().to_string(), word);
        self.flush_models(window, cx);
        cx.notify();
    }

    /// Send the first change that differs from settings.json, while nothing is out. One that already matches is dropped.
    fn flush_models(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving.is_some() {
            return;
        }
        let list = self.list.clone();
        self.models.want.retain(|key, word| text_of(list.as_ref(), key).as_deref() != Some(word.as_str()));
        let Some((key, word)) = self.models.want.iter().next().map(|(key, word)| (key.clone(), word.clone())) else { return };
        self.models.sent = Some((key.clone(), word.clone()));
        self.save(key, Some(SettingValue::Text(word)), window, cx);
    }

    /// A save answered: the change it carried is no longer wanted where it was the last one made, and is dropped
    /// on a refusal; the next one goes.
    pub(super) fn models_saved(&mut self, taken: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((key, word)) = self.models.sent.take() {
            if !taken || self.models.want.get(&key) == Some(&word) {
                self.models.want.remove(&key);
            }
        }
        self.flush_models(window, cx);
    }

    /// Whether Jobs may use a model: the roster saved, and every setting naming a model switched off moved.
    pub fn switch_model(&mut self, name: &str, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Read::Read(models) = self.kit_models() else { return };
        let Some(changes) = save_for_switch(&models, self.list.as_ref(), name, on) else { return };
        self.save_changes("models.roster".into(), changes, window, cx);
    }

    fn pick_model(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.models.picked = Some(name.to_string());
        if let Some([row, _]) = self.models.rows.get(name) {
            window.focus(row, cx);
        }
        cx.notify();
    }

    fn open_run(&mut self, at: usize, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Read::Read(models) = self.kit_models() else { return };
        let run = &RUNS[at];
        let now = self.slot_word(run.model);
        let options = models.iter().filter(|one| one.on).map(|one| (SharedString::from(one.display.clone()), one.name == now)).collect();
        let place = bounds_of(window, cx, Role::ComboBox, run.label).map(|bounds| bounds.bottom_left()).unwrap_or_default();
        self.open_choice(Picking::Run(at), run.label.into(), options, place, from_keys, window, cx);
    }

    /// A run's model picked from its list, by its place among the models on.
    pub(super) fn chose_run(&mut self, at: usize, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Read::Read(models) = self.kit_models() else { return };
        let Some(one) = models.iter().filter(|one| one.on).nth(n) else { return };
        let slot = RUNS[at].model;
        if self.slot_word(slot) != one.name {
            self.choose_slot(slot, one.name.clone(), window, cx);
        }
    }

    fn step_effort(&mut self, at: usize, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(slot), Read::Read(models)) = (RUNS[at].effort, self.kit_models()) else { return };
        let steps = self.effort_steps(&models);
        let now = steps.iter().position(|one| *one == self.slot_word(slot)).unwrap_or(0) as isize;
        let next = (now + by).clamp(0, steps.len() as isize - 1) as usize;
        if next as isize != now {
            self.choose_slot(slot, steps[next].clone(), window, cx);
        }
    }

    fn slide_effort(&mut self, at: usize, x: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(slot), Read::Read(models)) = (RUNS[at].effort, self.kit_models()) else { return };
        let Some(bounds) = self.models.tracks.borrow().get(&at).copied() else { return };
        let steps = self.effort_steps(&models);
        let share = ((x - bounds.origin.x) / bounds.size.width).clamp(0., 1.);
        let next = ((steps.len() - 1) as f32 * share).round() as usize;
        if steps[next] != self.slot_word(slot) {
            self.choose_slot(slot, steps[next].clone(), window, cx);
        }
    }

    /// The page's own keys: Esc clears the filter, ↑ and ↓ move between rows, Enter and Space switch and open,
    /// ← and → step an effort. `true` where one was taken.
    pub(super) fn models_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        let word = key.key.as_str();
        if self.models.filter_focused(window, cx) {
            if word == "escape" && !self.models.filter.read(cx).value().is_empty() {
                self.models.filter.update(cx, |filter, cx| filter.set_value("", window, cx));
                cx.notify();
                return true;
            }
            return false;
        }
        let focused_row = self.models.rows.iter().find_map(|(name, [row, switch])| {
            if row.is_focused(window) {
                Some((name.clone(), false))
            } else if switch.is_focused(window) {
                Some((name.clone(), true))
            } else {
                None
            }
        });
        if let Some((name, on_switch)) = focused_row {
            match (word, on_switch) {
                ("up" | "down", false) => {
                    let (rows, _) = self.filtered(cx);
                    let Some(at) = rows.iter().position(|one| one.name == name) else { return true };
                    let next = if word == "up" { at.saturating_sub(1) } else { (at + 1).min(rows.len() - 1) };
                    let to = rows[next].name.clone();
                    self.pick_model(&to, window, cx);
                    return true;
                }
                ("enter" | "space", false) => {
                    self.pick_model(&name, window, cx);
                    return true;
                }
                ("enter" | "space", true) => {
                    let (rows, _) = self.filtered(cx);
                    if let Some(one) = rows.iter().find(|one| one.name == name) {
                        let on = one.on;
                        if self.saving.is_none() {
                            self.switch_model(&name, !on, window, cx);
                        }
                    }
                    return true;
                }
                _ => return false,
            }
        }
        if let Some(at) = self.models.selects.iter().position(|one| one.is_focused(window)) {
            if matches!(word, "enter" | "space" | "down") {
                self.open_run(at, true, window, cx);
                return true;
            }
            return false;
        }
        if let Some(at) = self.models.sliders.iter().position(|one| one.is_focused(window)) {
            match word {
                "left" | "down" => self.step_effort(at, -1, window, cx),
                "right" | "up" => self.step_effort(at, 1, window, cx),
                "home" => self.step_effort(at, -99, window, cx),
                "end" => self.step_effort(at, 99, window, cx),
                _ => return false,
            }
            return true;
        }
        false
    }

    // ---- drawn --------------------------------------------------------------------------------


    /// The list column and the detail beside it, as Classic's Kit browser lays them out.
    pub(super) fn render_models(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let read = self.kit_models();
        let (rows, row) = self.filtered(cx);
        let off = !self.live || self.saving.is_some();
        let filter = labelled(
            div().id("models-filter").flex_none().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_SM)).child(Input::new(&self.models.filter)),
            Role::SearchInput,
            "Filter Models",
        );
        let mut list = panel(look, "models-list").flex_none().w(look.length(token::W_GUIDE_LIST)).p(look.length(token::SPACE_3)).child(filter);
        match &read {
            Read::Loading => {
                for n in 0..4 {
                    list = list.child(labelled(
                        div().id(SharedString::from(format!("models-bone-{n}"))).flex().items_center().justify_between().h(look.length(token::H_CONTROL)).px(look.length(token::SPACE_3)).child(bone(look, px(96.), look.length(token::LEADING_XS))).child(bone(look, px(48.), look.length(token::LEADING_XS))),
                        Role::Image,
                        "Loading",
                    ));
                }
            }
            Read::NotRead(why) => {
                list = list.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .px(look.length(token::SPACE_3))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Not read"))
                        .child(SharedString::from(why.clone())),
                );
            }
            Read::Read(models) => {
                let all_on = models.iter().filter(|one| one.on).count();
                let mut items = div().flex().flex_col();
                for (n, one) in rows.iter().enumerate() {
                    items = items.child(self.render_model_row(one, n > 0, row.as_ref().is_some_and(|row| row.name == one.name), one.on && all_on == 1, off, look, window, cx));
                }
                list = list.child(labelled(items.id("models-items"), Role::List, "Models"));
                if let Some(said) = self.refused.get("models.roster") {
                    list = list.child(labelled(super::view::refusal(look, said.clone()).id("models-refused"), Role::Alert, said.clone()));
                }
                list = list.child(self.render_runs(models, off, look, window, cx));
            }
        }
        let detail = panel(look, "models-detail").flex_1().p(look.length(token::SPACE_4)).children(row.map(|row| self.render_model_detail(&row, look)));
        div()
            .id("settings-page")
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .gap(look.length(token::GAP_PANEL))
            .child(labelled(list, Role::Region, "Models"))
            .child(labelled(detail, Role::Region, "Detail"))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_model_row(&self, one: &Model, ruled: bool, current: bool, last_on: bool, off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some([row_key, switch_key]) = self.models.rows.get(&one.name).cloned() else { return div().into_any_element() };
        let name = one.name.clone();
        let version = version_of(&one.resolved);
        let button = ringed(
            look,
            div()
                .id(SharedString::from(format!("model-{}", one.name)))
                .track_focus(&row_key)
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_3))
                .min_h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_DEFAULT))
                .cursor_pointer()
                .map(|row| if current { row.bg(look.color(token::SURFACE_ROW_SELECTED)) } else { row.hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.1))) })
                .on_click(cx.listener(move |this, _, window, cx| this.pick_model(&name, window, cx)))
                .child(div().min_w_0().child(SharedString::from(one.display.clone())))
                .child(
                    div()
                        .id(SharedString::from(format!("model-version-{}", one.name)))
                        .flex_none()
                        .text_color(look.color(token::FG_SUBTLE))
                        .tooltip(tip("Version"))
                        .child(SharedString::from(version)),
                ),
            &row_key,
            window,
        );
        let blocked = off || last_on;
        let (press, on) = (one.name.clone(), one.on);
        let switch = ringed(
            look,
            div()
                .id(SharedString::from(format!("model-switch-{}", one.name)))
                .track_focus(&switch_key)
                .flex_none()
                .rounded_full()
                .tooltip(tip("Jobs may use it"))
                .child(switch_track(look, one.on, blocked))
                .when(!blocked, |switch| switch.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.switch_model(&press, !on, window, cx)))),
            &switch_key,
            window,
        );
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .min_w_0()
            .when(ruled, |item| item.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::FG_DEFAULT).opacity(0.1)))
            .child(labelled(button, Role::Button, one.display.clone()))
            .child(labelled(switch, Role::Switch, format!("{} may be used", one.display)))
            .into_any_element()
    }

    fn render_runs(&self, models: &[Model], off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let steps = self.effort_steps(models);
        let caption = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .pt(look.length(token::SPACE_2))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_MUTED))
            .child("Default models")
            .child(div().flex_1().h(look.length(token::BORDER_WIDTH)).bg(look.color(token::BORDER_SUBTLE)));
        let mut section = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(labelled(caption.id("models-caption"), Role::Heading, "Default models"));
        for (at, run) in RUNS.iter().enumerate() {
            let mut block = div()
                .id(SharedString::from(format!("run-{at}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(run.label))
                .child(self.render_select(at, models, off, look, window, cx));
            if let Some(slot) = run.effort {
                block = block.child(self.render_effort(at, slot, &steps, off, look, window, cx));
            }
            for slot in std::iter::once(run.model).chain(run.effort) {
                if let Some(said) = self.refused.get(slot.key()) {
                    block = block.child(labelled(super::view::refusal(look, said.clone()).id(SharedString::from(format!("run-refused-{}", slot.key()))), Role::Alert, said.clone()));
                }
            }
            section = section.child(labelled(block, Role::Group, run.label));
        }
        labelled(section.id("models-choose"), Role::Region, "Default models").into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_select(&self, at: usize, models: &[Model], off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let run = &RUNS[at];
        let handle = self.models.selects[at].clone();
        let word = self.slot_word(run.model);
        let said = models.iter().find(|one| one.name == word).map_or(word.clone(), |one| one.display.clone());
        let trigger = ringed(
            look,
            div()
                .id(SharedString::from(format!("run-model-{at}")))
                .track_focus(&handle)
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) })
                .child(div().min_w_0().truncate().child(SharedString::from(said)))
                .child(svg().path(icon_path("chevron-down")).flex_none().size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
                .when(!off, |trigger| trigger.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.open_run(at, false, window, cx)))),
            &handle,
            window,
        );
        labelled(trigger, Role::ComboBox, run.label).into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_effort(&self, at: usize, slot: Slot, steps: &[String], off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let handle = self.models.sliders[at].clone();
        let word = self.slot_word(slot);
        let place = steps.iter().position(|one| *one == word).unwrap_or(0);
        let share = if steps.len() > 1 { place as f32 / (steps.len() - 1) as f32 } else { 0. };
        let rail = look.length(token::SPACE_1);
        let knob = look.length(token::SPACE_4);
        let tall = look.length(token::H_BADGE);
        let tracks = self.models.tracks.clone();
        let track = div()
            .id(SharedString::from(format!("run-effort-{at}")))
            .track_focus(&handle)
            .relative()
            .flex_1()
            .min_w_0()
            .h(tall)
            .when(!off, |track| track.cursor_pointer())
            .child(canvas(move |bounds, _, _| {
                tracks.borrow_mut().insert(at, bounds);
            }, |_, _, _, _| {}).absolute().left_0().top_0().size_full())
            .child(div().absolute().left_0().right_0().top(tall / 2. - rail / 2.).h(rail).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::BG_HOVER)))
            .child(
                div()
                    .absolute()
                    .top(tall / 2. - knob / 2.)
                    .left(gpui::relative(share))
                    .ml(-knob * share)
                    .size(knob)
                    .rounded_full()
                    .bg(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) })
                    .when(handle.is_focused(window), |knob| knob.border(look.length(token::FOCUS_RING_WIDTH)).border_color(ring_colour(look))),
            )
            .when(!off, |track| {
                track
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        let handle = this.models.sliders[at].clone();
                        window.focus(&handle, cx);
                        this.slide_effort(at, event.position.x, window, cx);
                    }))
                    .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, window, cx| {
                        if event.pressed_button == Some(MouseButton::Left) {
                            this.slide_effort(at, event.position.x, window, cx);
                        }
                    }))
            });
        let label = format!("{} effort", RUNS[at].label);
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_h(look.length(token::H_CONTROL))
            .child(labelled(track, Role::Slider, label))
            .child(
                div()
                    .flex_none()
                    .w(look.length(token::SPACE_12))
                    .flex()
                    .justify_end()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(effort_said(&word))),
            )
            .into_any_element()
    }

    fn render_model_detail(&self, row: &Model, look: &Look) -> AnyElement {
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_w_0()
            .min_h(look.length(token::SPACE_8) + look.length(token::SPACE_1))
            .mx(-look.length(token::SPACE_4))
            .mt(-look.length(token::SPACE_4))
            .px(look.length(token::SPACE_4))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(
                div().id("model-title").min_w_0().truncate().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(row.display.clone())),
                Role::Heading,
                row.display.clone(),
            ))
            .child(div().id("model-version").flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).tooltip(tip("Version")).child(SharedString::from(version_of(&row.resolved))));
        let efforts = div().id("model-efforts").flex().flex_wrap().gap(look.length(token::SPACE_3)).children(row.efforts.iter().map(|level| small(look, level.clone(), look.color(token::FG_SUBTLE))));
        labelled(
            div()
                .id("model-detail")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .min_w_0()
                .child(head)
                .child(div().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(row.resolved.clone())))
                .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(row.says.clone())))
                .child(labelled(efforts, Role::List, "Effort levels")),
            Role::Article,
            row.display.clone(),
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held() -> ModelChoices {
        super::super::mock::kit_models()
    }

    fn list() -> SettingsList {
        super::super::mock::MockFleet::new().list
    }

    #[test]
    fn a_version_is_read_from_the_tail_of_the_id() {
        assert_eq!(version_of("opus-5-5"), "5.5");
        assert_eq!(version_of("fable-5-1"), "5.1");
        assert_eq!(version_of("5.5"), "");
        assert_eq!(version_of("claude-opus"), "");
    }

    #[test]
    fn the_family_is_the_display_without_its_version() {
        let models = match models_of(Some(&Ok(held())), Some(&list())) {
            Read::Read(models) => models,
            other => panic!("read: {other:?}"),
        };
        let names: Vec<(&str, bool)> = models.iter().map(|one| (one.display.as_str(), one.on)).collect();
        assert_eq!(names, [("Opus", true), ("Fable", true), ("Sonnet", true), ("Haiku", false)]);
    }

    #[test]
    fn each_run_reads_its_own_key_else_the_default_model_else_the_harness() {
        let (held, list) = (held(), list());
        assert_eq!(choice_of(Slot::Jobs, Some(&held), Some(&list)), "opus");
        assert_eq!(choice_of(Slot::Drones, Some(&held), Some(&list)), "sonnet");
        assert_eq!(choice_of(Slot::Effort, Some(&held), Some(&list)), "high");
        assert_eq!(choice_of(Slot::JudgeEffort, Some(&held), Some(&list)), HARNESS_DEFAULT);
        let mut bare = list.clone();
        bare.settings.retain(|one| one.key != "models.review" && one.key != "effort.proposer");
        assert_eq!(choice_of(Slot::Reviews, Some(&held), Some(&bare)), held.default);
        assert_eq!(choice_of(Slot::ProposerEffort, Some(&held), Some(&bare)), HARNESS_DEFAULT);
    }

    #[test]
    fn switching_a_model_off_moves_every_setting_that_names_it() {
        let list = list();
        let Read::Read(models) = models_of(Some(&Ok(held())), Some(&list)) else { panic!("read") };
        let changes = save_for_switch(&models, Some(&list), "sonnet", false).expect("two stay on");
        let text = |key: &str| changes.get(key).cloned().flatten();
        assert_eq!(text("models.roster"), Some(SettingValue::List(vec![SettingValue::Text("opus".into()), SettingValue::Text("fable".into())])));
        assert_eq!(text("models.default"), Some(SettingValue::Text("opus".into())), "the strong tier falls to the first on");
        assert_eq!(text("models.judge"), Some(SettingValue::Text("fable".into())), "the cheap tier falls to the last on");
        assert_eq!(text("models.session"), None, "a setting naming a model still on is left");
    }

    #[test]
    fn the_last_model_on_cannot_be_switched_off() {
        let one = Model { name: "opus".into(), display: "Opus".into(), resolved: String::new(), says: String::new(), efforts: Vec::new(), on: true };
        assert!(save_for_switch(&[one], None, "opus", false).is_none());
    }

    #[test]
    fn the_effort_steps_lead_with_the_harness_and_keep_any_in_force() {
        let one = Model { name: "opus".into(), display: "Opus".into(), resolved: String::new(), says: String::new(), efforts: vec!["low".into(), "high".into()], on: true };
        assert_eq!(efforts_of(&[one], ["high".to_string(), "odd".to_string()]), [HARNESS_DEFAULT, "low", "high", "odd"]);
        assert_eq!(effort_said(HARNESS_DEFAULT), "default");
    }
}
