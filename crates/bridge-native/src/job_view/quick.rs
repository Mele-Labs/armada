//! The quick bar on the message box's top edge (`QuickBar.tsx`, section 4.7): the answers Fleet
//! offers to the agent's last message, then the lines the person saved, then the ones Fleet learnt.
//!
//! The saved lines live in the `thread.look` setting. Every act on them is a plain write of that
//! setting, drawn as done the same frame and put back with Fleet's reason if it refuses. A learnt
//! line takes the same menu, as Electron's `thread-look.ts` has it: Pin keeps it as his own, pinned;
//! Delete forgets it; Edit forgets its words and keeps his in their place.

use std::collections::{BTreeMap, HashMap};

use gpui::{
    div, point, prelude::*, svg, Anchor, AnyElement, ClickEvent, Context, Entity, Focusable, KeyDownEvent, MouseButton,
    Role, SharedString, Subscription, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{SaveSettings, SessionQuick, SettingValue, SettingsList, ThreadItem};

use super::head::{below_left, drawn, icon_button, tip};
use super::wire::Done;
use super::JobView;
use crate::assets::icon_path;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::look::{token, Look};

const KEY: &str = "thread.look";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum TextSize {
    Xs,
    Sm,
    Base,
    Lg,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Spacing {
    Compact,
    Default,
    Roomy,
}

/// The parts of the look the thread's rows are drawn by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Thread {
    pub show_times: bool,
    pub text_size: TextSize,
    pub spacing: Spacing,
}

/// What the Aa panel shows of the look.
#[derive(Clone, Debug)]
pub(super) struct Panel {
    pub agent_name: String,
    pub show_agent_name: bool,
    pub thread: Thread,
    pub learn: bool,
    pub voice: String,
    pub quick_off: bool,
    pub responses: Vec<Response>,
}

/// A quick response: one press on a line of an agent's message (`responses.ts`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Response {
    pub id: String,
    pub emoji: String,
    pub name: String,
    /// `ask` puts the quote in the box instead of sending it.
    pub ask: bool,
}

/// How many responses the bar holds.
pub(super) const MAX_RESPONSES: usize = 6;

impl Response {
    pub fn defaults() -> Vec<Response> {
        [("pick", "\u{2705}", "Pick this", false), ("yes", "\u{1f44d}", "Yes to this", false), ("no", "\u{1f44e}", "No to this", false), ("ask", "\u{2753}", "Ask about this", true)]
            .into_iter()
            .map(|(id, emoji, name, ask)| Response { id: id.into(), emoji: emoji.into(), name: name.into(), ask })
            .collect()
    }
}

/// About forty common reactions, in rows of eight, each named for its tooltip (`REACTIONS`).
pub(super) const REACTIONS: [(&str, &str); 40] = [
    ("\u{2705}", "Check"), ("\u{1f44d}", "Thumbs up"), ("\u{1f44e}", "Thumbs down"), ("\u{2753}", "Question"),
    ("\u{274c}", "Cross"), ("\u{1f440}", "Eyes"), ("\u{1f389}", "Party"), ("\u{1f525}", "Fire"),
    ("\u{1f4af}", "Hundred"), ("\u{1f64f}", "Thanks"), ("\u{1f44f}", "Clap"), ("\u{1f64c}", "Raised hands"),
    ("\u{1f4a1}", "Idea"), ("\u{1f914}", "Thinking"), ("\u{1f604}", "Smile"), ("\u{1f615}", "Confused"),
    ("\u{2764}\u{fe0f}", "Heart"), ("\u{2b50}", "Star"), ("\u{1f680}", "Rocket"), ("\u{1f41b}", "Bug"),
    ("\u{26a0}\u{fe0f}", "Warning"), ("\u{1f6d1}", "Stop"), ("\u{23f3}", "Waiting"), ("\u{1f501}", "Repeat"),
    ("\u{1f44c}", "OK hand"), ("\u{1f91d}", "Handshake"), ("\u{1f4aa}", "Strong"), ("\u{1f9ea}", "Test"),
    ("\u{1f4cc}", "Pin"), ("\u{1f4dd}", "Note"), ("\u{1f50d}", "Search"), ("\u{1f9f9}", "Sweep"),
    ("\u{1f197}", "OK"), ("\u{2795}", "Plus"), ("\u{2796}", "Minus"), ("\u{23ed}\u{fe0f}", "Skip"),
    ("\u{1f3c1}", "Finish"), ("\u{1f3af}", "Target"), ("\u{1f937}", "Shrug"), ("\u{1f605}", "Relief"),
];

/// The first emoji or character a field holds, so a paste of several leaves one: a pictograph with
/// its joiners, modifiers and variation selectors, or a flag.
pub(super) fn first_one(text: &str) -> String {
    let mut chars = text.trim().chars().peekable();
    let Some(first) = chars.next() else { return String::new() };
    let mut one = String::from(first);
    let flag = |c: char| ('\u{1f1e6}'..='\u{1f1ff}').contains(&c);
    if flag(first) {
        if let Some(second) = chars.next().filter(|c| flag(*c)) {
            one.push(second);
        }
        return one;
    }
    while let Some(next) = chars.peek().copied() {
        if matches!(next, '\u{fe0f}' | '\u{20e3}' | '\u{1f3fb}'..='\u{1f3ff}') {
            one.push(next);
            chars.next();
        } else if next == '\u{200d}' {
            one.push(next);
            chars.next();
            if let Some(joined) = chars.next() {
                one.push(joined);
            }
        } else {
            break;
        }
    }
    one
}

/// Whether `text` is exactly one emoji: something past ASCII, and no more than `first_one` leaves.
pub(super) fn is_emoji(text: &str) -> bool {
    !text.is_empty() && first_one(text) == text && text.chars().next().is_some_and(|c| !c.is_ascii() && !c.is_alphanumeric())
}

/// What he does to the responses, from the settings row.
pub(super) enum ResponseAct {
    Add { emoji: String, name: String },
    Edit { id: String, emoji: String, name: String },
    Remove(String),
    Move { id: String, by: isize },
}

/// The list after one act: added responses send, at most `MAX_RESPONSES`, and never fewer than one.
pub(super) fn act_on_responses(mut list: Vec<Response>, act: ResponseAct) -> Vec<Response> {
    match act {
        ResponseAct::Add { emoji, name } => {
            if list.len() < MAX_RESPONSES {
                list.push(Response { id: format!("response:{}", chrono::Utc::now().timestamp_millis()), emoji, name, ask: false });
            }
        }
        ResponseAct::Edit { id, emoji, name } => {
            if let Some(one) = list.iter_mut().find(|one| one.id == id) {
                one.emoji = emoji;
                one.name = name;
            }
        }
        ResponseAct::Remove(id) => {
            if list.len() > 1 {
                list.retain(|one| one.id != id);
            }
        }
        ResponseAct::Move { id, by } => {
            if let Some(from) = list.iter().position(|one| one.id == id) {
                let to = from as isize + by;
                if (0..list.len() as isize).contains(&to) {
                    let one = list.remove(from);
                    list.insert(to as usize, one);
                }
            }
        }
    }
    list
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Kind {
    Asked,
    Saved,
    Learnt,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Line {
    pub id: Option<String>,
    pub text: String,
    pub kind: Kind,
    pub pinned: bool,
}

struct Edit {
    id: String,
    input: Entity<InputState>,
    /// The look before a learnt line was taken in to be edited: a cancel puts it back.
    before: Option<Option<BTreeMap<String, SettingValue>>>,
    _subs: Vec<Subscription>,
}

#[derive(Default)]
pub(super) struct Quick {
    /// `thread.look` as Fleet holds it, once read.
    pub(super) look: Option<BTreeMap<String, SettingValue>>,
    looked: bool,
    asked: HashMap<SharedString, Vec<String>>,
    /// The message each Job's answers were asked for.
    asking: HashMap<SharedString, String>,
    learnt: Vec<String>,
    edit: Option<Edit>,
    /// What a mock scenario answers for every Job.
    mock: Option<SessionQuick>,
    /// Whisper's model as a mock scenario holds it: the real state is a folder on this Mac.
    pub whisper: Option<crate::whisper::State>,
}

impl Quick {
    /// A mock scenario's look and answers, where Fleet has none to give.
    pub fn mock(look: SettingValue, quick: SessionQuick) -> Self {
        let look = match look {
            SettingValue::Map(map) => Some(map),
            _ => None,
        };
        Quick { look, looked: true, learnt: quick.learnt.clone(), mock: Some(quick), ..Quick::default() }
    }

    fn flag_or(&self, key: &str, default: bool) -> bool {
        match self.look.as_ref().and_then(|look| look.get(key)) {
            Some(SettingValue::Bool(on)) => *on,
            _ => default,
        }
    }

    pub fn text_or(&self, key: &str, default: &str) -> String {
        match self.look.as_ref().and_then(|look| look.get(key)) {
            Some(SettingValue::Text(text)) => text.clone(),
            _ => default.to_string(),
        }
    }

    /// Put one key of the look, where it is not yet written.
    pub fn put(&mut self, key: &str, value: SettingValue) {
        self.look.get_or_insert_with(BTreeMap::new).insert(key.into(), value);
    }

    /// How the thread reads (`thread-look.ts`): the three things the rows draw by.
    pub fn thread(&self) -> Thread {
        Thread {
            show_times: self.flag_or("showTimes", true),
            text_size: match self.text_or("textSize", "base").as_str() {
                "xs" => TextSize::Xs,
                "sm" => TextSize::Sm,
                "lg" => TextSize::Lg,
                _ => TextSize::Base,
            },
            spacing: match self.text_or("spacing", "default").as_str() {
                "compact" => Spacing::Compact,
                "roomy" => Spacing::Roomy,
                _ => Spacing::Default,
            },
        }
    }

    /// What the Aa panel shows.
    pub fn panel(&self) -> Panel {
        Panel {
            agent_name: self.text_or("agentName", ""),
            show_agent_name: self.flag_or("showAgentName", true),
            thread: self.thread(),
            learn: self.flag_or("learn", true),
            voice: self.text_or("voice", "apple"),
            quick_off: self.flag_or("quickOff", false),
            responses: self.responses(),
        }
    }

    /// The quick responses on a line of the agent's message, as the look keeps them or the defaults.
    pub fn responses(&self) -> Vec<Response> {
        let Some(SettingValue::List(items)) = self.look.as_ref().and_then(|look| look.get("responses")) else {
            return Response::defaults();
        };
        let read: Vec<Response> = items
            .iter()
            .filter_map(|one| {
                let SettingValue::Map(map) = one else { return None };
                let text = |key: &str| match map.get(key) {
                    Some(SettingValue::Text(text)) => Some(text.clone()),
                    _ => None,
                };
                Some(Response { id: text("id")?, emoji: text("emoji")?, name: text("name")?, ask: text("kind").as_deref() == Some("ask") })
            })
            .collect();
        if read.is_empty() { Response::defaults() } else { read }
    }

    pub fn set_responses(&mut self, list: Vec<Response>) {
        let items = list
            .into_iter()
            .map(|one| {
                let mut map = BTreeMap::new();
                map.insert("id".to_string(), SettingValue::Text(one.id));
                map.insert("emoji".to_string(), SettingValue::Text(one.emoji));
                map.insert("name".to_string(), SettingValue::Text(one.name));
                map.insert("kind".to_string(), SettingValue::Text(if one.ask { "ask" } else { "send" }.into()));
                SettingValue::Map(map)
            })
            .collect();
        self.put("responses", SettingValue::List(items));
    }

    pub fn is_mock(&self) -> bool {
        self.mock.is_some()
    }

    /// Whether the look has been read: before that the Aa panel draws shapes, and writes nothing.
    pub fn loaded(&self) -> bool {
        self.look.is_some()
    }

    /// The lines Fleet learnt that are not kept or forgotten, as the Aa panel lists them.
    pub fn learnt_lines(&self) -> Vec<Line> {
        if !self.flag_or("learn", true) {
            return Vec::new();
        }
        let (forgotten, kept) = (self.texts("forgotten"), self.kept());
        self.learnt
            .iter()
            .filter(|text| !forgotten.contains(text) && !kept.iter().any(|one| one.text == **text))
            .map(|text| Line { id: None, text: text.clone(), kind: Kind::Learnt, pinned: false })
            .collect()
    }

    /// Whisper's model as a mock scenario holds it, if this is one: an install begun has finished by the time the panel opens again.
    pub fn mock_whisper(&mut self) -> Option<crate::whisper::State> {
        use crate::whisper::State;
        if self.mock.is_none() {
            return None;
        }
        if matches!(self.whisper, Some(State::Installing(_))) {
            self.whisper = Some(State::Installed);
        }
        Some(self.whisper.unwrap_or(State::Absent))
    }

    pub fn set_flag(&mut self, key: &str, on: bool) {
        self.put(key, SettingValue::Bool(on));
    }

    pub fn set_text(&mut self, key: &str, text: &str) {
        self.put(key, SettingValue::Text(text.into()));
    }

    /// The bar for one Job: on where it was hidden for this Job and shown again, off where hidden.
    pub fn set_shown_in(&mut self, job: &str, on: bool) {
        let mut off: Vec<String> = self.texts("quickOffFor").into_iter().filter(|one| one != job).collect();
        if !on {
            off.push(job.to_string());
        }
        self.put("quickOffFor", SettingValue::List(off.into_iter().map(SettingValue::Text).collect()));
    }

    /// Move the kept line `id` by `by` places; pinned lines stay first. False where it cannot move.
    pub fn move_line(&mut self, id: &str, by: isize) -> bool {
        let moved = moved_line(self.kept(), id, by);
        let changed = moved.iter().map(|one| &one.id).ne(self.kept().iter().map(|one| &one.id));
        if changed {
            self.set_kept(moved);
        }
        changed
    }

    /// Reword the kept line `id`: a line left empty or unchanged stays as it was.
    pub fn reword_line(&mut self, id: &str, text: &str) -> bool {
        let text = text.trim();
        let mut lines = self.kept();
        match lines.iter_mut().find(|one| one.id.as_deref() == Some(id)) {
            Some(one) if !text.is_empty() && one.text != text => {
                one.text = text.to_string();
                one.kind = Kind::Saved;
            }
            _ => return false,
        }
        self.set_kept(lines);
        true
    }

    /// The voice Talk hears with, from the `voice` setting.
    pub fn voice(&self) -> crate::voice::Engine {
        match self.look.as_ref().and_then(|look| look.get("voice")) {
            Some(SettingValue::Text(name)) => crate::voice::Engine::named(name),
            _ => crate::voice::Engine::Apple,
        }
    }

    fn flag(&self, key: &str) -> bool {
        matches!(self.look.as_ref().and_then(|look| look.get(key)), Some(SettingValue::Bool(true)))
    }

    pub fn texts(&self, key: &str) -> Vec<String> {
        match self.look.as_ref().and_then(|look| look.get(key)) {
            Some(SettingValue::List(items)) => items.iter().filter_map(|one| if let SettingValue::Text(text) = one { Some(text.clone()) } else { None }).collect(),
            _ => Vec::new(),
        }
    }

    /// The lines he saved, in the order the setting keeps them.
    pub fn kept(&self) -> Vec<Line> {
        let Some(SettingValue::List(items)) = self.look.as_ref().and_then(|look| look.get("quick")) else { return Vec::new() };
        items
            .iter()
            .filter_map(|one| {
                let SettingValue::Map(map) = one else { return None };
                let text = match map.get("text") {
                    Some(SettingValue::Text(text)) => text.clone(),
                    _ => return None,
                };
                let id = match map.get("id") {
                    Some(SettingValue::Text(id)) => Some(id.clone()),
                    _ => None,
                };
                let kind = match map.get("kind") {
                    Some(SettingValue::Text(kind)) if kind == "learnt" => Kind::Learnt,
                    _ => Kind::Saved,
                };
                Some(Line { id, text, kind, pinned: matches!(map.get("pinned"), Some(SettingValue::Bool(true))) })
            })
            .collect()
    }

    /// Whether the bar is drawn in this Job (`quickShownIn`).
    pub fn shown_in(&self, job: &str) -> bool {
        !self.flag("quickOff") && !self.texts("quickOffFor").iter().any(|one| one == job)
    }

    /// The lines as the bar draws them: Fleet's answers, then his own with pinned first, then learnt.
    pub fn lines(&self, job: &str) -> Vec<Line> {
        let mut lines: Vec<Line> = self
            .asked
            .get(job)
            .into_iter()
            .flatten()
            .map(|text| Line { id: None, text: text.clone(), kind: Kind::Asked, pinned: false })
            .collect();
        let kept = self.kept();
        lines.extend(kept.iter().filter(|one| one.pinned).cloned());
        lines.extend(kept.iter().filter(|one| !one.pinned).cloned());
        if self.flag_or("learn", true) {
            let forgotten = self.texts("forgotten");
            for text in &self.learnt {
                if !forgotten.contains(text) && !lines.iter().any(|one| one.text == *text) {
                    lines.push(Line { id: None, text: text.clone(), kind: Kind::Learnt, pinned: false });
                }
            }
        }
        lines
    }

    pub fn set_kept(&mut self, lines: Vec<Line>) {
        let items = lines
            .into_iter()
            .map(|one| {
                let mut map = BTreeMap::new();
                map.insert("id".to_string(), SettingValue::Text(one.id.unwrap_or_default()));
                map.insert("text".to_string(), SettingValue::Text(one.text));
                map.insert("kind".to_string(), SettingValue::Text(if one.kind == Kind::Learnt { "learnt" } else { "saved" }.into()));
                if one.pinned {
                    map.insert("pinned".to_string(), SettingValue::Bool(true));
                }
                SettingValue::Map(map)
            })
            .collect();
        self.look.get_or_insert_with(BTreeMap::new).insert("quick".into(), SettingValue::List(items));
    }
}

/// `lines` with `id` moved by `by` places, then the pinned ones first again (`pinnedFirst`).
pub(super) fn moved_line(mut lines: Vec<Line>, id: &str, by: isize) -> Vec<Line> {
    let Some(from) = lines.iter().position(|one| one.id.as_deref() == Some(id)) else { return lines };
    let to = from as isize + by;
    if (0..lines.len() as isize).contains(&to) {
        let one = lines.remove(from);
        lines.insert(to as usize, one);
    }
    let (mut pinned, rest): (Vec<Line>, Vec<Line>) = lines.into_iter().partition(|one| one.pinned);
    pinned.extend(rest);
    pinned
}

/// What a line's menu offers, by its id.
pub(super) const EDIT: &str = "edit";
pub(super) const PIN: &str = "pin";
pub(super) const DELETE: &str = "delete";
const SAVE_ASKED: &str = "save-asked";

impl JobView {
    /// Read `thread.look` once from Fleet.
    pub(super) fn want_look(&mut self, cx: &mut Context<Self>) {
        if self.composer.quick.looked || self.fleet.is_none() {
            return;
        }
        let Some(port) = self.port(cx) else { return };
        self.composer.quick.looked = true;
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, "/settings"))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<SettingsList>("settings", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match answer {
                    Ok(list) => {
                        let value = list.settings.into_iter().find(|one| one.key == KEY).map(|one| one.value);
                        if let Some(SettingValue::Map(map)) = value {
                            this.composer.quick.look = Some(map);
                        }
                    }
                    Err(_) => this.composer.quick.looked = false,
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The message Fleet's answers are for: the thread's last row that is a message.
    fn last_message(&self, id: &str, cx: &gpui::App) -> String {
        self.thread(id, cx)
            .and_then(|held| {
                held.items.iter().rev().find_map(|item| match item {
                    ThreadItem::Row { row: ipc::SessionRow::Message { id, .. } } => Some(id.clone()),
                    _ => None,
                })
            })
            .unwrap_or_default()
    }

    /// Ask Fleet for the answers to the Job's last message, once for each message.
    pub(super) fn quick_follow(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let message = self.last_message(&id, cx);
        if self.composer.quick.asking.get(&id) == Some(&message) {
            return;
        }
        self.composer.quick.asking.insert(id.clone(), message.clone());
        if let Some(mock) = &self.composer.quick.mock {
            let answers = mock.asked.clone();
            self.composer.quick.asked.insert(id, answers);
            return;
        }
        let Some(port) = self.port(cx) else { return };
        let path = format!("/sessions/quick?session_id={}", super::wire::encoded(&id));
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<SessionQuick>("SessionQuick", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match answer {
                    Ok(read) => {
                        this.composer.quick.asked.insert(id, read.asked);
                        this.composer.quick.learnt = read.learnt;
                    }
                    // A read that failed is asked again the next time the last message moves.
                    Err(_) => {
                        this.composer.quick.asking.remove(&id);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Whether a card in the thread waits on the person: the bar steps aside (`waits`).
    pub(super) fn waits(&self, id: &str, cx: &gpui::App) -> bool {
        self.thread(id, cx).is_some_and(|held| {
            held.head.asked.is_some()
                || held.head.lapsed.is_some()
                || held.head.plan.as_ref().is_some_and(|plan| plan.state == ipc::PlanCardState::Waiting)
        })
    }

    /// Write the look. The bar already shows it; a refusal puts the old one back with Fleet's reason.
    pub(super) fn write_look(&mut self, was: Option<BTreeMap<String, SettingValue>>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(map) = self.composer.quick.look.clone() else { return };
        let mut changes = BTreeMap::new();
        changes.insert(KEY.to_string(), Some(SettingValue::Map(map)));
        let job = self.shown.clone();
        let done: Done = Box::new(move |this, answer, _, cx| {
            if let Err(why) = answer {
                this.composer.quick.look = was;
                if let Some(page) = job.as_ref().and_then(|job| this.pages.get_mut(job)) {
                    page.refused = Some(why.into());
                }
            }
            cx.notify();
        });
        self.post("/settings/save".into(), ipc::encode(&SaveSettings { changes }), window, cx, done);
        cx.notify();
    }

    /// Save what is typed as a quick line (skips a line already kept).
    fn save_typed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.kept(cx).text.trim().to_string();
        self.save_line(text, window, cx);
    }

    pub(super) fn save_line(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        if text.is_empty() || self.composer.quick.kept().iter().any(|one| one.text == text) {
            return;
        }
        let was = self.composer.quick.look.clone();
        let mut lines = self.composer.quick.kept();
        lines.push(Line { id: Some(saved_id()), text, kind: Kind::Saved, pinned: false });
        self.composer.quick.set_kept(lines);
        self.write_look(was, window, cx);
    }

    /// Press line `n`: send it as it is, or with ⇧ put it after the draft. False where there is no line.
    pub(super) fn press_quick(&mut self, n: usize, shift: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.shown.clone() else { return false };
        if !self.composer.quick.shown_in(&id) || self.waits(&id, cx) {
            return false;
        }
        let Some(line) = self.composer.quick.lines(&id).get(n).cloned() else { return false };
        self.press_line(line.text, shift, window, cx);
        true
    }

    fn press_line(&mut self, text: String, shift: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !shift {
            self.send_line(text, window, cx);
            return;
        }
        let held = self.kept(cx);
        let joined = if held.text.trim().is_empty() { text } else { format!("{} {text}", held.text.trim_end()) };
        let kept = super::composer::Kept { text: joined, tags: held.tags };
        self.composer.setting = true;
        let content = kept.clone();
        self.composer.input.update(cx, |input, cx| {
            input.set_value(content.text.clone(), window, cx);
            let end = input.value().len();
            input.set_selected_range(end..end, cx);
            input.focus(window, cx);
        });
        self.typed_now(cx);
    }

    fn line_by(&self, id: &str) -> Option<Line> {
        self.composer.quick.kept().into_iter().find(|one| one.id.as_deref() == Some(id))
    }

    /// A learnt line's act: Pin keeps it as his own line, pinned; Delete forgets it; Edit forgets its
    /// words and opens his own line in its place, which a cancel takes back.
    fn act_on_learnt(&mut self, text: String, act: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let was = self.composer.quick.look.clone();
        let mut lines = self.composer.quick.kept();
        if act != PIN {
            self.forget(text.clone());
        }
        match act {
            PIN => {
                let place = lines.iter().filter(|other| other.pinned).count();
                lines.insert(place, Line { id: Some(saved_id()), text, kind: Kind::Saved, pinned: true });
            }
            DELETE => {}
            EDIT => {
                let id = saved_id();
                lines.push(Line { id: Some(id.clone()), text, kind: Kind::Saved, pinned: false });
                self.composer.quick.set_kept(lines);
                self.start_edit(id, Some(was), window, cx);
                return;
            }
            _ => return,
        }
        self.composer.quick.set_kept(lines);
        self.write_look(was, window, cx);
    }

    fn forget(&mut self, text: String) {
        let mut forgotten = self.composer.quick.texts("forgotten");
        if forgotten.contains(&text) {
            return;
        }
        forgotten.push(text);
        let look = self.composer.quick.look.get_or_insert_with(BTreeMap::new);
        look.insert("forgotten".into(), SettingValue::List(forgotten.into_iter().map(SettingValue::Text).collect()));
    }

    pub(super) fn act_on_line(&mut self, id: String, act: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let was = self.composer.quick.look.clone();
        let mut lines = self.composer.quick.kept();
        match act {
            PIN => {
                let Some(at) = lines.iter().position(|one| one.id.as_deref() == Some(&id)) else { return };
                let mut one = lines.remove(at);
                one.pinned = !one.pinned;
                one.kind = Kind::Saved;
                // Pinning puts the line last among the pinned, the order they were pinned in.
                let place = lines.iter().filter(|other| other.pinned).count();
                lines.insert(if one.pinned { place } else { lines.len() }, one);
            }
            DELETE => {
                let Some(gone) = lines.iter().find(|one| one.id.as_deref() == Some(&id)).map(|one| one.text.clone()) else { return };
                lines.retain(|one| one.id.as_deref() != Some(&id));
                let mut forgotten = self.composer.quick.texts("forgotten");
                forgotten.push(gone);
                if let Some(look) = self.composer.quick.look.as_mut() {
                    look.insert("forgotten".into(), SettingValue::List(forgotten.into_iter().map(SettingValue::Text).collect()));
                }
            }
            EDIT => {
                self.start_edit(id, None, window, cx);
                return;
            }
            _ => return,
        }
        self.composer.quick.set_kept(lines);
        self.write_look(was, window, cx);
    }

    fn start_edit(&mut self, id: String, before: Option<Option<BTreeMap<String, SettingValue>>>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(line) = self.line_by(&id) else { return };
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |input, cx| {
            input.set_value(line.text.clone(), window, cx);
            input.select_all(window, cx);
            input.focus(window, cx);
        });
        let entered = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } | InputEvent::Blur => this.finish_edit(true, window, cx),
            _ => {}
        });
        self.composer.quick.edit = Some(Edit { id, input, before, _subs: vec![entered] });
        cx.notify();
    }

    /// Enter and leaving save, Esc cancels, and a line left empty or unchanged is a cancel.
    fn finish_edit(&mut self, keep: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.composer.quick.edit.take() else { return };
        let said = edit.input.read(cx).value().trim().to_string();
        let box_keys = self.composer.input.read(cx).focus_handle(cx);
        window.focus(&box_keys, cx);
        let was = edit.before.clone().unwrap_or_else(|| self.composer.quick.look.clone());
        let mut lines = self.composer.quick.kept();
        let Some(one) = lines.iter_mut().find(|one| one.id.as_deref() == Some(&edit.id)) else { return };
        if keep && !said.is_empty() && said != one.text {
            one.text = said;
            one.kind = Kind::Saved;
            self.composer.quick.set_kept(lines);
            self.write_look(was, window, cx);
        } else if let Some(before) = edit.before {
            // A learnt line left as it was stays learnt: nothing was written.
            self.composer.quick.look = before;
        }
        cx.notify();
    }

    fn hide(&mut self, everywhere: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let was = self.composer.quick.look.clone();
        let look = self.composer.quick.look.get_or_insert_with(BTreeMap::new);
        if everywhere {
            look.insert("quickOff".into(), SettingValue::Bool(true));
        } else {
            let mut off = self.composer.quick.texts("quickOffFor");
            off.push(job.to_string());
            if let Some(look) = self.composer.quick.look.as_mut() {
                look.insert("quickOffFor".into(), SettingValue::List(off.into_iter().map(SettingValue::Text).collect()));
            }
        }
        self.write_look(was, window, cx);
    }

    fn open_hide_menu(&mut self, at: gpui::Point<gpui::Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let entries = vec![
            Entry::Act { id: "hide-job", label: "Hide for this Job".into(), icon: Some("eye-off"), danger: false },
            Entry::Act { id: "hide-all", label: "Hide for all Jobs".into(), icon: Some("eye-off"), danger: false },
        ];
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Hide quick replies", entries, at, Anchor::BottomRight, from_keys, cx));
        let picked = cx.subscribe_in(&menu, window, |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.hide(*id == "hide-all", window, cx);
        });
        self.open_popup(menu, back, vec![picked], window, cx);
    }

    /// A line's menu: Edit, Pin, Delete for a kept or learnt line; Save for one of Fleet's answers.
    fn open_line_menu(&mut self, line: Line, at: gpui::Point<gpui::Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let entries = match (&line.id, line.kind) {
            (Some(_), _) | (None, Kind::Learnt) => vec![
                Entry::Act { id: EDIT, label: "Edit".into(), icon: Some("square-pen"), danger: false },
                Entry::Act { id: PIN, label: if line.pinned { "Unpin" } else { "Pin" }.into(), icon: Some("arrow-up-to-line"), danger: false },
                Entry::Rule,
                Entry::Act { id: DELETE, label: "Delete".into(), icon: Some("trash"), danger: true },
            ],
            (None, Kind::Asked) => {
                let kept = self.composer.quick.kept().iter().any(|one| one.text == line.text);
                vec![Entry::Item {
                    id: SAVE_ASKED,
                    label: "Save".into(),
                    icon: Some("message-square-quote"),
                    danger: false,
                    key: None,
                    off: kept.then(|| "Already a quick reply".into()),
                }]
            }
            _ => return,
        };
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Quick reply", entries, at, Anchor::TopLeft, from_keys, cx).at_point(window, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            match (*id, line.id.clone()) {
                (SAVE_ASKED, _) => this.save_line(line.text.clone(), window, cx),
                (act, Some(job)) => this.act_on_line(job, act, window, cx),
                (act, None) if line.kind == Kind::Learnt => this.act_on_learnt(line.text.clone(), act, window, cx),
                _ => {}
            }
        });
        self.open_popup(menu, back, vec![picked], window, cx);
    }

    /// The bar (4.7): the chips, then Save and Hide. Not drawn where it is hidden or a card waits.
    pub(super) fn render_quick(&self, id: &SharedString, look: &Look, _window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.composer.quick.shown_in(id) || self.waits(id, cx) {
            return None;
        }
        let lines = self.composer.quick.lines(id);
        let asked = lines.iter().filter(|one| one.kind == Kind::Asked).count();
        let mut chips: Vec<AnyElement> = Vec::new();
        for (at, line) in lines.iter().enumerate() {
            if asked > 0 && at == asked {
                chips.push(div().flex_none().w(look.length(token::BORDER_WIDTH)).h(look.length(token::SPACE_4)).bg(look.color(token::BORDER_DEFAULT)).into_any_element());
            }
            if let (Some(edit), Some(line_id)) = (&self.composer.quick.edit, &line.id) {
                if edit.id == *line_id {
                    chips.push(
                        labelled(
                            div()
                                .id("quick-edit")
                                .flex_none()
                                .w(look.length(token::SPACE_12) * 3.)
                                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                    if event.keystroke.key == "escape" {
                                        this.finish_edit(false, window, cx);
                                        cx.stop_propagation();
                                    }
                                }))
                                .text_size(look.length(token::TEXT_XS))
                                .child(Input::new(&edit.input).appearance(false).cleanable(false)),
                            Role::TextInput,
                            format!("Edit {}", line.text),
                        )
                        .into_any_element(),
                    );
                    continue;
                }
            }
            let (glyph, word) = match line.kind {
                Kind::Asked => ("message-circle-question-mark", "Asked"),
                Kind::Learnt => ("repeat", "Learnt"),
                Kind::Saved => ("message-square-quote", "Saved"),
            };
            let text = line.text.clone();
            let name: SharedString = text.clone().into();
            let menu_line = line.clone();
            let key_line = line.clone();
            let press = text.clone();
            chips.push(
                labelled(
                    div()
                        .id(SharedString::from(format!("quick-{at}")))
                        .tab_index(0)
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_1))
                        .h(look.length(token::H_CONTROL))
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .bg(look.color(token::BG_RAISED))
                        .text_size(look.length(token::TEXT_XS))
                        .text_color(look.color(token::FG_DEFAULT))
                        .whitespace_nowrap()
                        .cursor_pointer()
                        .hover(|style| style.border_color(look.color(token::BORDER_DEFAULT)).bg(look.color(token::BG_HOVER)))
                        .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                        .child(
                            div().id(SharedString::from(format!("quick-kind-{at}"))).flex_none().tooltip(tip(word.into())).child(
                                svg().path(icon_path(glyph)).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)),
                            ),
                        )
                        .child(text.clone())
                        .when(line.pinned, |chip| {
                            chip.child(
                                div().id(SharedString::from(format!("quick-pin-{at}"))).flex_none().tooltip(tip("Pinned".into())).child(
                                    svg().path(icon_path("arrow-up-to-line")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)),
                                ),
                            )
                        })
                        .when(at < 9, |chip| {
                            chip.child(div().ml(look.length(token::SPACE_1)).text_color(look.color(token::FG_SUBTLE)).child(format!("\u{2318}{}", at + 1)))
                        })
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            this.press_line(press.clone(), event.modifiers().shift, window, cx);
                        }))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                                this.open_line_menu(menu_line.clone(), event.position, false, window, cx);
                                cx.stop_propagation();
                            }),
                        )
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                            let keys = &event.keystroke;
                            let menu_key = keys.key == "menu" || (keys.key == "f10" && keys.modifiers.shift);
                            if menu_key {
                                if let Some(at) = drawn(window, cx, Role::Button, &key_line.text).map(below_left) {
                                    this.open_line_menu(key_line.clone(), at, true, window, cx);
                                    cx.stop_propagation();
                                }
                            } else if keys.key == "enter" || keys.key == "space" {
                                this.press_line(key_line.text.clone(), keys.modifiers.shift, window, cx);
                                cx.stop_propagation();
                            }
                        })),
                    Role::Button,
                    name,
                )
                .into_any_element(),
            );
        }
        let empty = self.kept(cx).text.trim().is_empty();
        let save = labelled(
            icon_button(look, "quick-save", "message-square-quote", "Save what is typed as a quick reply".into(), false, empty)
                .when(empty, |button| button.opacity(0.4))
                .when(!empty, |button| button.on_click(cx.listener(|this, _, window, cx| this.save_typed(window, cx)))),
            Role::Button,
            "Save as quick reply",
        );
        let hide = labelled(
            icon_button(look, "quick-hide", "eye-off", "Hide quick replies".into(), false, false).on_click(cx.listener(
                |this, event: &ClickEvent, window, cx| {
                    let at = drawn(window, cx, Role::Button, "Hide quick replies").map(|bounds| point(bounds.origin.x + bounds.size.width, bounds.origin.y)).unwrap_or(event.position());
                    this.open_hide_menu(at, event.is_keyboard(), window, cx);
                },
            )),
            Role::Button,
            "Hide quick replies",
        );
        Some(
            labelled(
                div()
                    .id("quick")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .pl(look.length(token::SPACE_4))
                    .pr(look.length(token::SPACE_2))
                    .pt(look.length(token::SPACE_2))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| this.open_hide_menu(event.position, false, window, cx)),
                    )
                    .child(div().id("quick-chips").flex_1().min_w_0().flex().items_center().gap(look.length(token::SPACE_1)).overflow_x_scroll().children(chips))
                    .child(save)
                    .child(hide),
                Role::Group,
                "Quick replies",
            )
            .into_any_element(),
        )
    }
}

/// A new line of his own, named by when it was made.
fn saved_id() -> String {
    format!("saved:{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quick() -> Quick {
        let look = ipc::decode::<SettingValue>(
            "look",
            br#"{"learn":true,"quickOff":false,"quickOffFor":["gone"],"forgotten":["old"],"quick":[
              {"id":"a","text":"Later","kind":"saved"},{"id":"b","text":"First","kind":"saved","pinned":true}]}"#,
        )
        .expect("a look");
        let mut quick = Quick::mock(look, SessionQuick { asked: vec!["Yes".into()], learnt: vec!["old".into(), "Often".into(), "Later".into()] });
        quick.asked.insert("job".into(), vec!["Yes".into()]);
        quick
    }

    #[test]
    fn the_bar_draws_fleets_answers_then_pinned_lines_then_the_rest_then_learnt_ones_he_did_not_drop() {
        let texts: Vec<_> = quick().lines("job").into_iter().map(|one| (one.text, one.kind)).collect();
        assert_eq!(
            texts,
            [("Yes".to_string(), Kind::Asked), ("First".into(), Kind::Saved), ("Later".into(), Kind::Saved), ("Often".into(), Kind::Learnt)]
        );
    }

    #[test]
    fn the_thread_reads_by_the_look_and_by_the_defaults_where_it_has_none() {
        let mut quick = Quick::default();
        assert_eq!(quick.thread(), Thread { show_times: true, text_size: TextSize::Base, spacing: Spacing::Default });
        quick.put("showTimes", SettingValue::Bool(false));
        quick.put("textSize", SettingValue::Text("lg".into()));
        quick.put("spacing", SettingValue::Text("roomy".into()));
        assert_eq!(quick.thread(), Thread { show_times: false, text_size: TextSize::Lg, spacing: Spacing::Roomy });
        assert_eq!(quick.responses().len(), 4);
    }

    #[test]
    fn responses_are_added_up_to_six_removed_down_to_one_and_moved_within_the_list() {
        let list = Response::defaults();
        let added = act_on_responses(list.clone(), ResponseAct::Add { emoji: "\u{1f525}".into(), name: "Fire".into() });
        assert_eq!(added.len(), 5);
        let full = act_on_responses(act_on_responses(added, ResponseAct::Add { emoji: "a".into(), name: "A".into() }), ResponseAct::Add { emoji: "b".into(), name: "B".into() });
        assert_eq!(full.len(), MAX_RESPONSES);
        let one = [list[0].clone()].to_vec();
        assert_eq!(act_on_responses(one.clone(), ResponseAct::Remove("pick".into())), one);
        let moved = act_on_responses(list.clone(), ResponseAct::Move { id: "pick".into(), by: 1 });
        assert_eq!(moved[1].id, "pick");
        assert_eq!(act_on_responses(list.clone(), ResponseAct::Move { id: "pick".into(), by: -1 }), list);
        let edited = act_on_responses(list, ResponseAct::Edit { id: "yes".into(), emoji: "x".into(), name: "Y".into() });
        assert_eq!(edited[1].name, "Y");
    }

    #[test]
    fn one_emoji_is_what_a_response_takes() {
        assert_eq!(first_one(" \u{1f44d}\u{1f44e}"), "\u{1f44d}");
        assert_eq!(first_one("\u{2764}\u{fe0f} x"), "\u{2764}\u{fe0f}");
        assert_eq!(first_one("\u{1f468}\u{200d}\u{1f469}"), "\u{1f468}\u{200d}\u{1f469}");
        assert_eq!(first_one("\u{1f1fa}\u{1f1f8}\u{1f1fa}"), "\u{1f1fa}\u{1f1f8}");
        assert!(is_emoji("\u{1f525}") && is_emoji("\u{26a0}\u{fe0f}"));
        assert!(!is_emoji("") && !is_emoji("a") && !is_emoji("\u{1f525}\u{1f525}"));
        assert!(REACTIONS.iter().all(|(emoji, _)| is_emoji(emoji)));
    }

    #[test]
    fn a_line_moves_one_place_and_the_pinned_ones_stay_first() {
        let line = |id: &str, pinned: bool| Line { id: Some(id.into()), text: id.into(), kind: Kind::Saved, pinned };
        let list = vec![line("a", true), line("b", false), line("c", false)];
        let order = |lines: &[Line]| lines.iter().map(|one| one.text.clone()).collect::<Vec<_>>().join("");
        assert_eq!(order(&moved_line(list.clone(), "c", -1)), "acb");
        assert_eq!(order(&moved_line(list.clone(), "b", 1)), "acb");
        assert_eq!(order(&moved_line(list.clone(), "c", 1)), "abc");
        // Moved above a pinned line, it falls back under the pinned ones.
        assert_eq!(order(&moved_line(list, "b", -1)), "abc");
    }

    #[test]
    fn a_reworded_line_is_saved_and_an_empty_or_unchanged_one_is_left() {
        let mut quick = Quick::default();
        quick.set_kept(vec![Line { id: Some("x".into()), text: "Keep going".into(), kind: Kind::Learnt, pinned: false }]);
        assert!(!quick.reword_line("x", "  "));
        assert!(!quick.reword_line("x", "Keep going"));
        assert!(quick.reword_line("x", " Carry on "));
        assert_eq!(quick.kept()[0].text, "Carry on");
        assert_eq!(quick.kept()[0].kind, Kind::Saved);
    }

    #[test]
    fn the_bar_is_hidden_for_one_job_and_shown_again() {
        let mut quick = Quick::default();
        assert!(quick.shown_in("a"));
        quick.set_shown_in("a", false);
        assert!(!quick.shown_in("a") && quick.shown_in("b"));
        quick.set_shown_in("a", true);
        assert!(quick.shown_in("a"));
    }

    #[test]
    fn learnt_lines_are_listed_unless_learning_is_off_or_he_kept_or_dropped_them() {
        let mut quick = Quick::default();
        quick.learnt = vec!["Go on".into(), "Ship it".into(), "Stop".into()];
        assert_eq!(quick.learnt_lines().len(), 3, "learning is on until it is switched off");
        quick.set_kept(vec![Line { id: Some("a".into()), text: "Ship it".into(), kind: Kind::Saved, pinned: false }]);
        quick.put("forgotten", SettingValue::List(vec![SettingValue::Text("Stop".into())]));
        assert_eq!(quick.learnt_lines().iter().map(|one| one.text.as_str()).collect::<Vec<_>>(), ["Go on"]);
        quick.set_flag("learn", false);
        assert!(quick.learnt_lines().is_empty());
    }

    #[test]
    fn a_mock_install_that_was_begun_is_done_when_the_panel_opens_again() {
        use crate::whisper::State;
        let mut real = Quick::default();
        assert_eq!(real.mock_whisper(), None, "a Fleet's Mac holds the real state");
        let mut quick = Quick::mock(SettingValue::Map(Default::default()), ipc::decode("SessionQuick", br#"{"asked":[],"learnt":[]}"#).expect("quick"));
        assert_eq!(quick.mock_whisper(), Some(State::Absent));
        quick.whisper = Some(State::Installing(0.4));
        assert_eq!(quick.mock_whisper(), Some(State::Installed));
    }

    #[test]
    fn a_job_the_bar_is_hidden_for_draws_none() {
        assert!(quick().shown_in("job"));
        assert!(!quick().shown_in("gone"));
    }

    #[test]
    fn lines_written_back_keep_their_order_and_their_pin() {
        let mut quick = quick();
        let mut lines = quick.kept();
        lines.swap(0, 1);
        quick.set_kept(lines);
        let kept = quick.kept();
        assert_eq!((kept[0].text.as_str(), kept[0].pinned), ("First", true));
        assert_eq!((kept[1].text.as_str(), kept[1].pinned), ("Later", false));
    }
}
