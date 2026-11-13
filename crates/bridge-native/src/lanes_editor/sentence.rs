//! A rule opened as one sentence where its chip stands, or empty from Add rule: **When** a moment, **if** a condition, **do**
//! an act, and **Ask me first**. Each field opens its choices as a small menu under it; one that needs a value chains into a
//! second choice or into words in the same menu. Every choice is an edit of the draft, kept as the rule's piece draft with
//! the line that says what changed, so ⌘Z and the Changes list take it back as they take back a step.
//!
//! Keys: Tab and Shift-Tab between the fields, Enter or Space opens one (a switch flips), ↑ ↓ and Enter in the menu, ← back
//! to the first choice, Esc shuts the menu and then the sentence, ⌫ on a condition drops it.

use std::rc::Rc;

use gpui::{anchored, deferred, div, point, prelude::*, px, svg, Anchor, AnyElement, Bounds, Context, Entity, Focusable, FocusHandle, FontWeight, Keystroke, MouseButton, Pixels, Role, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{LifecyclePieceKind, LifecycleRule, PieceScope, RuleAct, RuleCondition, RuleWhen};

use super::in_place::unique_id;
use super::patch::RuleChange;
use super::rule::{self, Next, Opt, Set, Vocab};
use super::{Call, LanesEditor};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::lanes_canvas::Pick;
use crate::look::{token, Look};
use crate::settings::definition::steps_of;
use crate::settings::view::{refusal, ringed, switch_track, tip};

/// The most a field says of a long act before it is cut. The tooltip says the rest.
const FIELD_CHARS: usize = 56;

/// One field of the sentence, or its Add condition press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Slot {
    When,
    If(usize),
    AddIf,
    Do(usize),
    Ask,
}

struct Handles {
    when: FocusHandle,
    ifs: Vec<FocusHandle>,
    add_if: FocusHandle,
    dos: Vec<FocusHandle>,
    ask: FocusHandle,
}

/// Words typed in a menu, which make the field's value.
struct Typed {
    name: &'static str,
    input: Entity<InputState>,
    make: Rc<dyn Fn(String) -> Set>,
    _blur: Subscription,
}

/// A field's choices while they are open.
struct Menu {
    slot: Slot,
    shown: Vec<Opt>,
    /// The choices the second one came from, and what it was called.
    back: Vec<(Vec<Opt>, Option<String>)>,
    title: Option<String>,
    at: usize,
    typed: Option<Typed>,
}

pub(super) struct Sentence {
    /// The rule's id, once it has one: a new rule has none until its moment and its act are said.
    id: Option<String>,
    /// Who the rule is: its id, name, scope and what it does on failure, which no field here changes.
    base: LifecycleRule,
    when: Option<RuleWhen>,
    /// A condition not chosen yet is `None`, and is not part of the rule.
    ifs: Vec<Option<RuleCondition>>,
    dos: Vec<Option<RuleAct>>,
    /// The rule's own word on how far it goes alone: absent takes what its act's kind is set to.
    autonomy: Option<String>,
    handles: Handles,
    menu: Option<Menu>,
}

fn name_of(slot: Slot) -> String {
    match slot {
        Slot::When => "When".into(),
        Slot::If(0) => "If".into(),
        Slot::If(at) => format!("If {}", at + 1),
        Slot::AddIf => "Add condition".into(),
        Slot::Do(0) => "Do".into(),
        Slot::Do(at) => format!("Do {}", at + 1),
        Slot::Ask => "Ask me first".into(),
    }
}

impl Sentence {
    fn handles(ifs: usize, dos: usize, cx: &mut Context<LanesEditor>) -> Handles {
        let stop = |cx: &mut Context<LanesEditor>| cx.focus_handle().tab_stop(true);
        Handles { when: stop(cx), ifs: (0..ifs).map(|_| stop(cx)).collect(), add_if: stop(cx), dos: (0..dos).map(|_| stop(cx)).collect(), ask: stop(cx) }
    }

    /// The sentence of a rule that is there.
    fn of(rule: LifecycleRule, cx: &mut Context<LanesEditor>) -> Sentence {
        Sentence {
            id: Some(rule.id.clone()),
            when: Some(rule.when.clone()),
            ifs: rule.conditions.iter().cloned().map(Some).collect(),
            dos: rule.does.iter().map(|one| Some(one.what.clone())).collect(),
            autonomy: rule.autonomy.clone(),
            handles: Sentence::handles(rule.conditions.len(), rule.does.len(), cx),
            base: rule,
            menu: None,
        }
    }

    /// A sentence with nothing said.
    fn blank(cx: &mut Context<LanesEditor>) -> Sentence {
        let base = rule::blank("", "", PieceScope::Everywhere, RuleWhen::Job { event: "started".into() }, Vec::new());
        Sentence { id: None, base, when: None, ifs: Vec::new(), dos: vec![None], autonomy: None, handles: Sentence::handles(0, 1, cx), menu: None }
    }

    /// Every stop, in the order they are drawn.
    fn slots(&self) -> Vec<Slot> {
        let mut out = vec![Slot::When];
        out.extend((0..self.ifs.len()).map(Slot::If));
        out.push(Slot::AddIf);
        out.extend((0..self.dos.len()).map(Slot::Do));
        out.push(Slot::Ask);
        out
    }

    fn handle(&self, slot: Slot) -> &FocusHandle {
        match slot {
            Slot::When => &self.handles.when,
            Slot::If(at) => &self.handles.ifs[at],
            Slot::AddIf => &self.handles.add_if,
            Slot::Do(at) => &self.handles.dos[at],
            Slot::Ask => &self.handles.ask,
        }
    }

    pub(super) fn stops(&self) -> Vec<FocusHandle> {
        self.slots().into_iter().map(|slot| self.handle(slot).clone()).collect()
    }

    fn focused(&self, window: &Window) -> Option<Slot> {
        self.slots().into_iter().find(|slot| self.handle(*slot).is_focused(window))
    }

    pub(super) fn typed_focused(&self, window: &Window, cx: &gpui::App) -> bool {
        self.menu.as_ref().and_then(|menu| menu.typed.as_ref()).is_some_and(|typed| typed.input.read(cx).focus_handle(cx).is_focused(window))
    }

    pub(super) fn is_rule(&self, id: &str) -> bool {
        self.id.as_deref() == Some(id)
    }

    /// The first act's kind, which says what the rule asks by default.
    fn first_kind(&self) -> Option<&'static str> {
        self.dos.iter().flatten().next().map(rule::kind_of)
    }

    fn asking(&self, vocab: &Vocab) -> bool {
        self.autonomy.as_deref().or_else(|| self.first_kind().and_then(|kind| vocab.default_for(kind))) == Some("asks")
    }

    /// The rule as the fields say it, once it has a moment and an act.
    fn build(&self, vocab: &Vocab) -> Option<LifecycleRule> {
        let when = self.when.clone()?;
        let does: Vec<_> = self.dos.iter().flatten().map(|act| rule::doing(vocab, self.autonomy.as_deref(), act.clone())).collect();
        if does.is_empty() {
            return None;
        }
        Some(LifecycleRule { when, conditions: self.ifs.iter().flatten().cloned().collect(), does, autonomy: self.autonomy.clone(), ..self.base.clone() })
    }

    fn text(&self, slot: Slot, vocab: &Vocab) -> Option<String> {
        match slot {
            Slot::When => self.when.as_ref().map(|when| rule::when_phrase(vocab, when)),
            Slot::If(at) => self.ifs[at].as_ref().map(|one| rule::condition_phrase(vocab, one)),
            Slot::Do(at) => self.dos[at].as_ref().map(|act| rule::act_phrase(vocab, act)),
            Slot::AddIf | Slot::Ask => None,
        }
    }
}

impl LanesEditor {
    /// What a rule's choices are filled from, as the editor has it now.
    pub(super) fn vocab(&self) -> Vocab {
        Vocab {
            steps: steps_of(&self.text()),
            lifecycles: self.lifecycles.iter().filter(|one| one.kept.overridden_by.is_none()).map(|one| one.lifecycle.clone()).collect(),
            workflows: self.workflows.clone(),
            recipes: self.recipes.clone(),
            defaults: self.defaults.clone(),
        }
    }

    /// Each rule the draft sets, with the rule that runs where there is one: the piece drafts, then the edits still waiting
    /// for where the draft applies.
    pub(super) fn rule_changes(&self) -> Vec<RuleChange> {
        let mut out: Vec<RuleChange> = self
            .pieces
            .iter()
            .filter(|piece| piece.kind == LifecyclePieceKind::Rule)
            .filter_map(|piece| rule::rule_of(&piece.text).map(|now| RuleChange { was: self.running_rule(&piece.id, &piece.scope), now }))
            .collect();
        for waiting in &self.waiting_rules {
            let was = self.rules.iter().find(|one| one.rule.id == waiting.rule.id && one.kept.overridden_by.is_none()).map(|one| one.rule.clone());
            let change = RuleChange { was, now: waiting.rule.clone() };
            match out.iter_mut().find(|one| one.now.id == change.now.id) {
                Some(held) => *held = change,
                None => out.push(change),
            }
        }
        out
    }

    /// The rule as the draft has it, else as it runs.
    fn effective_rule(&self, id: &str) -> Option<LifecycleRule> {
        if let Some(waiting) = self.waiting_rules.iter().rev().find(|one| one.rule.id == id) {
            return Some(waiting.rule.clone());
        }
        let scope = self.target.as_ref().map(super::piece_scope);
        let drafted = self.pieces.iter().find(|piece| piece.kind == LifecyclePieceKind::Rule && piece.id == id && scope.as_ref().is_none_or(|scope| piece.scope == *scope)).and_then(|piece| rule::rule_of(&piece.text));
        drafted.or_else(|| self.rules.iter().find(|one| one.rule.id == id && one.kept.overridden_by.is_none()).map(|one| one.rule.clone()))
    }

    /// One edit of one rule: its whole text put as the piece's draft, at the scope the draft applies at, with the line the
    /// history says. Where the draft applies is not known yet, it waits.
    fn edit_rule(&mut self, said: String, rule: LifecycleRule, cx: &mut Context<Self>) {
        self.refused = None;
        self.stale = false;
        match self.target.as_ref().map(super::piece_scope) {
            Some(scope) => {
                let rule = LifecycleRule { scope: scope.clone(), ..rule };
                let text = rule::text_of(&rule);
                self.enqueue(Call::PutPiece { kind: LifecyclePieceKind::Rule, id: rule.id, scope, text, said: Some(said) }, cx);
            }
            None => self.waiting_rules.push(super::draft::WaitingRule { said, rule }),
        }
        self.refresh(cx);
        cx.notify();
    }

    // ---- opening and shutting ---------------------------------------------------------------------

    /// A rule's chip was picked, or Enter was pressed on it: its sentence stands under the chip. With `keyed` the keys go
    /// into its first field.
    pub(super) fn open_rule(&mut self, id: &str, keyed: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.settle(window, cx);
        self.in_place = None;
        if !self.sentence.as_ref().is_some_and(|one| one.is_rule(id)) {
            self.sentence = self.effective_rule(id).map(|rule| Sentence::of(rule, cx));
            self.reveal.set(true);
        }
        if keyed {
            if let Some(sentence) = &self.sentence {
                window.focus(&sentence.handles.when, cx);
            }
        }
        cx.notify();
    }

    /// Add rule: an empty sentence under the bar, at the top of the canvas, with the keys in When and its choices open.
    pub fn add_rule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settle(window, cx);
        self.in_place = None;
        self.state.pick(&self.id, None);
        self.picked = None;
        self.sentence = Some(Sentence::blank(cx));
        self.reveal.set(true);
        self.open_menu(Slot::When, window, cx);
    }

    /// The sentence shut with what is said kept. The keys go back to the canvas.
    fn close_sentence(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sentence = None;
        self.give_back(window, cx);
    }

    fn open_menu(&mut self, slot: Slot, window: &mut Window, cx: &mut Context<Self>) {
        let vocab = self.vocab();
        let Some(sentence) = self.sentence.as_mut() else { return };
        let shown = match slot {
            Slot::When => rule::when_options(&vocab),
            Slot::If(_) => rule::condition_options(&vocab),
            Slot::Do(_) => rule::act_options(&vocab, sentence.when.as_ref()),
            Slot::AddIf | Slot::Ask => return,
        };
        sentence.menu = Some(Menu { slot, shown, back: Vec::new(), title: None, at: 0, typed: None });
        window.focus(sentence.handle(slot), cx);
        cx.notify();
    }

    fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(sentence) = self.sentence.as_mut() else { return };
        if let Some(menu) = sentence.menu.take() {
            window.focus(sentence.handle(menu.slot), cx);
        }
        cx.notify();
    }

    /// A choice made: a value sets the field, a second choice replaces the list, words open a box.
    fn choose(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.sentence.as_mut().and_then(|sentence| sentence.menu.as_mut()) else { return };
        let Some(chosen) = menu.shown.get(at).cloned() else { return };
        match chosen.next {
            Next::Set(set) => self.set_field(set, window, cx),
            Next::More(list) => {
                menu.back.push((std::mem::replace(&mut menu.shown, list), menu.title.replace(chosen.label)));
                menu.at = 0;
                cx.notify();
            }
            Next::Words { name, make } => {
                let input = cx.new(|cx| InputState::new(window, cx));
                let blur = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Blur) && !this.typing(window, cx) {
                        this.close_menu(window, cx);
                    }
                });
                if let Some(menu) = self.sentence.as_mut().and_then(|sentence| sentence.menu.as_mut()) {
                    menu.title = Some(chosen.label);
                    menu.typed = Some(Typed { name, input: input.clone(), make, _blur: blur });
                }
                input.update(cx, |state, cx| state.focus(window, cx));
                cx.notify();
            }
        }
    }

    /// A value chosen for the field whose choices are open: one edit of the rule.
    fn set_field(&mut self, set: Set, window: &mut Window, cx: &mut Context<Self>) {
        let vocab = self.vocab();
        let Some(sentence) = self.sentence.as_mut() else { return };
        let Some(slot) = sentence.menu.as_ref().map(|menu| menu.slot) else { return };
        let named = |sentence: &Sentence| if sentence.base.name.is_empty() { "the rule".to_string() } else { sentence.base.name.clone() };
        let said = match (slot, set) {
            (Slot::When, Set::When(when)) => {
                let said = format!("Changed the trigger of {} to {}", named(sentence), rule::when_phrase(&vocab, &when));
                sentence.when = Some(when);
                said
            }
            (Slot::If(at), Set::Condition(condition)) => {
                let phrase = rule::condition_phrase(&vocab, &condition);
                let said = if sentence.ifs[at].is_some() { format!("Changed the condition of {} to {phrase}", named(sentence)) } else { format!("Added the condition {phrase} to {}", named(sentence)) };
                sentence.ifs[at] = Some(condition);
                said
            }
            (Slot::Do(at), Set::Act(act)) => {
                let said = format!("Changed the act of {} to {}", named(sentence), rule::act_phrase(&vocab, &act));
                sentence.dos[at] = Some(act);
                said
            }
            _ => return,
        };
        self.close_menu(window, cx);
        self.land(said, window, cx);
    }

    /// The sentence as the rule's draft: put once it has a moment and an act, and from then on after each field set. A new
    /// rule is named for what it does and given an id from that name.
    fn land(&mut self, said: String, _window: &mut Window, cx: &mut Context<Self>) {
        let vocab = self.vocab();
        let taken: Vec<String> = self.rules.iter().map(|one| one.rule.id.clone()).chain(self.pieces.iter().filter(|piece| piece.kind == LifecyclePieceKind::Rule).map(|piece| piece.id.clone())).chain(self.waiting_rules.iter().map(|one| one.rule.id.clone())).collect();
        let Some(sentence) = self.sentence.as_mut() else { return };
        let Some(mut rule) = sentence.build(&vocab) else { return };
        let said = if sentence.id.is_none() {
            let act = sentence.dos.iter().flatten().next().map(|act| rule::act_phrase(&vocab, act)).unwrap_or_default();
            let mut name = act.chars().next().map(|first| first.to_uppercase().chain(act.chars().skip(1)).collect::<String>()).unwrap_or_else(|| "New rule".into());
            if name.is_empty() {
                name = "New rule".into();
            }
            rule.id = unique_id(&name, &taken);
            rule.name = name.clone();
            sentence.id = Some(rule.id.clone());
            format!("Added the rule {name}")
        } else {
            said
        };
        sentence.base = rule.clone();
        self.edit_rule(said, rule, cx);
    }

    fn add_condition(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(sentence) = self.sentence.as_mut() else { return };
        if sentence.ifs.last().is_some_and(|one| one.is_none()) {
            let at = sentence.ifs.len() - 1;
            return self.open_menu(Slot::If(at), window, cx);
        }
        sentence.ifs.push(None);
        sentence.handles.ifs.push(cx.focus_handle().tab_stop(true));
        let at = sentence.ifs.len() - 1;
        self.open_menu(Slot::If(at), window, cx);
    }

    /// ⌫ on a condition: gone from the sentence, and from the rule where it was chosen.
    fn drop_condition(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(sentence) = self.sentence.as_mut() else { return };
        if at >= sentence.ifs.len() {
            return;
        }
        let was = sentence.ifs.remove(at);
        sentence.handles.ifs.remove(at);
        let next = if at > 0 { Slot::If(at - 1) } else { Slot::When };
        window.focus(sentence.handle(next), cx);
        if was.is_some() {
            let said = format!("Dropped the condition of {}", if sentence.base.name.is_empty() { "the rule" } else { &sentence.base.name });
            self.land(said, window, cx);
        }
        cx.notify();
    }

    fn flip_ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let vocab = self.vocab();
        let Some(sentence) = self.sentence.as_mut() else { return };
        let on = !sentence.asking(&vocab);
        let default_asks = sentence.first_kind().and_then(|kind| vocab.default_for(kind)) == Some("asks");
        sentence.autonomy = match (on, default_asks) {
            (true, _) => Some("asks".into()),
            (false, true) => Some("acts".into()),
            (false, false) => None,
        };
        let said = format!("Turned Ask me first {} for {}", if on { "on" } else { "off" }, if sentence.base.name.is_empty() { "the rule" } else { &sentence.base.name });
        window.focus(&sentence.handles.ask, cx);
        self.land(said, window, cx);
        cx.notify();
    }

    // ---- keys -----------------------------------------------------------------------------------

    /// Whether the keys are in a box a menu asked words in.
    pub(super) fn typing_words(&self, window: &Window, cx: &gpui::App) -> bool {
        self.sentence.as_ref().is_some_and(|sentence| sentence.typed_focused(window, cx))
    }

    /// The keys of the words box: Enter makes the value of them, Esc shuts the menu.
    pub(super) fn words_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        match key.key.as_str() {
            "enter" => {
                let typed = self.sentence.as_ref().and_then(|sentence| sentence.menu.as_ref()).and_then(|menu| menu.typed.as_ref()).map(|typed| (typed.input.read(cx).value().trim().to_string(), typed.make.clone()));
                if let Some((said, make)) = typed {
                    self.set_field(make(said), window, cx);
                }
                true
            }
            "escape" => {
                self.close_menu(window, cx);
                true
            }
            "tab" => true,
            _ => false,
        }
    }

    /// A move of the keys round the sentence's fields, past the last to the first.
    fn move_in_sentence(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(sentence) = &self.sentence else { return };
        let slots = sentence.slots();
        let at = sentence.focused(window).and_then(|slot| slots.iter().position(|one| *one == slot));
        let next = match (at, back) {
            (None, false) => 0,
            (None, true) => slots.len() - 1,
            (Some(at), false) => (at + 1) % slots.len(),
            (Some(at), true) => (at + slots.len() - 1) % slots.len(),
        };
        window.focus(sentence.handle(slots[next]), cx);
        cx.notify();
    }

    /// The sentence's keys while it holds them. `true` where one was taken.
    pub(super) fn sentence_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        let word = key.key.as_str();
        let Some(sentence) = &self.sentence else { return false };
        let focused = sentence.focused(window);
        if focused.is_none() && !(word == "tab" && self.canvas_focused(window)) {
            return false;
        }
        if let Some(menu) = &sentence.menu {
            let count = menu.shown.len();
            match word {
                "escape" => self.close_menu(window, cx),
                "up" | "down" if menu.typed.is_none() => {
                    if let Some(menu) = self.sentence.as_mut().and_then(|sentence| sentence.menu.as_mut()) {
                        menu.at = if word == "down" { (menu.at + 1) % count.max(1) } else { (menu.at + count.max(1) - 1) % count.max(1) };
                    }
                    cx.notify();
                }
                "enter" if menu.typed.is_none() => self.choose(menu.at, window, cx),
                "left" if menu.typed.is_none() && !menu.back.is_empty() => {
                    if let Some(menu) = self.sentence.as_mut().and_then(|sentence| sentence.menu.as_mut()) {
                        if let Some((shown, title)) = menu.back.pop() {
                            (menu.shown, menu.title, menu.at) = (shown, title, 0);
                        }
                    }
                    cx.notify();
                }
                "tab" => {}
                _ => return false,
            }
            return true;
        }
        match (word, focused) {
            ("tab", _) => self.move_in_sentence(mods.shift, window, cx),
            ("escape", _) => self.close_sentence(window, cx),
            ("enter" | "space", Some(Slot::Ask)) => self.flip_ask(window, cx),
            ("enter" | "space", Some(Slot::AddIf)) => self.add_condition(window, cx),
            ("enter" | "space" | "down", Some(slot @ (Slot::When | Slot::If(_) | Slot::Do(_)))) => self.open_menu(slot, window, cx),
            ("backspace" | "delete", Some(Slot::If(at))) => self.drop_condition(at, window, cx),
            _ => return false,
        }
        true
    }

    // ---- drawn ----------------------------------------------------------------------------------

    /// The rows the sentence wraps to in `room`, packing its pieces in order as the row does: a word with its field,
    /// Add condition, and Ask me first each stay whole.
    fn sentence_lines(&self, sentence: &Sentence, vocab: &Vocab, room: usize) -> f32 {
        let _ = self;
        let (mut lines, mut used) = (1usize, 0usize);
        for slot in sentence.slots() {
            let wide = match slot {
                Slot::AddIf => 16,
                Slot::Ask => 22,
                _ => 10 + sentence.text(slot, vocab).map_or(12, |said| rule::short(&said, FIELD_CHARS).chars().count()),
            };
            if used > 0 && used + wide + 2 > room {
                lines += 1;
                used = 0;
            }
            used += wide + 2;
        }
        lines as f32
    }

    /// The panel's height, worked out from the room its words take at this width, where `place` stands it.
    fn sentence_tall(&self, look: &Look, wide: Pixels, vocab: &Vocab) -> Pixels {
        let Some(sentence) = &self.sentence else { return px(0.) };
        let pad = look.length(token::SPACE_3) * 2. + look.length(token::BORDER_WIDTH) * 2.;
        let per_char = look.length(token::TEXT_SM) * 0.58;
        let lines = self.sentence_lines(sentence, vocab, ((wide - pad).max(px(1.)) / per_char) as usize);
        let said = self.pieces.iter().find(|piece| piece.kind == LifecyclePieceKind::Rule && sentence.is_rule(&piece.id)).and_then(|piece| piece.unfit.as_ref());
        let refused = if said.is_some() { look.length(token::LEADING_XS) * 2. + look.length(token::SPACE_2) } else { px(0.) };
        pad + look.length(token::H_CONTROL_SM) * lines + look.length(token::SPACE_2) * (lines - 1.) + refused
    }

    /// The chip the open rule stands on, as the canvas drew it last.
    fn chip_of_open(&self) -> Option<Bounds<Pixels>> {
        let sentence = self.sentence.as_ref()?;
        match &self.picked {
            Some(Pick::Rule(picked)) if sentence.is_rule(picked) => self.state.picked_bounds(&self.id),
            _ => None,
        }
    }

    fn sentence_wide(&self, look: &Look, area: Bounds<Pixels>) -> Pixels {
        (look.length(token::SPACE_12) * 16.).min(area.size.width.max(px(0.)))
    }

    fn sentence_area(&self, look: &Look) -> Bounds<Pixels> {
        let seen = self.seen.get();
        let gap = look.length(token::SPACE_2);
        Bounds { origin: point(seen.left() + gap, seen.top() + gap), size: gpui::size(seen.size.width - gap * 2., seen.size.height - gap * 2.) }
    }

    /// What the page shows when the sentence opens, relative to the canvas's frame: the chip and the sentence under it.
    pub(super) fn sentence_reveal(&self, look: &Look) -> Option<Bounds<Pixels>> {
        self.sentence.as_ref()?;
        let frame = self.frame.get();
        let vocab = self.vocab();
        let area = self.sentence_area(look);
        let tall = self.sentence_tall(look, self.sentence_wide(look, area), &vocab);
        let gap = look.length(token::SPACE_2);
        Some(match self.chip_of_open() {
            Some(chip) => Bounds { origin: point(chip.left() - frame.left(), chip.top() - frame.top()), size: gpui::size(chip.size.width, chip.size.height + gap + tall) },
            None => Bounds { origin: point(area.left() - frame.left(), area.top() - frame.top()), size: gpui::size(area.size.width, tall) },
        })
    }

    fn field(&self, look: &Look, slot: Slot, text: Option<String>, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(sentence) = &self.sentence else { return div().into_any_element() };
        let open = sentence.menu.as_ref().is_some_and(|menu| menu.slot == slot);
        let name = name_of(slot);
        let said = text.as_ref().map(|said| SharedString::from(rule::short(said, FIELD_CHARS)));
        let id = SharedString::from(format!("rule-field:{name}"));
        let field = div()
            .id(id)
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL_SM))
            .px(look.length(token::SPACE_2))
            .min_w(look.length(token::SPACE_12) * 2.)
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if open { token::ACCENT } else { token::BORDER_STRONG }))
            .when(said.is_none(), |field| field.border_dashed())
            .bg(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .whitespace_nowrap()
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .children(said.clone())
            .child(div().flex_1())
            .child(svg().flex_none().path(icon_path("chevron-down")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE)));
        let handle = sentence.handle(slot).clone();
        let field = ringed(look, field, &handle, window)
            .track_focus(&handle)
            .when(text.as_ref().is_some_and(|full| said.as_ref().is_some_and(|short| short.as_ref() != full.as_str())), |field| field.tooltip(tip(text.clone().unwrap_or_default())))
            .on_click(cx.listener(move |this, _, window, cx| {
                if this.sentence.as_ref().and_then(|sentence| sentence.menu.as_ref()).is_some_and(|menu| menu.slot == slot) {
                    this.close_menu(window, cx);
                } else {
                    this.open_menu(slot, window, cx);
                }
            }));
        labelled(field, Role::ComboBox, name).into_any_element()
    }

    fn word(&self, look: &Look, said: &'static str) -> AnyElement {
        div().flex_none().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_MUTED)).child(said).into_any_element()
    }

    /// The menu under the open field: where the field was drawn last, over everything.
    fn render_menu(&self, look: &Look, window: &mut Window, cx: &Context<Self>) -> Option<AnyElement> {
        let sentence = self.sentence.as_ref()?;
        let menu = sentence.menu.as_ref()?;
        let name = name_of(menu.slot);
        let Some(field) = crate::settings::view::bounds_of(window, cx, Role::ComboBox, &name) else {
            // The field is drawn this frame for the first time: the menu follows on the next one.
            window.on_next_frame(|window, _| window.refresh());
            return None;
        };
        let at = point(field.left(), field.bottom() + look.length(token::SPACE_1));
        let title = menu.title.as_ref().map(|title| {
            div().px(look.length(token::SPACE_2)).h(look.length(token::H_CONTROL_SM)).flex().items_center().text_size(look.length(token::TEXT_2XS)).font_family(crate::settings::view::mono(look)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(title.clone()))
        });
        let body: Vec<AnyElement> = match &menu.typed {
            Some(typed) => vec![div()
                .px(look.length(token::SPACE_1))
                .pb(look.length(token::SPACE_1))
                .w(look.length(token::SPACE_12) * 6.)
                .child(labelled(div().id("rule-words").key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).child(Input::new(&typed.input)), Role::TextInput, typed.name))
                .into_any_element()],
            None => menu
                .shown
                .iter()
                .enumerate()
                .map(|(n, one)| {
                    let chained = !matches!(one.next, Next::Set(_));
                    let label = SharedString::from(one.label.clone());
                    let row = div()
                        .id(SharedString::from(format!("rule-choice:{n}")))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(look.length(token::SPACE_3))
                        .h(look.length(token::H_CONTROL_SM))
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_SM))
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_DEFAULT))
                        .whitespace_nowrap()
                        .cursor_pointer()
                        .when(menu.at == n, |row| row.bg(look.color(token::BG_HOVER)))
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _, window, cx| this.choose(n, window, cx)))
                        .child(label.clone())
                        .children(chained.then(|| svg().flex_none().path(icon_path("chevron-right")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE))));
                    labelled(row, Role::MenuItem, label).into_any_element()
                })
                .collect(),
        };
        let list = div()
            .id("rule-menu")
            .occlude()
            .min_w(look.length(token::W_MENU))
            .max_h(look.length(token::SPACE_12) * 8.)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .p(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(title)
            .children(body);
        Some(deferred(anchored().position(at).anchor(Anchor::TopLeft).snap_to_window_with_margin(look.length(token::SPACE_1)).child(labelled(list, Role::Menu, format!("{name} choices")))).with_priority(2).into_any_element())
    }

    /// The sentence over the canvas, under the chip it opened from, or at the top of the canvas where it is new.
    pub(super) fn render_sentence(&self, frame: Bounds<Pixels>, look: &Look, window: &mut Window, cx: &Context<Self>) -> Option<AnyElement> {
        let sentence = self.sentence.as_ref()?;
        if frame.size.width <= px(0.) || self.seen.get().size.width <= px(0.) {
            return None;
        }
        let vocab = self.vocab();
        let area = self.sentence_area(look);
        let wide = self.sentence_wide(look, area);
        let tall = self.sentence_tall(look, wide, &vocab);
        let gap = look.length(token::SPACE_2);
        let clamp = |at: Pixels, low: Pixels, high: Pixels| if at > high { high } else { at }.max(low);
        // Under the chip where there is room, else over it, else over the lane from the top; new, at the top of the canvas.
        let panel = div().absolute().w(wide);
        let panel = match self.chip_of_open() {
            Some(chip) => {
                let x = clamp(chip.left(), area.left(), area.right() - wide) - frame.left();
                let below = area.bottom() - (chip.bottom() + gap);
                let above = (chip.top() - gap) - area.top();
                if below >= tall || above < tall {
                    panel.left(x).top(clamp(chip.bottom() + gap, area.top(), (area.bottom() - tall).max(area.top())) - frame.top())
                } else {
                    panel.left(x).bottom(frame.bottom() - (chip.top() - gap))
                }
            }
            None => panel.left(area.left() - frame.left()).top(area.top() - frame.top()),
        };
        let mut row = div().flex().flex_wrap().items_center().gap_x(look.length(token::SPACE_2)).gap_y(look.length(token::SPACE_2));
        let pair = |word: AnyElement, field: AnyElement| div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).child(word).child(field);
        row = row.child(pair(self.word(look, "When"), self.field(look, Slot::When, sentence.text(Slot::When, &vocab), window, cx)));
        for at in 0..sentence.ifs.len() {
            row = row.child(pair(self.word(look, if at == 0 { "if" } else { "and" }), self.field(look, Slot::If(at), sentence.text(Slot::If(at), &vocab), window, cx)));
        }
        let add = {
            let button = crate::settings::view::ghost_with(look, "rule-add-condition", Some("plus"), "Add condition", false);
            let button = ringed(look, button, &sentence.handles.add_if, window).track_focus(&sentence.handles.add_if).on_click(cx.listener(|this, _, window, cx| this.add_condition(window, cx)));
            labelled(button, Role::Button, "Add condition")
        };
        row = row.child(add);
        for at in 0..sentence.dos.len() {
            row = row.child(pair(self.word(look, if at == 0 { "do" } else { "then" }), self.field(look, Slot::Do(at), sentence.text(Slot::Do(at), &vocab), window, cx)));
        }
        let on = sentence.asking(&vocab);
        let ask = {
            let track = switch_track(look, on, false);
            let row = div()
                .id("rule-ask")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_MUTED))
                .cursor_pointer()
                .child("Ask me first")
                .child(track);
            let row = ringed(look, row, &sentence.handles.ask, window).track_focus(&sentence.handles.ask).on_click(cx.listener(|this, _, window, cx| this.flip_ask(window, cx)));
            labelled(row, Role::Switch, "Ask me first")
        };
        row = row.child(ask);
        // What Fleet said of the rule's draft, in its own words.
        let unfit = self.pieces.iter().find(|piece| piece.kind == LifecyclePieceKind::Rule && sentence.is_rule(&piece.id)).and_then(|piece| piece.unfit.clone());
        let refused = unfit.map(|said| labelled(refusal(look, said.clone()).id("rule-refused"), Role::Alert, said));
        let menu = self.render_menu(look, window, cx);
        let group = div()
            .id("lanes-rule-sentence")
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(row)
            .children(refused)
            .children(menu);
        let name = if sentence.base.name.is_empty() { "New rule".to_string() } else { sentence.base.name.clone() };
        let panel = panel.child(labelled(group, Role::Group, name));
        Some(div().absolute().top_0().left_0().size_full().overflow_hidden().child(panel).into_any_element())
    }
}
