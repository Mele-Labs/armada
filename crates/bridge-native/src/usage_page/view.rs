//! The Usage page drawn from what `model.rs` made: the header band with the window switch, the panel of settings that change the burn,
//! the panel of what each change did, the list of kinds, and under it, where a kind is open, a panel with the kind's fix and the table
//! of the Jobs it hit. Every value is a token.

use gpui::{div, list, prelude::*, px, Div, FontWeight, Hsla, IntoElement, Pixels, Render, Role, SharedString, Window};

use super::model::{shows_causes, Board, ChangeRow, JobLine, KindRow, Note, SettingRow, Shown, Span};
use super::UsageScreen;
use crate::generated::actions::ActionScope;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::reviews::panel;
use crate::settings::view::{bone, refusal, ring_colour, tip};
use crate::titlebar::parts::{glyph, mono, sans, spinner, Btn, Weight};

/// The width of a share column and of the peak column: two and a half steps of the largest space, so the figures line up down every panel.
fn figure_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 2.5
}

/// The width of the Now column of the settings table.
fn now_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 2.
}

/// The width of the dropdown a setting is changed with.
fn pick_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 3.
}

/// The width of the Cuts column: the title of a waste kind.
fn cuts_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 5.
}

/// The width of the column the Apply button stands in.
fn apply_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 2.
}

/// The name of a setting's dropdown, as VoiceOver reads it and as the list under it is found by.
pub fn dropdown_name(label: &str) -> String {
    format!("Change {label} to")
}

/// The width of the Cause column: six steps of the largest space, left-aligned, so the names line up down the table.
fn cause_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 6.
}

/// The square a kind's glyph stands in: the hue at 16% over the ground, the glyph in the hue.
fn chip(look: &Look, hue: Token, glyph_name: &str) -> Div {
    let side = look.length(token::SPACE_6);
    div().flex_none().flex().items_center().justify_center().size(side).rounded(look.length(token::RADIUS_MD)).bg(look.color(hue).opacity(0.16)).child(glyph(glyph_name, look.length(token::SPACE_4), look.color(hue)))
}

/// A figure in its fixed column, right-aligned.
fn figure(look: &Look, said: impl Into<SharedString>, ink: Hsla) -> Div {
    div().flex_none().w(figure_width(look)).flex().justify_end().font_family(mono(look)).text_color(ink).child(said.into())
}

/// The segment a window is drawn as: raised in an accent wash and ring where it is the one read, with a spinner after its word
/// while Fleet has not answered for it.
fn segment(look: &Look, span: Span, on: bool, pending: bool, cx: &mut gpui::Context<UsageScreen>) -> gpui::AnyElement {
    let edge = look.length(token::BORDER_WIDTH);
    let word = span.label();
    let one = div()
        .id(SharedString::from(format!("usage-window-{}", word.to_lowercase())))
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL) - look.length(token::SPACE_1) - edge * 2.)
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(edge)
        .border_color(gpui::transparent_black())
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .when(on, |segment| segment.bg(look.color(token::ACCENT_MUTED)).border_color(look.color(token::ACCENT)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)))
        .when(!on, |segment| segment.text_color(look.color(token::FG_MUTED)).cursor_pointer().hover(|style| style.text_color(look.color(token::FG_DEFAULT))))
        .aria_toggled(if on { gpui::Toggled::True } else { gpui::Toggled::False })
        .on_click(cx.listener(move |this, _, _, cx| this.pick(span, cx)))
        .child(word)
        .when(on && pending, |segment| {
            segment.gap(look.length(token::SPACE_2)).child(spinner(SharedString::from(format!("usage-window-{}-spin", word.to_lowercase())), look.length(token::SPACE_3), look.color(token::FG_MUTED), cx))
        });
    labelled(one, Role::RadioButton, word).into_any_element()
}

/// The window switch: a sunken track with a segment a window.
fn switch(look: &Look, now: Span, pending: bool, cx: &mut gpui::Context<UsageScreen>) -> gpui::AnyElement {
    let track = div()
        .id("usage-window")
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1) / 2.)
        .p(look.length(token::SPACE_1) / 2. + look.length(token::BORDER_WIDTH))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_SUNKEN))
        .children(Span::ALL.map(|span| segment(look, span, span == now, pending, cx)));
    labelled(track, Role::RadioGroup, "Window").into_any_element()
}

/// The band over the list: the line, and at the right the switch. It has weight: the well ground, the heading weight, a hairline under it.
fn band(look: &Look) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap(look.length(token::SPACE_4))
        .min_h(look.length(token::H_ROW_UNIT) + look.length(token::SPACE_2))
        .px(look.length(token::SPACE_4))
        .py(look.length(token::SPACE_2))
        .bg(look.color(token::SURFACE_WELL))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
}

impl UsageScreen {
    fn render_bones(&self, look: &Look) -> Div {
        let mut rows = div().flex().flex_col();
        for row in 0..6 {
            rows = rows.child(
                div()
                    .id(SharedString::from(format!("usage-bone-{row}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .h(look.length(token::H_ROW_UNIT))
                    .px(look.length(token::SPACE_4))
                    .child(bone(look, look.length(token::SPACE_6), look.length(token::SPACE_6)))
                    .child(div().flex_1().min_w_0().child(bone(look, px(180. + 36. * ((row * 5) % 4) as f32), look.length(token::LEADING_XS) - px(4.))))
                    .child(div().flex_none().w(figure_width(look)).flex().justify_end().child(bone(look, look.length(token::SPACE_8), look.length(token::LEADING_XS) - px(4.)))),
            );
        }
        rows
    }

    /// A read Fleet refused or could not make: the band keeps its switch, so another window can be asked, and under it Fleet's reason.
    fn render_failed(&self, why: &str, look: &Look, cx: &mut gpui::Context<Self>) -> Div {
        let note = labelled(
            div()
                .id("usage-failed")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_4))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Not read"))
                .child(SharedString::from(why.to_string())),
            Role::Alert,
            "Not read",
        );
        div().flex().flex_col().child(band(look).child(div().min_w_0()).child(switch(look, self.held.span(), false, cx))).child(note)
    }

    fn render_kind(&self, row: &KindRow, focused: bool, look: &Look, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let kind = row.kind.clone();
        let opened = self.selection.opened.as_deref() == Some(row.kind.as_str());
        let under_keys = self.selection.cursor.as_deref() == Some(row.kind.as_str());
        let bar = look.length(token::SPACE_1) / 2.;
        let mark = div()
            .id(SharedString::from(format!("usage-mark-{}", row.kind)))
            .tooltip(tip(row.title.clone()))
            .child(labelled(chip(look, row.hue, row.glyph).id(SharedString::from(format!("usage-glyph-{}", row.kind))), Role::Image, row.title.clone()));
        let share = div().id(SharedString::from(format!("usage-share-{}", row.kind))).tooltip(tip(row.tip.clone())).child(labelled(
            figure(look, row.share.clone(), look.color(token::FG_DEFAULT)).id(SharedString::from(format!("usage-share-figure-{}", row.kind))),
            Role::Label,
            format!("{} wasted", row.share),
        ));
        let line = div()
            .id(SharedString::from(format!("usage-kind-{}", row.kind)))
            .relative()
            .w_full()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_ROW_UNIT))
            .px(look.length(token::SPACE_4))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .map(|line| {
                if opened {
                    line.bg(look.color(token::SURFACE_ROW_SELECTED)).text_color(look.color(token::FG_DEFAULT))
                } else if under_keys && focused {
                    line.bg(look.color(token::SURFACE_ROW_HOVER)).text_color(look.color(token::FG_DEFAULT))
                } else {
                    line.text_color(look.color(token::FG_DEFAULT)).cursor_pointer().hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER)))
                }
            })
            .when(under_keys && focused, |line| line.child(div().absolute().left_0().top_0().bottom_0().w(bar * 2.).bg(ring_colour(look))))
            .aria_selected(opened)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open(Some(kind.clone()), false, window, cx);
                window.focus(&this.keys, cx);
            }))
            .child(mark)
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(row.title.clone())))
            .child(share);
        labelled(line, Role::Row, row.title.clone()).into_any_element()
    }

    /// The band over the panels: the line, and at the right the switch.
    fn render_head(&self, board: &Board, pending: bool, look: &Look, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let header = board.header().map(|said| {
            labelled(
                div().id("usage-header").min_w_0().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(said.clone())),
                Role::Heading,
                said,
            )
        });
        panel(look).id("usage-band").flex_none().child(band(look).child(div().min_w_0().children(header)).child(switch(look, self.held.span(), pending, cx))).into_any_element()
    }

    /// The title of a panel, over its rows.
    fn render_title(&self, id: &'static str, title: &'static str, look: &Look) -> gpui::AnyElement {
        labelled(
            div()
                .id(id)
                .flex_none()
                .flex()
                .items_center()
                .h(look.length(token::H_ROW_UNIT))
                .px(look.length(token::SPACE_4))
                .bg(look.color(token::SURFACE_WELL))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(title),
            Role::Heading,
            title,
        )
        .into_any_element()
    }

    /// The stripe at a row's left edge while the keys are on it.
    fn under_keys(&self, key: &str, focused: bool) -> bool {
        focused && self.selection.cursor.as_deref() == Some(key)
    }

    /// The panel of the settings that change the burn: a row each, the value in force, a dropdown of the values, what it cuts, what
    /// changing it to the suggestion would save, and Apply.
    fn render_settings(&self, board: &Board, focused: bool, look: &Look, cx: &mut gpui::Context<Self>) -> Option<gpui::AnyElement> {
        let rows = self.changing.rows(Some(board));
        if rows.is_empty() {
            return None;
        }
        let saves_head = self.changing.span().unwrap_or(board.span()).saves_head();
        let head = |name: &'static str, cell: Div| labelled(cell.id(SharedString::from(format!("usage-setting-column-{name}"))).text_color(look.color(token::FG_SUBTLE)).child(name), Role::ColumnHeader, name);
        let heads = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_ROW_HEADER))
            .px(look.length(token::SPACE_4))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .child(head("Setting", div().flex_1().min_w_0()))
            .child(head("Now", div().flex_none().w(now_width(look))))
            .child(head("Change to", div().flex_none().w(pick_width(look))))
            .child(head("Cuts", div().flex_none().w(cuts_width(look))))
            .child(head(saves_head, div().flex_none().w(figure_width(look)).flex().justify_end()))
            .child(div().flex_none().w(apply_width(look)));
        let lines: Vec<_> = rows.iter().map(|row| self.render_setting(row, focused, look, cx)).collect();
        let table = labelled(div().id("usage-settings-table").flex().flex_col().children(lines), Role::Table, "Settings that change it");
        Some(panel(look).id("usage-settings").flex_none().flex().flex_col().child(self.render_title("usage-settings-title", "Settings that change it", look)).child(heads).child(table).into_any_element())
    }

    fn render_setting(&self, row: &SettingRow, focused: bool, look: &Look, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let under_keys = self.under_keys(&row.key, focused);
        let bar = look.length(token::SPACE_1) / 2.;
        let key = row.key.clone();
        let name = dropdown_name(&row.label);
        let changed = row.can_apply;
        let chosen = row.choices.get(row.chosen).cloned().unwrap_or_default();
        let trigger = {
            let key = key.clone();
            div()
                .id(SharedString::from(format!("usage-pick-{key}")))
                .flex_none()
                .w(pick_width(look))
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(if changed { token::ACCENT } else { token::BORDER_DEFAULT }))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .when(!row.pending, |trigger| trigger.cursor_pointer())
                .child(SharedString::from(chosen))
                .child(glyph("chevron-down", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                .when(!row.pending, |trigger| trigger.on_click(cx.listener(move |this, _, window, cx| this.open(Some(key.clone()), false, window, cx))))
        };
        let now = div()
            .id(SharedString::from(format!("usage-now-{}", row.key)))
            .flex_none()
            .w(now_width(look))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_color(look.color(if row.pending { token::FG_MUTED } else { token::FG_DEFAULT }))
            .child(SharedString::from(row.now.clone()))
            .when(row.pending, |now| now.child(spinner(SharedString::from(format!("usage-now-{}-spin", row.key)), look.length(token::SPACE_3), look.color(token::FG_MUTED), cx)));
        let now = labelled(now, Role::Label, if row.pending { format!("{} being set", row.now) } else { format!("{} now", row.now) });
        let cuts = div().id(SharedString::from(format!("usage-cuts-{}", row.key))).flex_none().w(cuts_width(look)).truncate().text_color(look.color(token::FG_MUTED)).tooltip(tip(row.cuts.clone())).child(SharedString::from(row.cuts.clone()));
        let saves = {
            let cell = div().id(SharedString::from(format!("usage-saves-{}", row.key))).child(figure(look, row.saves.clone(), look.color(token::FG_DEFAULT)));
            let cell = if row.saves_tip.is_empty() { cell } else { cell.tooltip(tip(row.saves_tip.clone())) };
            labelled(cell, Role::Label, format!("{} saved", row.saves))
        };
        let apply = {
            let key = key.clone();
            let id = SharedString::from(format!("usage-apply-{key}"));
            let button = Btn::new(id, "Apply", if row.can_apply { Weight::Primary } else { Weight::Secondary }).disabled(!row.can_apply).draw(look, cx);
            let button = if row.can_apply {
                let said = format!("Set {} to {}", row.label, row.choices.get(row.chosen).cloned().unwrap_or_default());
                button.tooltip(tip(said)).on_click(cx.listener(move |this, _, window, cx| {
                    this.apply(Some(key.clone()), cx);
                    window.focus(&this.keys, cx);
                }))
            } else {
                button
            };
            div().flex_none().w(apply_width(look)).flex().justify_end().child(labelled(button, Role::Button, format!("Apply {}", row.label)))
        };
        let line = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_h(look.length(token::H_ROW_UNIT) + look.length(token::SPACE_2))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(row.label.clone())))
            .child(now)
            .child(labelled(trigger, Role::ComboBox, name))
            .child(cuts)
            .child(saves)
            .child(apply);
        let note = row.note.as_ref().map(|note| match note {
            Note::Failed(why) => labelled(refusal(look, why.clone()).id(SharedString::from(format!("usage-refused-{}", row.key))), Role::Alert, why.clone()).into_any_element(),
            Note::Restart => {
                let said = "Applies when Fleet restarts";
                labelled(
                    div()
                        .id(SharedString::from(format!("usage-restart-{}", row.key)))
                        .flex_none()
                        .px(look.length(token::SPACE_2))
                        .rounded_full()
                        .bg(look.color(token::STATUS_AWAITING_REVIEW_BG))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::STATUS_AWAITING_REVIEW))
                        .child(said),
                    Role::Status,
                    said,
                )
                .into_any_element()
            }
        });
        let key = row.key.clone();
        let whole = div()
            .id(SharedString::from(format!("usage-setting-{}", row.key)))
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .px(look.length(token::SPACE_4))
            .pb(if note.is_some() { look.length(token::SPACE_2) } else { px(0.) })
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .when(under_keys, |whole| whole.bg(look.color(token::SURFACE_ROW_HOVER)).child(div().absolute().left_0().top_0().bottom_0().w(bar * 2.).bg(ring_colour(look))))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.rest_on(key.clone(), cx);
                window.focus(&this.keys, cx);
            }))
            .child(line)
            .children(note.map(|note| div().flex().child(note)));
        labelled(whole, Role::Row, row.label.clone()).into_any_element()
    }

    /// What each change did: the change, and the share of the burn the waste it cuts took before and since. Nothing where none was made.
    fn render_changes(&self, board: &Board, look: &Look) -> Option<gpui::AnyElement> {
        let rows = self.changing.changes(Some(board));
        if rows.is_empty() {
            return None;
        }
        let lines: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(at, ChangeRow { said, effect })| {
                let line = div()
                    .id(SharedString::from(format!("usage-change-{at}")))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(look.length(token::SPACE_4))
                    .h(look.length(token::H_ROW_UNIT))
                    .px(look.length(token::SPACE_4))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(said.clone())))
                    .child(div().flex_none().font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(effect.clone())));
                labelled(line, Role::Row, format!("{said}, {effect}")).into_any_element()
            })
            .collect();
        Some(panel(look).id("usage-changes").flex_none().flex().flex_col().child(self.render_title("usage-changes-title", "After a change", look)).children(lines).into_any_element())
    }

    /// The list of kinds of waste, each a share of the window's burn.
    fn render_kinds(&self, board: &Board, focused: bool, look: &Look, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let rows: Vec<_> = board.rows().iter().map(|row| self.render_kind(row, focused, look, cx)).collect();
        if rows.is_empty() {
            return div().into_any_element();
        }
        panel(look).id("usage-kinds").flex_none().flex().flex_col().child(self.render_title("usage-kinds-title", "Waste by kind", look)).children(rows).into_any_element()
    }

    /// The panel under the list: the kind's fix as its head, then the table of the Jobs it hit.
    fn render_open(&self, board: &Board, look: &Look, cx: &mut gpui::Context<Self>) -> Option<gpui::AnyElement> {
        let kind = board.kind(self.selection.opened.as_deref()?)?;
        let style = super::model::look_of(&kind.kind);
        let head = band(look)
            .id("usage-fix-head")
            .justify_start()
            .gap(look.length(token::SPACE_3))
            .child(chip(look, style.hue, style.glyph))
            .child(labelled(div().id("usage-fix").flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(kind.fix.clone())), Role::Heading, kind.fix.clone()));
        let head = labelled(head, Role::Group, "Fix");
        let column = |name: &'static str| labelled(figure(look, name, look.color(token::FG_SUBTLE)).font_family(sans(look)).id(SharedString::from(format!("usage-column-{name}"))), Role::ColumnHeader, name);
        let heads = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_ROW_HEADER))
            .px(look.length(token::SPACE_4))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .child(div().flex_1().min_w_0().text_color(look.color(token::FG_SUBTLE)).child("Job"))
            .when(shows_causes(&kind.kind), |heads| heads.child(labelled(div().id("usage-column-Cause").flex_none().w(cause_width(look)).text_color(look.color(token::FG_SUBTLE)).child("Cause"), Role::ColumnHeader, "Cause")))
            .child(column(board.span().share_head()))
            .child(column("Peak context"));
        let table = labelled(
            div().id("usage-jobs").flex_1().min_h_0().child(list(self.jobs.clone(), cx.processor(move |this, ix: usize, _window, cx| this.render_job(ix, cx))).size_full()),
            Role::Table,
            format!("Jobs hit by {}", kind.title),
        );
        Some(panel(look).id("usage-open").flex_none().h(look.length(token::SPACE_12) * 8.).flex().flex_col().child(head).child(heads).child(table).into_any_element())
    }

    /// The Job's largest cause in its fixed column, its middle left out where it is long, the whole name in a tooltip.
    fn render_cause(&self, id: &str, cause: String, whole: String, look: &Look) -> gpui::AnyElement {
        let cell = div().id(SharedString::from(format!("usage-job-cause-{id}"))).flex_none().w(cause_width(look)).truncate().text_color(look.color(token::FG_MUTED)).child(SharedString::from(cause));
        if whole.is_empty() {
            return cell.into_any_element();
        }
        cell.tooltip(tip(whole)).into_any_element()
    }

    fn render_job(&self, ix: usize, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let look = cx.look().clone();
        let look = &look;
        let Some(JobLine { id, title, share, peak, cause, cause_tip, tip: said }) = self.lines.get(ix).cloned() else { return div().into_any_element() };
        let with_cause = self.selection.opened.as_deref().is_some_and(shows_causes);
        let line = div()
            .id(SharedString::from(format!("usage-job-{id}")))
            .w_full()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_ROW))
            .px(look.length(token::SPACE_4))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER)))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(title.clone())))
            .when(with_cause, |line| line.child(self.render_cause(&id, cause, cause_tip, look)))
            .child(div().id(SharedString::from(format!("usage-job-share-{id}"))).tooltip(tip(said)).child(figure(look, share, look.color(token::FG_DEFAULT))))
            .child(figure(look, peak, look.color(token::FG_MUTED)));
        labelled(line, Role::Row, title).into_any_element()
    }
}

impl Render for UsageScreen {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let focused = self.keys.is_focused(window);
        let (top, open): (Vec<gpui::AnyElement>, Option<gpui::AnyElement>) = match self.held.shown() {
            Shown::Loading => (
                vec![panel(&look)
                    .id("usage-kinds")
                    .flex_none()
                    .child(div().flex().flex_col().child(band(&look).child(bone(&look, px(260.), look.length(token::LEADING_BASE) - px(4.))).child(bone(&look, px(200.), look.length(token::H_CONTROL) - px(4.)))).child(labelled(self.render_bones(&look).id("usage-loading"), Role::Image, "Loading")))
                    .into_any_element()],
                None,
            ),
            Shown::Failed(why) => (vec![panel(&look).id("usage-kinds").flex_none().child(self.render_failed(why, &look, cx)).into_any_element()], None),
            Shown::Read { board, pending } => {
                let scrolled = div()
                    .id("usage-scroll")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::GAP_PANEL))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .children(self.render_settings(board, focused, &look, cx))
                    .children(self.render_changes(board, &look))
                    .child(self.render_kinds(board, focused, &look, cx))
                    .children(self.render_open(board, &look, cx));
                (vec![self.render_head(board, pending, &look, cx), scrolled.into_any_element()], None)
            }
        };
        let body = labelled(
            div()
                .id("usage")
                .key_context(ActionScope::Usage.key_context().unwrap_or_default())
                .track_focus(&self.keys)
                .on_key_down(cx.listener(Self::on_key))
                .on_action(cx.listener(Self::on_act))
                .size_full()
                .flex()
                .flex_col()
                .gap(look.length(token::GAP_PANEL))
                .p(look.length(token::SPACE_3))
                .bg(look.color(token::BG_BASE))
                .font_family(sans(&look))
                .text_color(look.color(token::FG_DEFAULT))
                .children(top)
                .children(open),
            Role::Region,
            "Usage",
        );
        // The list of a setting's values stands beside the page's keys and not inside them, so its Enter and Space are its own.
        div().id("usage-page").track_focus(&self.within).size_full().child(body).children(self.popup.as_ref().map(|(menu, _)| menu.clone()))
    }
}
