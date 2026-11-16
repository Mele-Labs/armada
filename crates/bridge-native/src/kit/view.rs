//! Kit's three panes, drawn from what `model.rs` made of Fleet's read: the rail of kinds, the pieces grouped by
//! where they come from, and the piece picked with its acts. Every value is a token.

use gpui::{div, list, prelude::*, px, Div, FontWeight, Hsla, IntoElement, Render, Role, SharedString, Stateful, Window};
use gpui_component::input::Input;

use super::model::{self, Kind, Line, Mark, Read, Removal, Row, KINDS};
use super::{KitScreen, Load, Pane};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::settings::view::{bone, refusal, ring_colour, switch_track, tip};
use crate::titlebar::parts::{glyph, kbd, mono, sans, Btn, Weight};

/// What an off Edit and Open say until Fleet can read and write the file.
const NEEDS_FILE_OPS: &str = "Needs Fleet's Kit file ops";

/// A hue as a tint: the ground a filled chip and a kind's tile wear (`color-mix(... 22%, transparent)`).
fn tint(look: &Look, hue: Token) -> Hsla {
    look.color(hue).opacity(0.22)
}

/// A filled chip: its words in a hue on that hue's tint.
fn chip(look: &Look, hue: Token, words: String, caps: bool) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .bg(tint(look, hue))
        .text_color(look.color(hue))
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .whitespace_nowrap()
        .child(SharedString::from(if caps { words.to_uppercase() } else { words }))
}

/// The hue a group's chip wears.
fn mark_hue(mark: &Mark) -> Token {
    match mark {
        Mark::Yours => token::ACCENT,
        Mark::Plugin(_) => token::KIND_GATE,
        Mark::Repository => token::KIND_SETUP,
        Mark::Armada => token::OPERATOR,
        Mark::Plain(_) => token::FG_MUTED,
    }
}

/// A square tile with a registry glyph in a hue: a kind's mark in the rail, the list and the detail's head.
fn tile(look: &Look, hue: Token, name: &str) -> Div {
    let size = look.length(token::SPACE_6);
    div().flex_none().flex().items_center().justify_center().size(size).rounded(look.length(token::RADIUS_MD)).bg(tint(look, hue)).child(glyph(name, look.length(token::SPACE_4) - px(2.), look.color(hue)))
}

/// A button in the detail's head: short words, a line round the one that matters, dimmed when it cannot be pressed.
fn button(look: &Look, id: impl Into<gpui::ElementId>, words: &str, line: bool, off: bool) -> Stateful<Div> {
    div()
        .id(id.into())
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if line { look.color(token::BORDER_DEFAULT) } else { gpui::transparent_black() })
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(if line { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) })
        .when(off, |button| button.opacity(0.35))
        .when(!off, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
        .child(SharedString::from(words.to_string()))
}

impl KitScreen {
    fn scope_name(&self) -> String {
        if self.scope.name.is_empty() {
            "this project".into()
        } else {
            self.scope.name.clone()
        }
    }

    // ---- the rail ----------------------------------------------------------------------------

    fn render_rail(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let focused = self.panes[0].is_focused(window);
        let key = crate::generated::actions::ACTIONS.iter().find(|act| act.id == "open_kit").map(|act| act.shortcut).unwrap_or_default();
        let heading = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .pt(look.length(token::SPACE_1))
            .pb(look.length(token::SPACE_3))
            .child(labelled(
                div().id("kit-title").text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Kit"),
                Role::Heading,
                "Kit",
            ))
            .when(!key.is_empty(), |row| row.child(kbd(key, look)));
        let mut kinds = div().flex().flex_col().gap(px(2.));
        for kind in KINDS {
            let on = kind == self.kind;
            let row = div()
                .id(SharedString::from(format!("kit-kind-{}", kind.id())))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .min_h(look.length(token::H_CONTROL_LG))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if on { self.selection(look, focused) } else { gpui::transparent_black() })
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .when(on, |row| row.bg(look.color(token::SURFACE_ROW_SELECTED)).text_color(look.color(token::FG_DEFAULT)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                .when(!on, |row| row.text_color(look.color(token::FG_MUTED)).cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                .aria_selected(on)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.select_kind(kind, cx);
                    this.focus(Pane::Rail, window, cx);
                }))
                .child(tile(look, kind.hue(), kind.glyph()))
                .child(div().min_w_0().truncate().child(kind.label()));
            kinds = kinds.child(labelled(row, Role::ListBoxOption, kind.label()));
        }
        let caption = div().px(look.length(token::SPACE_2)).pt(look.length(token::SPACE_3)).pb(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child("In Settings");
        let mut links = div().flex().flex_col().gap(px(2.));
        for (id, page, label, mark, hue) in [("models", "kit:models", "Models", "cpu", token::STATUS_ESCALATED), ("prompts", "kit:prompts", "Prompts", "file", token::FG_MUTED), ("workflows", "kit:workflows", "Workflows", "git-branch", token::FG_MUTED)] {
            let row = div()
                .id(SharedString::from(format!("kit-settings-{id}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .min_h(look.length(token::H_CONTROL_LG))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip(format!("Open {label} in Settings")))
                .on_click(cx.listener(move |this, _, _, cx| this.settings_page(page, cx)))
                .child(tile(look, hue, mark))
                .child(div().flex_1().min_w_0().truncate().child(label))
                .child(glyph("chevron-right", look.length(token::SPACE_4), look.color(token::FG_SUBTLE)));
            links = links.child(labelled(row, Role::Button, format!("{label} in Settings")));
        }
        labelled(
            div()
                .id("kit-rail")
                .track_focus(&self.panes[0])
                .flex_none()
                .w(look.length(token::W_SETTINGS_INDEX))
                .h_full()
                .flex()
                .flex_col()
                .py(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_2))
                .border_r(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_SUNKEN))
                .child(heading)
                .child(labelled(kinds.id("kit-kinds"), Role::ListBox, "Kinds"))
                .child(caption)
                .child(links),
            Role::Region,
            "Kit kinds",
        )
    }

    /// The edge a picked row wears: the focus colour where the keys are, the accent where they are not.
    fn selection(&self, look: &Look, focused: bool) -> Hsla {
        if focused {
            ring_colour(look)
        } else {
            look.color(token::ACCENT).opacity(0.5)
        }
    }

    // ---- the list ----------------------------------------------------------------------------

    fn render_list(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let add = Btn::new("kit-add-button", "Add", Weight::Primary).icon("plus").draw(look, cx).tooltip(tip("Paste a link to a plugin or a skill. Armada reads it and shows what installing it runs first.")).on_click(cx.listener(|this, _, window, cx| this.start_add(window, cx)));
        let top = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(div().id("kit-filter").flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_SM)).child(Input::new(&self.filter)), Role::SearchInput, format!("Filter {}", self.kind.label())))
            .child(labelled(add, Role::Button, "Add"));
        let body = match (&self.inventory, &self.read) {
            (Load::Reading, _) => labelled(self.render_bones(look).id("kit-loading"), Role::Image, "Loading").into_any_element(),
            (Load::Failed(why), _) => note(look, "Not read", why),
            (_, Read::NotRead(why)) => note(look, "Not read", why),
            _ => {
                let focused = self.panes[1].is_focused(window);
                labelled(
                    div().id("kit-pieces").size_full().child(list(self.list.clone(), cx.processor(move |this, ix: usize, window, cx| this.render_line(ix, focused, window, cx))).size_full()),
                    Role::ListBox,
                    self.kind.label(),
                ).into_any_element()
            }
        };
        labelled(
            div()
                .id("kit-list")
                .track_focus(&self.panes[1])
                .flex_none()
                .w(look.length(token::W_GUIDE_LIST))
                .h_full()
                .flex()
                .flex_col()
                .border_r(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_BASE))
                .child(top)
                .child(div().flex_1().min_h_0().child(body)),
            Role::Region,
            self.kind.label(),
        )
    }

    fn render_bones(&self, look: &Look) -> Div {
        let mut bones = div().flex().flex_col().gap(look.length(token::SPACE_2)).p(look.length(token::SPACE_2)).pt(look.length(token::SPACE_3));
        for n in 0..5 {
            bones = bones.child(
                div()
                    .id(SharedString::from(format!("kit-bone-{n}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_RAISED))
                    .child(bone(look, px(8.), px(8.)).rounded_full())
                    .child(div().flex().flex_col().gap(look.length(token::SPACE_1)).child(bone(look, px(96.), look.length(token::LEADING_XS))).child(bone(look, px(168.), look.length(token::LEADING_2XS)))),
            );
        }
        bones
    }

    fn render_line(&mut self, ix: usize, focused: bool, window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let look = cx.look().clone();
        let look = &look;
        match self.listed.lines.get(ix).cloned() {
            Some(Line::Head(mark)) => {
                let label = mark.label();
                labelled(
                    div().id(SharedString::from(format!("kit-group-{label}"))).flex().items_center().px(look.length(token::SPACE_3)).pt(look.length(token::SPACE_3)).pb(look.length(token::SPACE_1)).child(chip(look, mark_hue(&mark), label.clone(), true)),
                    Role::Heading,
                    label,
                )
                .into_any_element()
            }
            Some(Line::Row(at)) => {
                let Some(shown) = self.listed.shown.get(at).cloned() else { return div().into_any_element() };
                self.render_piece(&shown, focused, look, window, cx)
            }
            None => div().into_any_element(),
        }
    }

    /// One piece as a rounded card: its lamp, its name and its one line. A removed one stays amber and struck through.
    fn render_piece(&self, shown: &model::Shown, focused: bool, look: &Look, _window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let row = &shown.row;
        let picked = Some(&row.id) == self.selected.as_ref();
        let gone = shown.gone.is_some();
        let off = self.off_here();
        let lamp = row.lamp(&off, &self.scope);
        let id = row.id.clone();
        let says = match &shown.gone {
            Some(Some(_)) => "Removed, in the Trash. ⌘Z restores".to_string(),
            Some(None) => "Removed".to_string(),
            None => row.says.clone().unwrap_or_default(),
        };
        let slot = look.length(token::SPACE_4);
        let lamp_mark = match lamp {
            Some(true) if !gone => div().size(px(8.)).rounded_full().bg(look.color(token::STATUS_COMPLETED_SUCCESS)).border(px(3.)).border_color(look.color(token::STATUS_COMPLETED_SUCCESS_BG)),
            Some(_) => div().size(px(8.)).rounded_full().border(px(1.5)).border_color(look.color(token::STATUS_NOT_STARTED)),
            None => div(),
        };
        let lamp_said = match (lamp, row.kind) {
            (_, _) if gone => String::new(),
            (Some(true), Kind::OutputStyles) => "In force".to_string(),
            (Some(false), Kind::OutputStyles) => "Available".to_string(),
            (Some(true), Kind::Skills) if row.jobs.is_none() => "On in every project".to_string(),
            (Some(true), _) => format!("On in {}", self.scope_name()),
            (Some(false), Kind::Servers) if row.server.is_none() => "Reaches no Drone".to_string(),
            (Some(false), _) => format!("Off in {}", self.scope_name()),
            (None, _) => String::new(),
        };
        let lamp_cell = div().id(SharedString::from(format!("kit-lamp-{id}"))).flex_none().w(slot).flex().items_center().justify_center().when(!lamp_said.is_empty(), |cell| cell.tooltip(tip(lamp_said.clone()))).child(lamp_mark);
        let lamp_cell = if lamp_said.is_empty() { lamp_cell.into_any_element() } else { labelled(lamp_cell, Role::Image, lamp_said).into_any_element() };
        let mut name = div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).min_w_0().truncate();
        name = if gone { name.line_through().text_color(look.color(token::FG_MUTED)) } else if lamp == Some(false) && row.kind == Kind::Skills { name.text_color(look.color(token::FG_MUTED)) } else { name.text_color(look.color(token::FG_DEFAULT)) };
        let restore = matches!(shown.gone, Some(Some(_))).then(|| {
            let press = id.clone();
            labelled(
                button(look, SharedString::from(format!("kit-restore-{id}")), "Restore", true, false)
                    .tooltip(tip("Put it back from the Trash"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.pick(press.clone(), cx);
                        this.restore(cx);
                    })),
                Role::Button,
                format!("Restore {}", row.name),
            )
        });
        let card = div()
            .id(SharedString::from(format!("kit-piece-{id}")))
            .mx(look.length(token::SPACE_2))
            .my(px(2.))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .map(|card| {
                if gone {
                    card.border_color(look.color(token::STATUS_AWAITING_REVIEW)).bg(look.color(token::STATUS_AWAITING_REVIEW_BG))
                } else if picked {
                    card.border_color(self.selection(look, focused)).bg(look.color(token::ACCENT).opacity(0.12))
                } else {
                    card.border_color(look.color(token::BORDER_SUBTLE)).bg(look.color(token::BG_RAISED)).cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER)))
                }
            })
            .aria_selected(picked)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.pick(id.clone(), cx);
                this.focus(Pane::List, window, cx);
            }))
            .child(lamp_cell)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(name.child(SharedString::from(row.name.clone())))
                    .child(div().truncate().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(says))),
            )
            .children(restore);
        labelled(card, Role::ListBoxOption, row.name.clone()).into_any_element()
    }

    // ---- the detail --------------------------------------------------------------------------

    fn render_detail(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let focused = self.panes[2].is_focused(window);
        let inside = match (&self.inventory, self.picked()) {
            (Load::Reading, _) => self.detail_bones(look).into_any_element(),
            (_, Some(shown)) => self.piece_detail(shown, look, cx).into_any_element(),
            _ => div().flex_1().into_any_element(),
        };
        labelled(
            div()
                .id("kit-detail")
                .track_focus(&self.panes[2])
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .bg(look.color(token::BG_BASE))
                .when(focused, |detail| detail.border_t(look.length(token::BORDER_WIDTH) * 2.).border_color(ring_colour(look)))
                .child(inside)
                .child(self.render_keys(look)),
            Role::Region,
            "Detail",
        )
    }

    fn detail_bones(&self, look: &Look) -> Div {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .p(look.length(token::SPACE_4))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_RAISED))
                    .child(bone(look, look.length(token::SPACE_6), look.length(token::SPACE_6)))
                    .child(bone(look, px(144.), look.length(token::LEADING_LG))),
            )
            .child(div().flex_1().bg(look.color(token::BG_SUNKEN)).p(look.length(token::SPACE_3)).flex().flex_col().gap(look.length(token::SPACE_2)).children((0..4).map(|n| bone(look, px(120. + 48. * (n % 3) as f32), look.length(token::LEADING_2XS)))))
    }

    fn piece_detail(&self, shown: &model::Shown, look: &Look, cx: &mut gpui::Context<Self>) -> Div {
        let row = &shown.row;
        let gone = shown.gone.is_some();
        let off = self.off_here();
        let on = row.lamp(&off, &self.scope).unwrap_or(false);
        let name = SharedString::from(row.name.clone());
        let label = self.scope_name();

        // The switch: its own fixed place, empty where the kind has none.
        let switch = if row.switchable() && !gone {
            let track = div()
                .id("kit-project-switch")
                .flex_none()
                .rounded_full()
                .cursor_pointer()
                .tooltip(tip(format!("Jobs in {label} may use it")))
                .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
                .aria_toggled(if on { gpui::accesskit::Toggled::True } else { gpui::accesskit::Toggled::False })
                .child(switch_track(look, on, false));
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(look.length(token::SPACE_2))
                .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).whitespace_nowrap().child(SharedString::from(format!("In {label}"))))
                .child(labelled(track, Role::Switch, format!("In {label}")))
                .into_any_element()
        } else {
            div().into_any_element()
        };
        let switch = div().flex_none().w(look.length(token::SPACE_12) * 3.).flex().justify_end().child(switch);

        let mut acts = div().flex().items_center().gap(look.length(token::SPACE_1));
        if row.opens().is_some() && !gone {
            for (id, words) in [("edit", "Edit"), ("open", "Open")] {
                acts = acts.child(labelled(button(look, SharedString::from(format!("kit-{id}")), words, id == "edit", true).tooltip(tip(NEEDS_FILE_OPS)), Role::Button, format!("{words} {name}")));
            }
        }
        if row.path.is_some() && row.kind == Kind::Plugins && !gone {
            acts = acts.child(labelled(button(look, "kit-finder", "Show in Finder", false, false).tooltip(tip("Show the plugin's folder in Finder")).on_click(cx.listener(|this, _, _, cx| this.show_in_finder(cx))), Role::Button, format!("Show {name} in Finder")));
        }
        if !gone {
            acts = match row.removal() {
                Removal::Locked(plugin) => {
                    let said = format!("Comes with the {plugin} plugin. Remove the plugin under Plugins.");
                    acts.child(labelled(
                        div().id("kit-locked").flex_none().flex().items_center().justify_center().h(look.length(token::H_CONTROL_SM)).px(look.length(token::SPACE_3)).tooltip(tip(said.clone())).child(glyph("lock", look.length(token::SPACE_4) - px(2.), look.color(token::FG_SUBTLE))),
                        Role::Image,
                        said,
                    ))
                }
                Removal::None => acts,
                _ => acts.child(labelled(button(look, "kit-remove", "Remove", false, false).on_click(cx.listener(|this, _, _, cx| this.remove(cx))), Role::Button, format!("Remove {name}"))),
            };
        } else if matches!(shown.gone, Some(Some(_))) {
            acts = acts.child(labelled(button(look, "kit-restore", "Restore", true, false).tooltip(tip("Put it back from the Trash")).on_click(cx.listener(|this, _, _, cx| this.restore(cx))), Role::Button, format!("Restore {name}")));
        }

        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_4))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .child(tile(look, row.kind.hue(), row.kind.glyph()))
            .child(labelled(
                div().id("kit-piece-name").min_w_0().truncate().text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(name.clone()),
                Role::Heading,
                name.clone(),
            ))
            .child(chip(look, mark_hue(&row.mark), row.mark.label(), false))
            .child(div().flex_1())
            .child(switch)
            .child(acts);

        let home = match &self.source {
            super::Source::Mock(_) => Some(super::mock::USER.to_string()),
            super::Source::Live(_) => std::env::var("HOME").ok(),
        };
        let place = row.opens().or_else(|| row.path.clone()).or_else(|| row.server.as_ref().map(|server| server.address.clone())).map(|path| model::tilded(&path, home.as_deref()));
        let meta = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_SUBTLE))
            .children(place.map(|place| labelled(div().id("kit-path").min_w_0().truncate().child(SharedString::from(place.clone())), Role::Label, place)));

        let said = self.refused.clone().map(|said| labelled(refusal(look, said.clone()).id("kit-refused").px(look.length(token::SPACE_4)).py(look.length(token::SPACE_2)), Role::Alert, said));

        div().flex_1().min_h_0().flex().flex_col().child(head).child(meta).children(said).child(self.render_file(row, look))
    }

    /// The file's text in a sunken pane with line numbers, as Fleet's read has it: the file itself where Fleet has handed it
    /// over, else the piece's own words.
    fn render_file(&self, row: &Row, look: &Look) -> impl IntoElement {
        let text = row.file.clone().or_else(|| match &self.source {
            super::Source::Mock(kit) => kit.files.get(&row.id).cloned(),
            super::Source::Live(_) => None,
        });
        let text = text.or_else(|| row.says.clone()).unwrap_or_default();
        let lines: Vec<String> = if text.is_empty() { Vec::new() } else { text.lines().map(str::to_string).collect() };
        let numbers = div()
            .flex_none()
            .flex()
            .flex_col()
            .items_end()
            .min_w(look.length(token::SPACE_8) - look.length(token::SPACE_1))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_3))
            .border_r(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_color(look.color(token::FG_SUBTLE))
            .children((1..=lines.len()).map(|n| div().child(SharedString::from(n.to_string()))));
        let mut fenced = false;
        let mut body = div().flex_1().min_w_0().flex().flex_col().p(look.length(token::SPACE_3)).text_color(look.color(token::FG_DEFAULT));
        for (n, line) in lines.iter().enumerate() {
            // The front matter's fences and keys wear the kind's hue, as the mock's file does.
            let fence = line == "---";
            let lit = fence || (fenced && !fence && line.split(':').next().is_some_and(|key| line.contains(':') && !key.contains(' ')));
            if fence {
                fenced = n == 0 || !fenced;
            }
            let key_end = line.find(':').filter(|_| lit && !fence);
            let hue = look.color(row.kind.hue());
            let mut one = div().id(SharedString::from(format!("kit-line-{n}"))).flex().whitespace_nowrap();
            one = match key_end {
                Some(at) => one.child(div().text_color(hue).child(SharedString::from(line[..=at].to_string()))).child(SharedString::from(line[at + 1..].to_string())),
                None if fence => one.text_color(hue).child("---"),
                None => one.child(SharedString::from(if line.is_empty() { " ".to_string() } else { line.clone() })),
            };
            body = body.child(one);
        }
        labelled(
            div()
                .id("kit-file")
                .flex_1()
                .min_h_0()
                .flex()
                .overflow_y_scroll()
                .bg(look.color(token::BG_SUNKEN))
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(px(20.))
                .child(numbers)
                .child(body),
            Role::Region,
            "File",
        )
    }

    // ---- the keys' strip ---------------------------------------------------------------------

    fn render_keys(&self, look: &Look) -> impl IntoElement {
        let on = format!("On in {}", self.scope_name());
        let item = |keys: &[&str], words: String| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .whitespace_nowrap()
                .children(keys.iter().map(|key| kbd((*key).to_string(), look)))
                .child(SharedString::from(words))
        };
        div()
            .id("kit-keys")
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_4))
            .gap_y(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_SUBTLE))
            .font_family(sans(look))
            .child(item(&["j", "k"], "Move".into()))
            .child(item(&["⇥"], "Next pane".into()))
            .child(item(&["space"], on))
            .child(item(&["e"], "Edit".into()))
            .child(item(&["⌫"], "Remove".into()))
            .child(item(&["⌘Z"], "Restore".into()))
            .child(item(&["/"], "Filter".into()))
            .child(item(&["a"], "Add".into()))
    }
}

/// A heading and a reason, where a kind could not be read.
fn note(look: &Look, heading: &str, why: &str) -> gpui::AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .p(look.length(token::SPACE_4))
        .text_size(look.length(token::TEXT_SM))
        .text_color(look.color(token::FG_MUTED))
        .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(heading.to_string())))
        .child(SharedString::from(why.to_string()))
        .into_any_element()
}

impl Render for KitScreen {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let rail = self.render_rail(&look, window, cx);
        let list = self.render_list(&look, window, cx);
        let detail = self.render_detail(&look, window, cx);
        let add = self.render_add(&look, cx);
        let confirm = self.render_confirm(&look, cx);
        div()
            .id("kit")
            .track_focus(&self.within)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::on_act))
            .relative()
            .size_full()
            .flex()
            .bg(look.color(token::BG_BASE))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(rail)
            .child(list)
            .child(detail)
            .children(add)
            .children(confirm)
    }
}
