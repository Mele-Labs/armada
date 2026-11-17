//! A feature's page drawn from what `page_model.rs` made of Fleet's read: a strip across the top, the sections
//! down the side, and the open one beside them. Every value is a token.

use gpui::{div, list, prelude::*, px, Div, FontWeight, IntoElement, Render, Role, SharedString, Toggled, Window};

use super::asks;
use super::page::{FeaturePageView, Load, Region};
use super::page_model::{Block, Body, Chip, Code, Entry, Section, Written};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::settings::view::{bone, ring_colour, tip};
use crate::titlebar::parts::{mono, sans};

/// The words a pane's small heads are in: caps, light and muted.
fn small_head(look: &Look, words: &str) -> Div {
    div()
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_MUTED))
        .child(SharedString::from(words.to_uppercase()))
}

impl FeaturePageView {
    fn render_top(&self, look: &Look) -> impl IntoElement {
        let title = match &self.page {
            Load::Read(page) => div().child(SharedString::from(page.feature.title.clone())).into_any_element(),
            _ => bone(look, px(160.), look.length(token::LEADING_SM) - px(4.)).into_any_element(),
        };
        let area = match (&self.page, &self.area) {
            (Load::Read(_), Some(area)) => Some(area.clone()),
            (Load::Read(page), None) => Some(page.feature.area.replace('-', " ")),
            _ => None,
        };
        let current = match &self.page {
            Load::Read(page) => super::page_model::current_as_of(page),
            _ => None,
        };
        div()
            .id("feature-top")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_4))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(div().id("feature-title").text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(title), Role::Heading, match &self.page {
                Load::Read(page) => page.feature.title.clone(),
                _ => "Feature".into(),
            }))
            .children(area.map(|area| {
                labelled(
                    div()
                        .id("feature-area")
                        .flex_none()
                        .flex()
                        .items_center()
                        .h(look.length(token::H_BADGE))
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .bg(look.color(token::FG_DEFAULT).opacity(0.08))
                        .text_size(look.length(token::TEXT_2XS))
                        .text_color(look.color(token::FG_MUTED))
                        .whitespace_nowrap()
                        .child(SharedString::from(area.clone())),
                    Role::Label,
                    format!("Area: {area}"),
                )
            }))
            .children(current.map(|current| {
                div().id("feature-current").ml_auto().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).whitespace_nowrap().child(SharedString::from(current))
            }))
    }

    // ---- the rail ----------------------------------------------------------------------------

    fn render_rail(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let focused = self.rail.is_focused(window);
        let mut rail = div().flex().flex_col();
        match &self.page {
            Load::Read(_) => {
                for entry in &self.entries {
                    rail = rail.child(self.render_entry(entry, focused, look, cx));
                }
            }
            Load::Reading => {
                for at in 0..Section::ALL.len() {
                    rail = rail.child(
                        div()
                            .id(SharedString::from(format!("feature-section-bone-{at}")))
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .px(look.length(token::SPACE_4))
                            .py(look.length(token::SPACE_2))
                            .child(bone(look, px(84. + 18. * (at % 3) as f32), look.length(token::LEADING_XS) - px(4.)))
                            .child(bone(look, px(120. + 14. * (at % 4) as f32), look.length(token::LEADING_2XS) - px(4.))),
                    );
                }
                rail = div().flex().flex_col().child(labelled(rail.id("feature-sections-loading"), Role::Image, "Loading"));
            }
            Load::Failed(_) => {}
        }
        div()
            .id("feature-rail")
            .track_focus(&self.rail)
            .flex_none()
            .w(look.length(token::SPACE_12) * 5.)
            .h_full()
            .py(look.length(token::SPACE_2))
            .bg(look.color(token::BG_SUNKEN))
            .border_r(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(rail.id("feature-sections"), Role::TabList, "Sections"))
    }

    /// A section: its title, and the line under it. The open one is filled and has an edge at its left.
    fn render_entry(&self, entry: &Entry, focused: bool, look: &Look, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let section = entry.section;
        let open = self.section == section;
        let summary_ink = if entry.stale { token::STATUS_AWAITING_REVIEW } else { token::FG_SUBTLE };
        let row = div()
            .id(SharedString::from(format!("feature-section-{}", section.id())))
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if open && focused { ring_colour(look) } else { gpui::transparent_black() })
            .when(open, |row| row.bg(look.color(token::ACCENT_MUTED)))
            .when(!open, |row| row.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
            .aria_selected(open)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.select(section, cx);
                this.focus_region(Region::Rail, window, cx);
            }))
            .children(open.then(|| div().absolute().left_0().top_0().bottom_0().w(look.length(token::EDGE_ACTIVE)).bg(look.color(token::ACCENT))))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(section.title()),
            )
            .child(
                div()
                    .min_w_0()
                    .h(look.length(token::LEADING_2XS))
                    .truncate()
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(summary_ink))
                    .child(SharedString::from(entry.summary.clone())),
            );
        labelled(row, Role::Tab, section.title()).into_any_element()
    }

    // ---- the pane ----------------------------------------------------------------------------

    fn render_chips(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let focused = self.chips.is_focused(window);
        let mut chips = div().flex().items_center().gap(look.length(token::SPACE_2));
        for chip in Chip::ALL {
            let on = self.chip == chip;
            let cursor = focused && self.chip_at == chip;
            let said = match chip {
                Chip::Since => Some(match self.looked {
                    Some(at) => format!("Asks changed after {}", super::page_model::day(at)),
                    None => "Not looked at before, so every ask shows".to_string(),
                }),
                _ => None,
            };
            let pill = div()
                .id(SharedString::from(format!("feature-chip-{}", chip.label().to_lowercase().replace(' ', "-"))))
                .flex_none()
                .flex()
                .items_center()
                .h(look.length(token::H_BADGE))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if cursor { ring_colour(look) } else { gpui::transparent_black() })
                .bg(if on { look.color(token::ACCENT_MUTED) } else { look.color(token::FG_DEFAULT).opacity(0.08) })
                .text_size(look.length(token::TEXT_2XS))
                .text_color(if on { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) })
                .whitespace_nowrap()
                .cursor_pointer()
                .when(!on, |pill| pill.hover(|style| style.bg(look.color(token::BG_HOVER))))
                .aria_toggled(if on { Toggled::True } else { Toggled::False })
                .when_some(said, |pill, said| pill.tooltip(tip(said)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.apply(chip, cx);
                    this.focus_region(Region::Chips, window, cx);
                }))
                .child(chip.label());
            chips = chips.child(labelled(pill, Role::Button, chip.label()));
        }
        div().id("feature-chips").track_focus(&self.chips).px(look.length(token::SPACE_2)).child(labelled(chips.id("feature-chip-row"), Role::Group, "Show"))
    }

    fn render_row(&self, ix: usize, look: &Look, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let Body::Rows(rows) = &self.body else { return div().into_any_element() };
        let Some(row) = rows.get(ix) else { return div().into_any_element() };
        let picked = self.row_at == Some(ix);
        let line = asks::row(look, SharedString::from(format!("feature-row-{ix}")), row, picked).on_click(cx.listener(move |this, _, window, cx| {
            this.pick_row(ix, cx);
            this.focus_region(Region::Rows, window, cx);
        }));
        labelled(line, Role::ListBoxOption, row.title.clone()).into_any_element()
    }

    fn render_code(&self, code: &Code, look: &Look) -> Div {
        let line = |text: &str| div().min_w_0().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(text.to_string()));
        let block = |head: &str, lines: Div| div().flex().flex_col().gap(look.length(token::SPACE_1)).child(small_head(look, head)).child(lines);
        let mut out = div().flex().flex_col().gap(look.length(token::SPACE_4));
        if !code.paths.is_empty() {
            out = out.child(block("Paths", code.paths.iter().fold(div().flex().flex_col(), |lines, path| lines.child(line(path)))));
        }
        if !code.entry_points.is_empty() {
            out = out.child(block("Entry points", code.entry_points.iter().fold(div().flex().flex_col(), |lines, point| lines.child(line(point)))));
        }
        if !code.files.is_empty() {
            let mut files = div().flex().flex_col();
            for (at, (path, jobs)) in code.files.iter().enumerate() {
                // The figure stands in a column of its own, so the figures line up down the files.
                let figure = div()
                    .id(SharedString::from(format!("feature-file-jobs-{at}")))
                    .flex_none()
                    .w(look.length(token::SPACE_8))
                    .flex()
                    .justify_end()
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .tooltip(tip(if *jobs == 1 { "Jobs that wrote this file: 1".to_string() } else { format!("Jobs that wrote this file: {jobs}") }))
                    .child(SharedString::from(jobs.to_string()));
                files = files.child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(look.length(token::SPACE_3))
                        .child(div().flex_1().min_w_0().child(line(path)))
                        .child(labelled(figure, Role::Image, format!("Jobs that wrote {path}: {jobs}"))),
                );
            }
            out = out.child(block("Files", files));
        }
        out
    }

    fn render_written(&self, written: &Written, look: &Look) -> Div {
        let mut out = div().flex().flex_col().gap(look.length(token::SPACE_3));
        for Block { text, command } in &written.blocks {
            out = out.child(if *command {
                div()
                    .px(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::BG_SUNKEN))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(text.clone()))
            } else {
                div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(text.clone()))
            });
        }
        out.child(
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_SUBTLE))
                .child(SharedString::from(written.by.clone()))
                .children(written.stale.clone().map(|stale| labelled(asks::tinted(look, token::STATUS_AWAITING_REVIEW, stale.clone()).id("feature-stale"), Role::Image, stale))),
        )
    }

    fn render_body(&self, look: &Look, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let reading = match (&self.page, &self.asks) {
            (Load::Reading, _) => true,
            (_, Load::Reading) => self.section.is_asks(),
            _ => false,
        };
        if reading {
            let mut bones = div().flex().flex_col();
            for at in 0..4 {
                bones = bones.child(
                    div()
                        .id(SharedString::from(format!("feature-row-bone-{at}")))
                        .flex()
                        .items_start()
                        .gap(look.length(token::SPACE_3))
                        .px(look.length(token::SPACE_2))
                        .py(look.length(token::SPACE_3))
                        .border_t(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .child(bone(look, look.length(token::SPACE_4), look.length(token::SPACE_4)))
                        .child(div().flex().flex_col().gap(px(4.)).child(bone(look, px(220. + 40. * (at % 3) as f32), look.length(token::LEADING_SM) - px(6.))).child(bone(look, px(140.), look.length(token::LEADING_XS) - px(6.)))),
                );
            }
            return labelled(bones.id("feature-pane-loading"), Role::Image, "Loading").into_any_element();
        }
        let failed = match (&self.page, &self.asks) {
            (Load::Failed(why), _) => Some(why.clone()),
            (_, Load::Failed(why)) if self.section.is_asks() => Some(why.clone()),
            _ => None,
        };
        if let Some(why) = failed {
            return div()
                .px(look.length(token::SPACE_2))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Not read"))
                .child(SharedString::from(why))
                .into_any_element();
        }
        match &self.body {
            Body::Rows(rows) if rows.is_empty() => div().into_any_element(),
            Body::Rows(_) => {
                labelled(
                    div()
                        .id("feature-rows")
                        .track_focus(&self.rows)
                        .flex_1()
                        .min_h_0()
                        .child(list(self.list.clone(), cx.processor(move |this, ix: usize, _, cx| {
                            let look = cx.look().clone();
                            this.render_row(ix, &look, cx)
                        })).size_full()),
                    Role::ListBox,
                    self.section.title(),
                )
                .into_any_element()
            }
            Body::Code(code) => div().id("feature-code").flex_1().min_h_0().overflow_y_scroll().px(look.length(token::SPACE_2)).child(self.render_code(code, look)).into_any_element(),
            Body::Written(None) => div().into_any_element(),
            Body::Written(Some(written)) => div().id("feature-written").flex_1().min_h_0().overflow_y_scroll().px(look.length(token::SPACE_2)).child(self.render_written(written, look)).into_any_element(),
        }
    }

    fn render_pane(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let title = self.section.title();
        let head = labelled(
            div().id("feature-pane-title").px(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(title),
            Role::Heading,
            title,
        );
        let chips = (self.section.is_asks() && matches!(self.page, Load::Read(_))).then(|| self.render_chips(look, window, cx));
        let body = self.render_body(look, cx);
        labelled(
            div()
                .id("feature-pane")
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_4))
                .child(head)
                .children(chips)
                .child(body),
            Role::Region,
            title,
        )
    }
}

impl Render for FeaturePageView {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let top = self.render_top(&look);
        let rail = self.render_rail(&look, window, cx);
        let pane = self.render_pane(&look, window, cx);
        div()
            .id("feature-page-view")
            .key_context(crate::generated::actions::ActionScope::Features.key_context().unwrap_or_default())
            .track_focus(&self.within)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::on_act))
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(top)
            .child(div().flex_1().min_h_0().flex().child(rail).child(pane))
    }
}
