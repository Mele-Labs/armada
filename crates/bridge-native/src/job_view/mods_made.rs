//! The mods a Mod Job made, from Fleet's mod list: the rows whose `made_by` is the Job (`mods-made.ts`).
//! Each has the Mods page's switch, and a mod applies as it is saved, so turning one off is the undo.
//! They stand in Done (`ModsMade`, `ModsPanel.tsx`) and set a mod Job's route in Now (`route.ts`). A
//! press on a row opens the mod's panel (`ModPanel`, `shared-panel/mod.tsx`): its kind, its switch,
//! Change it, which starts an empty Mod Job aimed at what the mod is, and where it lives.
//!
//! **Still owed:** Promote in the panel, which is the Mods page's own act and arrives with that page.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{ModAim, ModKind, ModOrigin, ModSummary, SessionId};

use super::aa::switch;
use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// One mod the Job made, as its row reads it.
#[derive(Clone)]
pub(super) struct ModMade {
    pub name: String,
    pub title: String,
    pub kind: ModKind,
    pub enabled: bool,
}

/// What the mod changes, in words (`MOD_KIND_SAID`).
pub(super) fn kind_said(kind: ModKind) -> &'static str {
    match kind {
        ModKind::Theme => "Theme",
        ModKind::Tweak => "Theme tweak",
        ModKind::Layout => "Layout",
        ModKind::Words => "Words",
        ModKind::Fleet => "Fleet",
        ModKind::Tracker => "Tracker",
        ModKind::Commands => "Commands",
        ModKind::Tab => "Tab",
        ModKind::Instrument => "Instrument",
    }
}

/// The kind's glyph, which Kit's Mods list draws too (`GLYPH`, `ModRow.tsx`).
fn kind_glyph(kind: ModKind) -> &'static str {
    match kind {
        ModKind::Theme => "palette",
        ModKind::Tweak => "eclipse",
        ModKind::Layout => "panels-top-left",
        ModKind::Words => "link",
        ModKind::Fleet => "bell",
        ModKind::Tracker => "ticket",
        ModKind::Commands => "command",
        ModKind::Tab => "panel-top",
        ModKind::Instrument => "gauge",
    }
}

/// What a Mod Job aims at to change a mod of this kind: a tweak is laid over a theme, so it is changed as one (`AIM`).
fn aim_of(kind: ModKind) -> ModAim {
    match kind {
        ModKind::Theme | ModKind::Tweak => ModAim::Theme,
        ModKind::Layout => ModAim::Layout,
        ModKind::Words => ModAim::Words,
        ModKind::Fleet => ModAim::Fleet,
        ModKind::Tracker => ModAim::Tracker,
        ModKind::Commands => ModAim::Commands,
        ModKind::Tab => ModAim::Source,
        ModKind::Instrument => ModAim::Instrument,
    }
}

/// The kind as it reads in the words the box opens on (`WORD`).
fn kind_word(kind: ModKind) -> &'static str {
    match kind {
        ModKind::Theme | ModKind::Tweak => "theme",
        ModKind::Layout => "layout",
        ModKind::Words => "words",
        ModKind::Fleet => "Fleet rule",
        ModKind::Tracker => "tracker",
        ModKind::Commands => "Commands",
        ModKind::Tab => "tab",
        ModKind::Instrument => "instrument",
    }
}

/// The words an empty Mod Job's box opens on to change this mod, the caret after the colon (`changeWords`).
fn change_words(title: &str, kind: ModKind) -> String {
    format!("Change the {title} {}: ", kind_word(kind))
}

/// A mod's folder name as a person reads it: `dusk` is Dusk, `built-in:calm` is Calm (`modTitle`).
pub(super) fn mod_title(name: &str) -> String {
    let bare = name.strip_prefix("built-in:").unwrap_or(name);
    let mut chars = bare.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

/// A made mod's title: a tab reads as its tab does, so the card and the tab never disagree (`madeTitle`, `tabTitle`).
fn made_title(name: &str, kind: ModKind) -> String {
    if kind != ModKind::Tab {
        return mod_title(name);
    }
    let words = name.replace(['-', '_'], " ");
    mod_title(words.trim())
}

/// Where a mod's folder is made on this machine (`MODS_FOLDER`).
const MODS_FOLDER: &str = "~/Library/Application Support/Armada/mods";

impl JobView {
    /// Fleet's mod list as it stands, or the mock's.
    fn mod_list(&self, cx: &gpui::App) -> Vec<ModSummary> {
        match self.fleet.as_ref() {
            Some(fleet) => fleet.read(cx).held().mods.clone().unwrap_or_default(),
            None => crate::mock::mods(),
        }
    }

    /// Whether a mod is on: what he pressed, until Fleet's list says otherwise.
    fn mod_on(&self, summary: &ModSummary) -> bool {
        match self.mod_switch.get(&summary.name) {
            Some((was, now)) if *was == summary.enabled => *now,
            _ => summary.enabled,
        }
    }

    /// The Job's mods, oldest first as Fleet lists them (`useModsMade`).
    pub(super) fn mods_made(&self, job: &str, cx: &gpui::App) -> Vec<ModMade> {
        self.mod_list(cx)
            .iter()
            .filter_map(|row| {
                let kind = row.kind?;
                (row.made_by.as_ref().map(SessionId::as_str) == Some(job)).then(|| ModMade { name: row.name.clone(), title: made_title(&row.name, kind), kind, enabled: self.mod_on(row) })
            })
            .collect()
    }

    /// Fleet's own `set_mod_enabled`: the switch moves at once, and a refusal puts it back and says why.
    fn set_mod_enabled(&mut self, name: String, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(was) = self.mod_list(cx).iter().find(|one| one.name == name).map(|one| one.enabled) else { return };
        self.mod_switch.insert(name.clone(), (was, on));
        cx.notify();
        // On mock data there is no Fleet to say otherwise: the switch stands as pressed.
        if self.fleet.is_none() {
            return;
        }
        let body = ipc::encode(&ipc::SetModEnabled { name: name.clone(), enabled: on });
        self.post(
            "/mods/enable".into(),
            body,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                if let Err(why) = answer {
                    this.mod_switch.remove(&name);
                    cx.notify();
                    cx.defer(move |cx| crate::toasts::tell(why, cx));
                }
            }),
        );
    }

    /// Change it: an empty Mod Job aimed at what the mod is, opened with its box holding the words to go on from (`startEmptyModJob`).
    fn change_mod(&mut self, title: String, kind: ModKind, window: &mut Window, cx: &mut Context<Self>) {
        let start = ipc::StartModSession { title: None, model: None, effort: None, aim: Some(aim_of(kind)), note: None, said: None, from_job: None, from_tab: None };
        let words = change_words(&title, kind);
        self.post(
            "/mods/session".into(),
            ipc::encode(&start),
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                let started = answer.and_then(|bytes| ipc::decode::<ipc::ModSessionStarted>("ModSessionStarted", &bytes).map_err(|why| why.to_string()));
                match started {
                    Ok(started) => {
                        let id = started.session_id.as_str().to_string();
                        this.keep_words(&id, &words, cx);
                        this.close_panel(window, cx);
                        this.open_job(id, cx);
                    }
                    Err(why) => cx.defer(move |cx| crate::toasts::tell(format!("Mod Job not started: {why}"), cx)),
                }
            }),
        );
    }

    /// The Job's mods in Done (`ModsMade`): one row each, drawing nothing until the first is listed.
    pub(super) fn mods_made_rows(&self, job: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let made = self.mods_made(job, cx);
        (!made.is_empty()).then(|| labelled(div().id("mods-made").flex().flex_col().gap(look.length(token::SPACE_1)).children(made.into_iter().map(|one| self.mod_row(one, look, cx))), Role::List, "Mods made").into_any_element())
    }

    /// One mod (`ModRow`): its kind's glyph, its name, which opens it, and its switch, which alone
    /// toggles it; then the row's four control cells, kept empty so the switches line up down a list.
    fn mod_row(&self, one: ModMade, look: &Look, cx: &Context<Self>) -> AnyElement {
        let said = kind_said(one.kind);
        let mark = labelled(div().id(SharedString::from(format!("mod-kind:{}", one.name))).flex_none().flex().tooltip(tip(said)).child(glyph(look, kind_glyph(one.kind), Size::Mark, look.color(token::FG_MUTED))), Role::Image, said);
        let (name, title, kind) = (one.name.clone(), one.title.clone(), one.kind);
        let open = labelled(
            div()
                .id(SharedString::from(format!("mod-open:{}", one.name)))
                .flex_1()
                .min_w_0()
                .truncate()
                .cursor_pointer()
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(SharedString::from(one.title.clone()))
                .on_click(cx.listener(move |this, _, window, cx| this.open_mod(name.clone(), title.clone(), kind, window, cx))),
            Role::Button,
            format!("Open {}", one.title),
        );
        let (name, on) = (one.name.clone(), one.enabled);
        let toggle = labelled(switch(look, SharedString::from(format!("mod-switch:{}", one.name)), "", on, false).flex_none().on_click(cx.listener(move |this, _, window, cx| {
            cx.stop_propagation();
            this.set_mod_enabled(name.clone(), !on, window, cx);
        })), Role::Switch, one.title.clone());
        let cell = || div().flex_none().size(look.length(token::H_CONTROL));
        let (name, title, kind) = (one.name.clone(), one.title.clone(), one.kind);
        labelled(
            div()
                .id(SharedString::from(format!("mod-row:{}", one.name)))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .min_w_0()
                .rounded(look.length(token::RADIUS_SM))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .on_click(cx.listener(move |this, _, window, cx| this.open_mod(name.clone(), title.clone(), kind, window, cx)))
                .child(mark)
                .child(div().flex_1().min_w_0().flex().items_center().justify_between().gap(look.length(token::SPACE_4)).child(open).child(toggle))
                .child(cell())
                .child(cell())
                .child(cell())
                .child(cell()),
            Role::Group,
            one.title,
        )
        .into_any_element()
    }

    /// The mod's panel body (`ModPanel`): its kind, the switch and Change it, then Details: what it
    /// is, where it lives, its version, whether it is valid, and the Job that made it.
    pub(super) fn mod_body(&self, name: &str, title: &str, kind: ModKind, look: &Look, cx: &Context<Self>) -> AnyElement {
        let row = self.mod_list(cx).into_iter().find(|one| one.name == name);
        let kind = row.as_ref().and_then(|one| one.kind).unwrap_or(kind);
        let said = kind_said(kind);
        let meta = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_color(look.color(token::FG_MUTED))
            .child(labelled(div().id("mod-panel-kind").flex().tooltip(tip(said)).child(glyph(look, kind_glyph(kind), Size::Mark, look.color(token::FG_MUTED))), Role::Image, said))
            .child(said);
        let toggle = row.as_ref().map(|one| {
            let (name, on) = (one.name.clone(), self.mod_on(one));
            labelled(switch(look, "mod-panel-switch", "Switched on", on, false).on_click(cx.listener(move |this, _, window, cx| this.set_mod_enabled(name.clone(), !on, window, cx))), Role::Switch, "Switched on")
        });
        let change_title = title.to_string();
        let change = labelled(
            div()
                .id("mod-panel-change")
                .flex_none()
                .flex()
                .items_center()
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::ACCENT))
                .text_color(look.color(token::FG_INVERSE))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .cursor_pointer()
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::FG_DEFAULT)))
                .child("Change it")
                .on_click(cx.listener(move |this, _, window, cx| this.change_mod(change_title.clone(), kind, window, cx))),
            Role::Button,
            "Change it",
        );
        let acts = div().flex().items_center().gap(look.length(token::SPACE_3)).children(toggle).child(change);
        let built_in = row.as_ref().is_some_and(|one| one.origin == ModOrigin::BuiltIn);
        let fact = |term: &'static str, value: AnyElement| {
            div()
                .flex()
                .gap(look.length(token::SPACE_3))
                .child(div().flex_none().w(look.length(token::SPACE_12) * 2.).text_color(look.color(token::FG_MUTED)).child(term))
                .child(div().flex_1().min_w_0().child(value))
        };
        let words = |said: String| div().child(SharedString::from(said)).into_any_element();
        let mut facts = vec![fact(
            "Lives in",
            if built_in { words("Built in".into()) } else { div().font_family(mono(look)).child(SharedString::from(format!("{MODS_FOLDER}/{name}"))).into_any_element() },
        )];
        if let Some(version) = row.as_ref().and_then(|one| one.version.clone()) {
            facts.push(fact("Version", div().font_family(mono(look)).child(SharedString::from(version)).into_any_element()));
        }
        if let Some(one) = row.as_ref() {
            facts.push(fact("Valid", words(if one.valid { "Valid".into() } else { one.reason.clone().unwrap_or_else(|| "Not valid".into()) })));
        }
        if let Some(maker) = row.as_ref().and_then(|one| one.made_by.clone()) {
            let id = maker.as_str().to_string();
            let named = self.record(&id, cx).and_then(|job| job.title.clone()).unwrap_or_else(|| "Job".to_string());
            let press = labelled(
                div()
                    .id("mod-panel-maker")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .cursor_pointer()
                    .rounded(look.length(token::RADIUS_SM))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(glyph(look, "box", Size::Small, look.color(token::FG_MUTED)))
                    .child(SharedString::from(named.clone()))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.close_panel(window, cx);
                        this.open_job(id.clone(), cx);
                    })),
                Role::Button,
                named,
            );
            facts.push(fact("Made by", press.into_any_element()));
        }
        let description = row.as_ref().and_then(|one| one.description.clone()).map(|said| div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(said)));
        let details = labelled(div().id("mod-panel-details").flex().flex_col().gap(look.length(token::SPACE_3)).children(description).child(div().flex().flex_col().gap(look.length(token::SPACE_2)).children(facts)), Role::Region, "Details");
        div().flex_1().min_h_0().flex().flex_col().gap(look.length(token::SPACE_4)).px(look.length(token::SPACE_6)).py(look.length(token::SPACE_4)).child(meta).child(acts).child(details).into_any_element()
    }
}
