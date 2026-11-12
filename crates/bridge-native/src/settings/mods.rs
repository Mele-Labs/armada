//! The mods of one kind on this machine, one row each: its kind, whether it is on, what is wrong with it if
//! anything, and Promote. Classic's `ModList` over `ModRow` (`packages/surfaces/settings/src/ModList.tsx`,
//! `packages/components/src/compositions/ModRow/`). Theme lists the theme mods and the tweaks; Layout and Mods
//! list theirs from the same rows.
//!
//! **Not yet here:** a press on a row opening the mod's panel, which the Jobs screen holds, and the shared
//! home's marks, which native does not read.

use std::collections::BTreeMap;

use gpui::{div, prelude::*, svg, AnyElement, App, Context, FocusHandle, KeyDownEvent, Role, SharedString, Window};
use ipc::{ModKind, ModOrigin, ModPromoted, ModSummary};

use super::overlay::SettingsOverlay;
use super::view::{card, ringed, switch_track, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// A mod Armada ships is named `built-in:` and its folder's name; its title is the folder's name.
const BUILT_IN: &str = "built-in:";
pub const BUILT_IN_SAID: &str = "Ships with Armada";

/// What the mod changes, in words (`MOD_KIND_SAID`).
pub fn kind_said(kind: ModKind) -> &'static str {
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

/// The kind's glyph, by its name in the icon registry (`GLYPH`).
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

/// One row as it is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Listed {
    pub name: String,
    pub title: String,
    pub kind: ModKind,
    pub enabled: bool,
    pub built_in: bool,
    /// The one plain fact under the title: what is wrong, the branch it was put on, or what a tweak does.
    pub detail: Option<String>,
    pub promote_off: bool,
    /// The pull request Promote opened, by number, title and address, and the Job holding it, by id and title.
    pub pull: Option<(u64, String, String)>,
    pub job: Option<(String, String)>,
}

/// The rows of `kind`, as Classic's list draws them: a theme mod Armada ships before a local one of the same name,
/// then all by title.
pub fn listed(mods: &[ModSummary], kind: ModKind, promoted: &BTreeMap<String, ModPromoted>, switched: &BTreeMap<String, (bool, bool)>) -> Vec<Listed> {
    let mut rows: Vec<Listed> = mods
        .iter()
        .filter(|row| row.kind.unwrap_or(ModKind::Theme) == kind)
        .map(|row| {
            let built_in = row.origin == ModOrigin::BuiltIn;
            let title = row.name.strip_prefix(BUILT_IN).unwrap_or(&row.name).to_string();
            let promote = promoted.get(&row.name);
            let branch = promote.map(|done| done.branch.clone());
            let (pull, job) = match (&row.pull_request, promote) {
                (Some(open), _) => (Some((open.number, open.title.clone(), open.address.clone())), Some((open.session_id.as_str().to_string(), open.title.clone()))),
                (None, Some(done)) => (done.pull_request.as_ref().and_then(|address| number_of(address).map(|number| (number, done.title.clone(), address.clone()))), Some((done.session_id.as_str().to_string(), done.title.clone()))),
                (None, None) => (None, None),
            };
            let problem = (!row.valid).then(|| row.reason.clone().unwrap_or_else(|| "Did not pass its checks".into()));
            let detail = match kind {
                ModKind::Theme | ModKind::Layout => problem.clone().or_else(|| branch.clone().filter(|_| pull.is_none())),
                _ => {
                    let line = [row.description.clone().filter(|_| row.valid), row.taught_on.clone().filter(|on| !on.is_empty()).map(|on| format!("Taught on {on}"))].into_iter().flatten().collect::<Vec<_>>().join(". ");
                    let line = if row.valid { line } else { problem.clone().unwrap_or_default() };
                    match &branch {
                        Some(branch) => Some(format!("{line} ({branch})")),
                        None => (!line.is_empty()).then_some(line),
                    }
                }
            };
            let enabled = match switched.get(&row.name) {
                Some((was, now)) if *was == row.enabled => *now,
                _ => row.enabled,
            };
            Listed { name: row.name.clone(), title, kind, enabled, built_in, detail, promote_off: problem.is_some() || branch.is_some() || pull.is_some(), pull, job }
        })
        .collect();
    rows.sort_by_key(|one| !one.built_in);
    rows.sort_by(|a, b| a.title.cmp(&b.title));
    rows
}

/// The number at the end of a pull request's address.
fn number_of(address: &str) -> Option<u64> {
    address.trim_end_matches('/').rsplit('/').next()?.parse().ok()
}

/// The mod rows' own state: the switches pressed while Fleet answers, what a Promote opened, and each row's keys.
#[derive(Default)]
pub struct ModRows {
    pub switched: BTreeMap<String, (bool, bool)>,
    pub promoted: BTreeMap<String, ModPromoted>,
    pub promoting: Option<String>,
    pub refused: BTreeMap<String, String>,
    /// Each row's switch, Promote, pull request and Job, by mod name.
    pub keys: BTreeMap<String, [FocusHandle; 4]>,
}

const SWITCH: usize = 0;
const PROMOTE: usize = 1;
const PULL: usize = 2;
const JOB: usize = 3;

impl SettingsOverlay {
    pub fn mod_rows(&self, kind: ModKind, cx: &App) -> Vec<Listed> {
        listed(&self.wire.mods(cx), kind, &self.mods.promoted, &self.mods.switched)
    }

    /// The stops of a kind's rows, in the order Tab reaches them.
    pub fn mod_stops(&self, kind: ModKind, cx: &App) -> Vec<FocusHandle> {
        let mut stops = Vec::new();
        for one in self.mod_rows(kind, cx) {
            let Some(keys) = self.mods.keys.get(&one.name) else { continue };
            stops.push(keys[SWITCH].clone());
            if !one.built_in && !one.promote_off {
                stops.push(keys[PROMOTE].clone());
            }
            if one.pull.is_some() {
                stops.push(keys[PULL].clone());
            }
            if one.job.is_some() {
                stops.push(keys[JOB].clone());
            }
        }
        stops
    }

    /// Make the keys of every row Fleet lists, once each.
    pub fn mod_keys(&mut self, cx: &mut App) {
        for row in self.wire.mods(cx) {
            self.mods.keys.entry(row.name).or_insert_with(|| [cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true)]);
        }
    }

    /// Fleet's `set_mod_enabled`: the switch moves at once, and a refusal puts it back and says why.
    fn switch_mod(&mut self, name: String, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(was) = self.wire.mods(cx).iter().find(|one| one.name == name).map(|one| one.enabled) else { return };
        self.mods.switched.insert(name.clone(), (was, on));
        self.mods.refused.remove(&name);
        let sent = self.wire.set_mod_enabled(&name, on, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update(cx, |this, cx| {
                if let Err(why) = answer {
                    this.mods.switched.remove(&name);
                    this.mods.refused.insert(name, why.said);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Promote: Fleet starts a Job of its own that puts the mod into Armada in a pull request.
    fn promote_mod(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.mods.promoting.is_some() {
            return;
        }
        self.mods.promoting = Some(name.clone());
        self.mods.refused.remove(&name);
        let sent = self.wire.promote_mod(&name, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.mods.promoting = None;
                match answer {
                    Ok(done) => {
                        this.mods.promoted.insert(name.clone(), done);
                        // Promote goes once it has been pressed: the keys go to the row's switch.
                        if let Some(keys) = this.mods.keys.get(&name) {
                            window.focus(&keys[SWITCH], cx);
                        }
                    }
                    Err(why) => {
                        this.mods.refused.insert(name, why.said);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// The Job holding a promote's pull request opens as a press on its row anywhere does.
    fn open_promote_job(id: &str, title: &str, window: &mut Window, cx: &mut App) {
        let Some(open) = cx.try_global::<crate::clash_mark::OpenHolder>().cloned() else { return };
        (open.0)(&ipc::ClashHolder { kind: ipc::ClashSide::Session, id: id.to_string(), label: title.to_string() }, window, cx);
    }

    /// A kind's rows in a card, titled where Classic titles it. Nothing is drawn while there are none.
    pub fn render_mods(&self, kind: ModKind, title: Option<&str>, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let rows = self.mod_rows(kind, cx);
        if rows.is_empty() {
            return None;
        }
        let body: Vec<AnyElement> = rows.into_iter().map(|one| self.render_mod(one, look, window, cx)).collect();
        let id = SharedString::from(format!("card-mods-{}", kind_said(kind)));
        Some(match title {
            Some(title) => card(look, id, title, None, vec![div().flex().flex_col().gap(look.length(token::SPACE_3)).children(body).into_any_element()]).into_any_element(),
            None => div()
                .id(id)
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::SURFACE_CARD))
                .children(body)
                .into_any_element(),
        })
    }

    fn render_mod(&self, one: Listed, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(keys) = self.mods.keys.get(&one.name).cloned() else { return div().into_any_element() };
        let off = !self.live;
        let control = look.length(token::H_CONTROL);
        let glyph = labelled(
            div().id(SharedString::from(format!("mod-kind-{}", one.name))).flex_none().flex().text_color(look.color(token::FG_MUTED)).tooltip(tip(kind_said(one.kind))).child(svg().path(icon_path(kind_glyph(one.kind))).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED))),
            Role::Image,
            kind_said(one.kind),
        );
        let text = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .min_w_0()
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(SharedString::from(one.title.clone()))
            .children(one.detail.clone().map(|line| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(line)));
        let (name, on) = (one.name.clone(), one.enabled);
        let switch = ringed(
            look,
            div()
                .id(SharedString::from(format!("mod-switch-{}", one.name)))
                .flex_none()
                .rounded_full()
                .track_focus(&keys[SWITCH])
                .when(!off, |switch| switch.cursor_pointer())
                .child(switch_track(look, on, off))
                .when(!off, |switch| {
                    let flip = name.clone();
                    switch.on_click(cx.listener(move |this, _, window, cx| this.switch_mod(flip.clone(), !on, window, cx))).on_key_down(cx.listener({
                        let name = name.clone();
                        move |this, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.switch_mod(name.clone(), !on, window, cx);
                                cx.stop_propagation();
                            }
                        }
                    }))
                }),
            &keys[SWITCH],
            window,
        );
        let head = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(look.length(token::SPACE_4))
            .min_w_0()
            .flex_1()
            .child(text)
            .child(labelled(switch, Role::Switch, one.title.clone()));
        let cell = || div().flex_none().w(control).h(control).flex().items_center().justify_center();
        let promote: AnyElement = if one.built_in {
            labelled(cell().id(SharedString::from(format!("mod-shipped-{}", one.name))).tooltip(tip(BUILT_IN_SAID)).child(svg().path(icon_path("package")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED))), Role::Image, BUILT_IN_SAID).into_any_element()
        } else {
            let promote_off = off || one.promote_off || self.mods.promoting.is_some();
            let said = format!("Add {} to Armada in a pull request", one.title);
            let name = one.name.clone();
            labelled(
                ringed(
                    look,
                    cell()
                        .id(SharedString::from(format!("mod-promote-{}", one.name)))
                        .rounded(look.length(token::RADIUS_MD))
                        .track_focus(&keys[PROMOTE])
                        .tooltip(tip(said))
                        .when(!promote_off, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                        .child(svg().path(icon_path("git-pull-request-arrow")).size(look.length(token::SPACE_4)).text_color(if promote_off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) }))
                        .when(!promote_off, |button| {
                            let press = name.clone();
                            button.on_click(cx.listener(move |this, _, window, cx| this.promote_mod(press.clone(), window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.promote_mod(name.clone(), window, cx);
                                    cx.stop_propagation();
                                }
                            }))
                        }),
                    &keys[PROMOTE],
                    window,
                ),
                Role::Button,
                format!("Promote {}", one.title),
            )
            .into_any_element()
        };
        let row = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_w_0()
            .child(glyph)
            .child(head)
            // Details, the shared home's mark and its act: kept empty so Promote keeps its column.
            .child(cell())
            .child(cell())
            .child(cell())
            .child(promote);
        let promoted = (one.pull.is_some() || one.job.is_some()).then(|| {
            let pull = one.pull.clone().map(|(number, title, address)| {
                let open = address.clone();
                labelled(
                    ringed(
                        look,
                        div()
                            .id(SharedString::from(format!("mod-pull-{}", one.name)))
                            .track_focus(&keys[PULL])
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .px(look.length(token::SPACE_2))
                            .py(look.length(token::SPACE_1))
                            .rounded(look.length(token::RADIUS_MD))
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .text_color(look.color(token::FG_MUTED))
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                            .child(svg().path(icon_path("git-pull-request")).flex_none().size(look.length(token::SPACE_3)).text_color(look.color(token::FG_SUBTLE)))
                            .child(div().font_family(super::view::mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(format!("#{number}"))))
                            .child(div().min_w_0().truncate().child(SharedString::from(title.clone())))
                            .on_click(move |_, _, cx| cx.open_url(&open))
                            .on_key_down(move |event: &KeyDownEvent, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    cx.open_url(&address);
                                }
                            }),
                        &keys[PULL],
                        window,
                    ),
                    Role::Button,
                    format!("#{number} {title}"),
                )
            });
            let job = one.job.clone().map(|(id, title)| {
                let (open_id, open_title) = (id.clone(), title.clone());
                labelled(
                    ringed(
                        look,
                        div()
                            .id(SharedString::from(format!("mod-job-{}", one.name)))
                            .track_focus(&keys[JOB])
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .px(look.length(token::SPACE_2))
                            .py(look.length(token::SPACE_1))
                            .rounded(look.length(token::RADIUS_MD))
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .text_color(look.color(token::FG_MUTED))
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                            .child(svg().path(icon_path("box")).flex_none().size(look.length(token::SPACE_3)).text_color(look.color(token::FG_SUBTLE)))
                            .child(div().min_w_0().truncate().child(SharedString::from(title.clone())))
                            .on_click(move |_, window, cx| Self::open_promote_job(&open_id, &open_title, window, cx))
                            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    Self::open_promote_job(&id, &title, window, cx);
                                }
                            }),
                        &keys[JOB],
                        window,
                    ),
                    Role::Button,
                    format!("Job {}", one.title),
                )
            });
            div().flex().flex_col().pl(look.length(token::SPACE_4) + look.length(token::SPACE_3)).children(pull).children(job)
        });
        let refused = self.mods.refused.get(&one.name).cloned().map(|said| labelled(super::view::small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id(SharedString::from(format!("mod-refused-{}", one.name))), Role::Alert, said));
        labelled(div().id(SharedString::from(format!("mod-{}", one.name))).flex().flex_col().gap(look.length(token::SPACE_1)).child(row).children(promoted).children(refused), Role::Group, one.title.clone()).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(json: &str) -> Vec<ModSummary> {
        ipc::decode::<ipc::ModList>("mods", json.as_bytes()).expect("decodes").mods
    }

    #[test]
    fn a_theme_row_says_what_is_wrong_and_shipped_ones_cannot_be_promoted() {
        let mods = rows(r#"{"mods":[
            {"name":"dusk","kind":"theme","enabled":true,"valid":false,"reason":"theme.css names no block for dusk"},
            {"name":"built-in:paper","origin":"built_in","kind":"theme","enabled":false,"valid":true},
            {"name":"calm","kind":"tweak","enabled":true,"valid":true,"description":"Stopped rows lighter","taught_on":"the Jobs list"}
        ]}"#);
        let themes = listed(&mods, ModKind::Theme, &BTreeMap::new(), &BTreeMap::new());
        assert_eq!(themes.iter().map(|one| one.title.as_str()).collect::<Vec<_>>(), ["dusk", "paper"]);
        assert_eq!(themes[0].detail.as_deref(), Some("theme.css names no block for dusk"));
        assert!(themes[0].promote_off);
        assert!(themes[1].built_in);
        let tweaks = listed(&mods, ModKind::Tweak, &BTreeMap::new(), &BTreeMap::new());
        assert_eq!(tweaks[0].detail.as_deref(), Some("Stopped rows lighter. Taught on the Jobs list"));
    }

    #[test]
    fn a_pressed_switch_stands_until_fleet_says_otherwise() {
        let mods = rows(r#"{"mods":[{"name":"dusk","kind":"theme","enabled":true,"valid":true}]}"#);
        let pressed: BTreeMap<String, (bool, bool)> = [("dusk".to_string(), (true, false))].into_iter().collect();
        assert!(!listed(&mods, ModKind::Theme, &BTreeMap::new(), &pressed)[0].enabled);
        let moved = rows(r#"{"mods":[{"name":"dusk","kind":"theme","enabled":false,"valid":true}]}"#);
        let other: BTreeMap<String, (bool, bool)> = [("dusk".to_string(), (false, true))].into_iter().collect();
        assert!(listed(&moved, ModKind::Theme, &BTreeMap::new(), &other)[0].enabled);
    }

    #[test]
    fn what_promote_opened_goes_under_the_row_and_promote_is_off() {
        let mods = rows(r#"{"mods":[{"name":"dusk","kind":"theme","enabled":true,"valid":true}]}"#);
        let done: ModPromoted = ipc::decode("ModPromoted", br#"{"name":"dusk","session_id":"01JOBAAAAAAAAAAAAAAAAAAAAA","title":"Add the dusk theme mod","branch":"armada/mod-dusk","commit":"abc","pull_request":"https://github.com/o/r/pull/42"}"#).expect("decodes");
        let promoted: BTreeMap<String, ModPromoted> = [("dusk".to_string(), done)].into_iter().collect();
        let one = &listed(&mods, ModKind::Theme, &promoted, &BTreeMap::new())[0];
        assert_eq!(one.pull.as_ref().map(|pull| pull.0), Some(42));
        assert_eq!(one.job.as_ref().map(|job| job.1.as_str()), Some("Add the dusk theme mod"));
        assert_eq!(one.detail, None);
        assert!(one.promote_off);
    }
}
