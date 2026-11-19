//! The Usage page drawn from the board `model.rs` made: a panel with the header band over the list of kinds, and under it,
//! where a kind is open, a panel with the kind's fix and the table of the Jobs it hit. Every value is a token.

use gpui::{div, list, prelude::*, px, Div, FontWeight, Hsla, IntoElement, Pixels, Render, Role, SharedString, Window};

use super::model::{Board, JobLine, KindRow, Span};
use super::{Load, UsageScreen};
use crate::generated::actions::ActionScope;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::reviews::panel;
use crate::settings::view::{bone, ring_colour, tip};
use crate::titlebar::parts::{glyph, mono, sans};

/// The width of the units column and of the peak column: two steps of the largest space, so the figures line up down both panels.
fn figure_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 2.
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

/// The segment a window is drawn as: raised in an accent wash and ring where it is the one read.
fn segment(look: &Look, span: Span, on: bool, cx: &mut gpui::Context<UsageScreen>) -> gpui::AnyElement {
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
        .child(word);
    labelled(one, Role::RadioButton, word).into_any_element()
}

/// The window switch: a sunken track with a segment a window.
fn switch(look: &Look, now: Span, cx: &mut gpui::Context<UsageScreen>) -> gpui::AnyElement {
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
        .children(Span::ALL.map(|span| segment(look, span, span == now, cx)));
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

    fn render_kind(&self, row: &KindRow, focused: bool, look: &Look, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let kind = row.kind.clone();
        let opened = self.selection.opened.as_deref() == Some(row.kind.as_str());
        let under_keys = self.selection.cursor.as_deref() == Some(row.kind.as_str());
        let bar = look.length(token::SPACE_1) / 2.;
        let mark = div()
            .id(SharedString::from(format!("usage-mark-{}", row.kind)))
            .tooltip(tip(row.title.clone()))
            .child(labelled(chip(look, row.hue, row.glyph).id(SharedString::from(format!("usage-glyph-{}", row.kind))), Role::Image, row.title.clone()));
        let units = div().id(SharedString::from(format!("usage-units-{}", row.kind))).tooltip(tip(row.tip.clone())).child(labelled(
            figure(look, row.units.clone(), look.color(token::FG_DEFAULT)).id(SharedString::from(format!("usage-units-figure-{}", row.kind))),
            Role::Label,
            format!("{} wasted", row.units),
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
                this.open(Some(kind.clone()), cx);
                window.focus(&this.keys, cx);
            }))
            .child(mark)
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(row.title.clone())))
            .child(units);
        labelled(line, Role::Row, row.title.clone()).into_any_element()
    }

    fn render_board(&self, board: &Board, focused: bool, look: &Look, cx: &mut gpui::Context<Self>) -> Div {
        let header = board.header().map(|said| {
            labelled(
                div().id("usage-header").min_w_0().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(said.clone())),
                Role::Heading,
                said,
            )
        });
        let rows: Vec<_> = board.rows().iter().map(|row| self.render_kind(row, focused, look, cx)).collect();
        div().flex().flex_col().child(band(look).child(div().min_w_0().children(header)).child(switch(look, board.span(), cx))).children(rows)
    }

    /// The panel under the list: the kind's fix as its head, then the table of the Jobs it hit.
    fn render_open(&self, board: &Board, look: &Look, cx: &mut gpui::Context<Self>) -> Option<gpui::AnyElement> {
        let kind = board.kind(self.selection.opened.as_deref()?)?;
        let style = super::model::look_of(&kind.kind);
        let head = band(look)
            .justify_start()
            .gap(look.length(token::SPACE_3))
            .child(chip(look, style.hue, style.glyph))
            .child(labelled(div().id("usage-fix").flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(kind.fix.clone())), Role::Heading, kind.fix.clone()));
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
            .child(column("Units wasted"))
            .child(column("Peak context"));
        let table = labelled(
            div().id("usage-jobs").flex_1().min_h_0().child(list(self.jobs.clone(), cx.processor(move |this, ix: usize, _window, cx| this.render_job(ix, cx))).size_full()),
            Role::Table,
            format!("Jobs hit by {}", kind.title),
        );
        Some(panel(look).id("usage-open").flex_1().min_h_0().flex().flex_col().child(head).child(heads).child(table).into_any_element())
    }

    fn render_job(&self, ix: usize, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let look = cx.look().clone();
        let look = &look;
        let Some(JobLine { id, title, units, peak, tip: said }) = self.lines.get(ix).cloned() else { return div().into_any_element() };
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
            .child(div().id(SharedString::from(format!("usage-job-units-{id}"))).tooltip(tip(said)).child(figure(look, units, look.color(token::FG_DEFAULT))))
            .child(figure(look, peak, look.color(token::FG_MUTED)));
        labelled(line, Role::Row, title).into_any_element()
    }
}

impl Render for UsageScreen {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let focused = self.keys.is_focused(window);
        let (kinds, open) = match &self.load {
            Load::Reading => (
                div().flex().flex_col().child(band(&look).child(bone(&look, px(260.), look.length(token::LEADING_BASE) - px(4.))).child(bone(&look, px(200.), look.length(token::H_CONTROL) - px(4.)))).child(labelled(self.render_bones(&look).id("usage-loading"), Role::Image, "Loading")),
                None,
            ),
            Load::Read(board) => (self.render_board(board, focused, &look, cx), self.render_open(board, &look, cx)),
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
                .child(panel(&look).id("usage-kinds").flex_none().child(kinds))
                .children(open),
            Role::Region,
            "Usage",
        );
        div().id("usage-page").track_focus(&self.within).size_full().child(body)
    }
}
