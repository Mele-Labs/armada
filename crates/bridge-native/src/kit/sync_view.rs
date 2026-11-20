//! What Kit draws of the Mac link: the bar across its top, a kind's switch in the rail, a row's mark and its own
//! switch, and a clash's two sides. Drawn from `sync.rs`; every value is a token.

use chrono::{DateTime, Local};
use gpui::accesskit::Toggled;
use gpui::{div, prelude::*, px, AnyElement, Div, FocusHandle, FontWeight, Role, SharedString, Stateful, Window};

use super::model::{Kind, Row};
use super::sync::{self, Bar, KindCross, Standing, Tint};
use super::{Compared, KitScreen, Load, Stop};
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::settings::view::{bone, ring_colour, switch_track, tip};
use crate::titlebar::parts::{glyph, mono, Btn, Weight};

/// The switch's third look, for a kind that is off with some of its items crossing: the knob in the middle of a
/// muted accent track, the same size as on and off.
fn switch_some(look: &Look) -> Div {
    let edge = look.length(token::BORDER_WIDTH);
    let tall = look.length(token::H_BADGE);
    let pad = edge * 2.;
    let knob = tall - pad * 2.;
    let air = look.length(token::SPACE_1);
    let half = (look.length(token::SPACE_4) + look.length(token::SPACE_2)) / 2.;
    let disc = div().flex_none().size(knob).rounded_full().border(edge * 2.).border_color(look.color(token::ACCENT).opacity(0.6)).bg(look.color(token::FG_DEFAULT));
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(air)
        .h(tall)
        .pl(air)
        .pr(pad - edge)
        .border(edge)
        .border_color(look.color(token::ACCENT))
        .rounded(tall / 2.)
        .bg(look.color(token::ACCENT_MUTED))
        .child(div().flex_none().w(half))
        .child(disc)
        .child(div().flex_none().w(half - air))
}

/// The width of a row's mark: room for `Only on` and a short Mac's name, which a longer one truncates in.
fn mark_width(look: &Look) -> gpui::Pixels {
    look.length(token::SPACE_12) * 2. + look.length(token::SPACE_6)
}

/// A hue's ground, as Kit's chips wear it.
fn tint(look: &Look, hue: Token) -> gpui::Hsla {
    look.color(hue).opacity(0.22)
}

/// A registry glyph, where the chip is a state, and its words on the hue's ground. `Same` recedes: no ground, muted, so the rows that need a look stand out.
fn state_chip(look: &Look, glyph_name: Option<&str>, hue: Token, words: String, quiet: bool) -> Div {
    div()
        .flex_none()
        .min_w_0()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .when(!quiet, |chip| chip.bg(tint(look, hue)))
        .text_color(look.color(hue))
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .whitespace_nowrap()
        .children(glyph_name.map(|name| glyph(name, look.length(token::SPACE_3), look.color(hue))))
        .child(div().min_w_0().truncate().child(SharedString::from(words)))
}

/// A focus ring on a control Tab reached.
fn ringed(look: &Look, element: Stateful<Div>, handle: Option<&FocusHandle>, window: &Window) -> Stateful<Div> {
    match handle {
        Some(handle) => element.track_focus(handle).when(handle.is_focused(window), |element| element.border(look.length(token::FOCUS_RING_WIDTH)).border_color(ring_colour(look))),
        None => element,
    }
}

/// `Changed Nov 18 21:10`: when that Mac first read its version.
fn changed_words(at: Option<&ipc::Instant>, now: DateTime<Local>) -> String {
    match at.and_then(crate::snooze::local_of) {
        Some(at) => format!("Changed {}", crate::settings::time::stamp_of(at, crate::settings::time::held(), now)),
        None => String::new(),
    }
}

impl KitScreen {
    // ---- the bar ---------------------------------------------------------------------------------

    pub(super) fn render_bar(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let lead = |hue: Token| div().flex_none().flex().items_center().justify_center().size(look.length(token::SPACE_6)).rounded(look.length(token::RADIUS_MD)).bg(tint(look, hue)).child(glyph("monitor", look.length(token::SPACE_4), look.color(hue)));
        let words = |title: String, aside: Option<&str>, line: String| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(look.length(token::SPACE_2))
                        .child(labelled(div().id("kit-sync-title").text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).whitespace_nowrap().child(SharedString::from(title.clone())), Role::Heading, title))
                        .children(aside.map(|aside| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).whitespace_nowrap().child(SharedString::from(aside.to_string())))),
                )
                .child(div().truncate().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(line)))
        };
        let row = div().flex_none().flex().items_center().gap(look.length(token::SPACE_3)).px(look.length(token::SPACE_4)).py(look.length(token::SPACE_2)).min_h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_4));
        let (row, name) = match &self.link {
            Load::Reading => (row.child(bone(look, look.length(token::SPACE_6), look.length(token::SPACE_6))).child(div().flex().flex_col().gap(look.length(token::SPACE_1)).child(bone(look, px(160.), look.length(token::LEADING_XS))).child(bone(look, px(280.), look.length(token::LEADING_2XS)))), "Loading".to_string()),
            Load::Failed(why) => (row.child(lead(token::STATUS_COMPLETED_FAILED)).child(words("Not read".into(), Some("over the Mac link"), why.clone())), "Not read over the Mac link".to_string()),
            Load::Read(link) => match sync::bar(link) {
                Bar::Unlinked => {
                    let handle = self.stop_handle(&Stop::Account);
                    let link_a_mac = ringed(look, Btn::new("kit-sync-account", "Link a Mac", Weight::Secondary).draw(look, cx), handle, window)
                        .tooltip(tip("Settings › This Mac › Account"))
                        .on_click(cx.listener(|this, _, _, cx| this.settings_page("fleet:account", cx)));
                    (
                        row.child(lead(token::FG_MUTED)).child(words("No Mac linked".into(), None, "Kit crosses to your other Macs once one is linked.".into())).child(labelled(link_a_mac, Role::Button, "Link a Mac in Settings")),
                        "No Mac linked".to_string(),
                    )
                }
                Bar::Linked { to, crossed, apart } => {
                    let takes = if crossed.len() > 1 { "switch" } else { "switches" };
                    let now = Local::now();
                    let mut row = row.child(lead(token::STATUS_HANDED_IN)).child(words(format!("Crosses to {to}"), Some("over the Mac link"), format!("{to} {takes} on the same kinds to take them.")));
                    if let Some(apart) = apart {
                        let (word, said) = sync::apart_said(&apart);
                        let (name, hue) = if apart.clash { ("split", token::STATUS_AWAITING_REVIEW) } else { ("git-compare", token::STATUS_RUNNING) };
                        row = row.child(labelled(state_chip(look, Some(name), hue, word.into(), false).id("kit-sync-apart").tooltip(tip(said.clone())), Role::Image, format!("{word} over the Mac link")));
                    }
                    for (mac, at) in &crossed {
                        let when = at.as_ref().and_then(|at| crate::job_view::age(at.as_str(), now));
                        let words = match (&when, crossed.len() > 1) {
                            (Some(when), false) => format!("Crossed {when}"),
                            (Some(when), true) => format!("{mac} crossed {when}"),
                            (None, false) => "Not crossed yet".to_string(),
                            (None, true) => format!("{mac} not crossed yet"),
                        };
                        let hue = if when.is_some() { token::STATUS_COMPLETED_SUCCESS } else { token::FG_MUTED };
                        let full = at.as_ref().and_then(crate::snooze::local_of).map_or_else(|| format!("{mac}'s Kit has not crossed to this Mac"), |at| format!("{mac}'s Kit last crossed at {}", crate::settings::time::stamp_of(at, crate::settings::time::held(), now)));
                        row = row.child(labelled(state_chip(look, None, hue, words.clone(), when.is_none()).id(SharedString::from(format!("kit-sync-crossed-{mac}"))).tooltip(tip(full)), Role::Status, words));
                    }
                    (row, format!("Crosses to {to}"))
                }
            },
        };
        labelled(crate::reviews::panel(look).id("kit-sync-bar").flex_none().child(row), Role::Region, name).into_any_element()
    }

    // ---- the rail ---------------------------------------------------------------------------------

    /// A kind's switch over the link, in a fixed place at the row's end: on, off, or some where the kind is off and
    /// some of its items cross by themselves. Empty for a kind that does not cross.
    pub(super) fn kind_switch(&self, kind: Kind, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let cell = div().flex_none().flex().justify_end().w(look.length(token::SPACE_12));
        let Load::Read(link) = &self.link else { return cell.into_any_element() };
        let Some(stands) = sync::kind_cross(link, kind) else { return cell.into_any_element() };
        let label = kind.label();
        let (said, toggled, track) = match stands {
            KindCross::On => (format!("{label} cross to your linked Macs: this Mac sends its own and takes theirs"), Toggled::True, switch_track(look, true, false)),
            KindCross::Partly => ("Some cross".to_string(), Toggled::Mixed, switch_some(look)),
            KindCross::Off => (format!("{label} stay on this Mac"), Toggled::False, switch_track(look, false, false)),
        };
        let track = div()
            .id(SharedString::from(format!("kit-cross-{}", kind.id())))
            .flex_none()
            .rounded_full()
            .cursor_pointer()
            .tooltip(tip(said))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.cross_kind(kind, cx);
            }))
            .aria_toggled(toggled)
            .child(track);
        cell.child(labelled(track, Role::Switch, format!("Cross {label}"))).into_any_element()
    }

    // ---- the list ---------------------------------------------------------------------------------

    /// A row's mark over the link, in its fixed column at the end of the name's line. The column is kept, empty, on
    /// a row of his own that has no mark, and left out on a plugin's or a repository's piece, which never crosses, and
    /// where no row of the list has a mark.
    pub(super) fn mark_cell(&self, row: &Row, look: &Look) -> Option<AnyElement> {
        if self.standings.is_empty() || (!sync::his(row) && !self.standings.contains_key(&row.id)) {
            return None;
        }
        let cell = div().flex_none().w(mark_width(look)).flex().justify_end();
        let Some(standing) = self.standings.get(&row.id) else { return Some(cell.into_any_element()) };
        let word = standing.word();
        let chip = state_chip(look, Some(standing.glyph()), standing.hue(), word.clone(), matches!(standing, Standing::Same(_))).flex_shrink(1.).max_w_full().overflow_hidden().id(SharedString::from(format!("kit-mark-{}", row.id))).tooltip(tip(standing.said()));
        Some(cell.child(labelled(chip, Role::Image, word)).into_any_element())
    }

    /// A row's own switch over the link, in its fixed column at the end of the row's second line, under its mark.
    /// The column is kept, empty, on a row that never crosses, and left out where the kind does not cross. A row
    /// Fleet refuses shows its switch off and unpressable, with Fleet's reason.
    pub(super) fn cross_cell(&self, row: &Row, gone: bool, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> Option<AnyElement> {
        let Load::Read(link) = &self.link else { return None };
        sync::home_kind(row.kind)?;
        let cell = div().flex_none().flex().justify_end().w(look.length(token::SPACE_12));
        if gone || !sync::row_switchable(row) {
            return Some(cell.into_any_element());
        }
        let name = format!("Cross {}", row.name);
        let track = div().id(SharedString::from(format!("kit-cross-row-{}", row.id))).flex_none().rounded_full();
        if let Some(why) = sync::refusal(link, row) {
            let track = track.tooltip(tip(why.to_string())).aria_toggled(Toggled::False).aria_description(why.to_string()).child(switch_track(look, false, true));
            return Some(cell.child(labelled(track, Role::Switch, name)).into_any_element());
        }
        let on = sync::row_crosses(link, row);
        let said = match (&row.elsewhere, on) {
            (_, true) => "Crosses to your linked Macs".to_string(),
            (Some(macs), false) => format!("On {macs}. On, this Mac takes it"),
            (None, false) => "Stays on this Mac".to_string(),
        };
        let id = row.id.clone();
        let track = track
            .cursor_pointer()
            .tooltip(tip(said))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.cross_row(&id, cx);
            }))
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .child(switch_track(look, on, false));
        let picked = Some(&row.id) == self.selected.as_ref();
        let track = if picked { ringed(look, track, self.stop_handle(&Stop::CrossItem), window) } else { track };
        Some(cell.child(labelled(track, Role::Switch, name)).into_any_element())
    }

    // ---- the detail -------------------------------------------------------------------------------

    /// The picked row's clashes: each Mac's side by side, and a button to keep each.
    pub(super) fn render_clash(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let now = Local::now();
        let mut pane = div().id("kit-clash").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap(look.length(token::SPACE_3)).p(look.length(token::SPACE_4)).bg(look.color(token::BG_SUNKEN));
        for one in &self.compared {
            let head = div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_SM))
                .child(glyph("split", look.length(token::SPACE_4), look.color(token::STATUS_AWAITING_REVIEW)))
                .child(SharedString::from(format!("This Mac and {} both changed it since they last agreed.", one.name)));
            let sides = div()
                .flex()
                .gap(look.length(token::SPACE_3))
                .child(self.side(one, true, look, now, window, cx))
                .child(self.side(one, false, look, now, window, cx));
            let foot = div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Each Mac keeps its own until you pick.");
            pane = pane.child(labelled(div().id(SharedString::from(format!("kit-clash-{}", one.mac))).flex().flex_col().gap(look.length(token::SPACE_3)).child(head).child(sides).child(foot), Role::Region, format!("Clash with {}", one.name)));
        }
        pane.into_any_element()
    }

    fn side(&self, one: &Compared, here: bool, look: &Look, now: DateTime<Local>, window: &Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let (who, mark, at, lines) = if here { ("This Mac".to_string(), "laptop", one.here_at.as_ref(), &one.here) } else { (one.name.clone(), "monitor", one.there_at.as_ref(), &one.there) };
        let keep = if here { ipc::KitKeep::Here } else { ipc::KitKeep::There };
        let words = if here { "Keep this Mac's".to_string() } else { format!("Keep {}'s", one.name) };
        let said = if here { format!("{} takes this Mac's version too", one.name) } else { format!("This Mac takes {}'s version", one.name) };
        let stop = Stop::Keep { mac: one.mac.clone(), keep };
        let press = (one.mac.clone(), keep);
        let button = Btn::new(SharedString::from(format!("kit-keep-{}-{}", one.mac, if here { "here" } else { "there" })), words.clone(), if here { Weight::Primary } else { Weight::Secondary }).draw(look, cx);
        let button = ringed(look, button, self.stop_handle(&stop), window).tooltip(tip(said)).on_click(cx.listener(move |this, _, window, cx| this.settle(press.0.clone(), press.1, window, cx)));
        let mut text = div().flex().flex_col().py(look.length(token::SPACE_2)).font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).line_height(px(20.));
        for (tint_of, line) in lines {
            let (sign, hue) = match tint_of {
                Tint::Both => (" ", None),
                Tint::Ours => ("+", Some(token::STATUS_COMPLETED_SUCCESS)),
                Tint::Theirs => ("−", Some(token::STATUS_COMPLETED_FAILED)),
            };
            text = text.child(
                div()
                    .flex()
                    .px(look.length(token::SPACE_2))
                    .when_some(hue, |line, hue| line.bg(look.color(hue).opacity(0.12)))
                    .text_color(hue.map_or(look.color(token::FG_DEFAULT), |hue| look.color(hue)))
                    .child(div().flex_none().w(look.length(token::SPACE_4)).child(sign))
                    .child(div().flex_1().min_w_0().child(SharedString::from(if line.is_empty() { " ".to_string() } else { line.clone() }))),
            );
        }
        let changed = changed_words(at, now);
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::SURFACE_CARD))
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(glyph(mark, look.length(token::SPACE_4), look.color(if here { token::ACCENT } else { token::STATUS_HANDED_IN })))
                    .child(div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(who.clone())))
                    .child(div().flex_1())
                    .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).whitespace_nowrap().child(SharedString::from(changed))),
            )
            .child(labelled(text.id(SharedString::from(format!("kit-side-{}-{}", one.mac, if here { "here" } else { "there" }))), Role::Region, format!("{who}'s version")))
            .child(div().flex().px(look.length(token::SPACE_3)).py(look.length(token::SPACE_2)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(labelled(button, Role::Button, words)))
    }
}
