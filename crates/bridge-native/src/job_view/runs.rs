//! The items that group rows: runs of calls, subagents, Check runs and Fleet turns (`job-view.md`,
//! A3 to A6).

use std::rc::Rc;

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::{SessionCheckState, SessionRow, ToolSubagent, ToolSubagentState};

use super::acts::refusal;
use super::calls;
use super::menus::{notice_entries, teach_entries};
use super::rows::{check_look, elapsed_words, glyph, mono, tip, Size};
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};

/// The first sentence of a notice: up to the first line break or `. `, without a closing full stop.
fn summary_of(text: &str) -> String {
    let line = text.lines().next().unwrap_or("");
    let first = line.split(". ").next().unwrap_or(line);
    first.trim_end_matches('.').to_string()
}

impl JobView {
    /// The mark of a Check run and the word it says.
    pub(super) fn check_mark(&self, state: SessionCheckState, look: &Look) -> AnyElement {
        let (icon, ink, word) = check_look(state);
        div()
            .id(SharedString::from(format!("check-mark:{word}")))
            .flex_none()
            .flex()
            .justify_center()
            .w(look.length(token::SPACE_4))
            .child(glyph(look, icon, Size::Small, look.color(ink)))
            .tooltip(tip(word))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn checks(
        &self,
        ix: usize,
        id: &str,
        rows: &[SessionRow],
        page: &Page,
        look: &Look,
        now: DateTime<Local>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let item = self.checks_item(ix, id, rows, page, look, now, cx);
        let Some(SessionRow::Check { name, target, state, at, .. }) = rows.iter().rev().find(|row| matches!(row, SessionRow::Check { .. })) else { return item };
        let spot = self.spot(page, &format!("check-item:{id}"), cx);
        let (_, _, word) = check_look(*state);
        let text = [name.as_str(), target.as_deref().unwrap_or(""), word].join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
        let wrap = div().id(SharedString::from(format!("check-item:{id}"))).track_focus(&spot.focus).child(item);
        self.menu_on(wrap, &spot, "Check", spot.focus.clone(), Rc::new(teach_entries), self.teach_pick("check", text, Some(at.clone())), cx).into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn checks_item(
        &self,
        ix: usize,
        id: &str,
        rows: &[SessionRow],
        page: &Page,
        look: &Look,
        now: DateTime<Local>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let runs: Vec<(&String, SessionCheckState, Option<&String>, &ipc::Instant)> = rows
            .iter()
            .filter_map(|row| match row {
                SessionRow::Check { name, state, target, at, .. } => Some((name, *state, target.as_ref(), at)),
                _ => None,
            })
            .collect();
        let Some(&(name, state, target, _)) = runs.last() else { return div().into_any_element() };
        let (_, _, word) = check_look(state);
        let target_line = |target: Option<&String>| {
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(mono(look))
                .text_color(look.color(token::FG_SUBTLE))
                .child(target.cloned().unwrap_or_default())
        };
        if runs.len() == 1 {
            let said = match target {
                Some(target) => format!("Check {name} {target}, {word}"),
                None => format!("Check {name}, {word}"),
            };
            return self.plain(
                format!("check:{id}"),
                said,
                look,
                vec![
                    self.check_mark(state, look),
                    div().flex_none().text_color(look.color(token::FG_MUTED)).child(name.clone()).into_any_element(),
                    target_line(target).into_any_element(),
                ],
            );
        }
        let head = vec![self.check_mark(state, look), div().child(name.clone()).into_any_element()];
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .children(runs.iter().map(|&(_, state, target, at)| {
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(self.check_mark(state, look))
                    .child(target_line(target))
                    .child(self.stamp(at, look, now, true, cx))
            }))
            .into_any_element();
        self.fold(ix, format!("checks:{id}"), format!("Check {name}, latest {word}"), head, Some(body), page, look, cx)
    }

    /// The line a call reads as in the live list: the whole command for a shell call (A3.3).
    fn live_line(row: &SessionRow, newest: bool) -> String {
        match row {
            SessionRow::Tool { text, .. } => match text.strip_prefix("Bash ") {
                Some(command) => format!("{} {}", if newest { "Running" } else { "Ran" }, command.trim()),
                None => calls::call_line(text, newest),
            },
            SessionRow::Ask { ask, .. } => calls::call_line(&format!("{} {}", ask.tool, ask.detail), newest),
            SessionRow::WindowAsk { question, .. } => question.clone(),
            _ => String::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn calls(
        &self,
        ix: usize,
        id: &str,
        rows: &[SessionRow],
        live: bool,
        page: &Page,
        look: &Look,
        cx: &Context<Self>,
    ) -> AnyElement {
        let texts: Vec<String> = rows
            .iter()
            .map(|row| match row {
                SessionRow::Tool { text, .. } => text.clone(),
                SessionRow::Ask { ask, .. } => format!("{} {}", ask.tool, ask.detail),
                _ => String::new(),
            })
            .collect();
        let refs: Vec<&str> = texts.iter().map(String::as_str).filter(|text| !text.is_empty()).collect();
        let label = format!("Tool calls: {}", calls::names_of(&refs).join(", "));
        if live {
            let last = rows.len().saturating_sub(1);
            return labelled(
                div()
                    .id(SharedString::from(format!("live:{id}")))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .children(rows.iter().enumerate().map(|(at, row)| {
                        let newest = at == last;
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_SM))
                            .text_color(look.color(if newest { token::FG_DEFAULT } else { token::FG_SUBTLE }))
                            .child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .justify_center()
                                    .w(look.length(token::SPACE_4))
                                    .when(newest, |cell| cell.child(self.working_mark(look, look.color(token::STATUS_RUNNING)))),
                            )
                            .child(div().min_w_0().truncate().child(Self::live_line(row, newest)))
                    })),
                Role::List,
                label,
            )
            .into_any_element();
        }
        let line = calls::calls_line(&refs, false);
        let head = vec![
            glyph(look, "wrench", Size::Small, look.color(token::FG_MUTED)).into_any_element(),
            div().min_w_0().truncate().child(line).into_any_element(),
        ];
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .children(rows.iter().map(|row| {
                div()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(Self::live_line(row, false))
            }))
            .into_any_element();
        self.fold(ix, format!("calls:{id}"), label, head, Some(body), page, look, cx)
    }

    pub(super) fn subagent(&self, id: &str, subagent: &ToolSubagent, look: &Look) -> AnyElement {
        let task = subagent.task.clone();
        if subagent.state == ToolSubagentState::Running {
            return labelled(
                div()
                    .id(SharedString::from(format!("subagent:{id}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(glyph(look, "bot", Size::Large, look.color(token::FG_SUBTLE)))
                    .child(div().min_w_0().truncate().child(format!("Started a subagent: {task}"))),
                Role::Row,
                format!("Subagent {task}, running"),
            )
            .into_any_element();
        }
        let cut = subagent.state == ToolSubagentState::CutOff;
        let said = if cut { "Subagent cut off" } else { "Subagent finished" };
        let tooltip = match &subagent.agent_type {
            Some(kind) => format!("{said} \u{b7} {kind}"),
            None => said.to_string(),
        };
        let ran = subagent.ended.as_ref().and_then(|ended| elapsed_words(subagent.since.as_str(), ended.as_str()));
        labelled(
            Self::padded(look, div())
                .id(SharedString::from(format!("subagent:{id}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_SUNKEN))
                .text_color(look.color(token::FG_DEFAULT))
                .child(
                    div()
                        .id(SharedString::from(format!("subagent-mark:{id}")))
                        .child(glyph(
                            look,
                            if cut { "power" } else { "check" },
                            Size::Small,
                            look.color(if cut { token::FG_DEFAULT } else { token::STEP_ADVANCED }),
                        ))
                        .tooltip(tip(tooltip)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .child(task.clone()),
                )
                .children(ran.map(|ran| {
                    div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(ran)
                })),
            Role::Row,
            format!("Subagent {task}, {}", if cut { "cut off" } else { "finished" }),
        )
        .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn fleet_turn(
        &self,
        ix: usize,
        id: &str,
        notice: &SessionRow,
        rows: &[SessionRow],
        handled: bool,
        page: &Page,
        look: &Look,
        cx: &Context<Self>,
    ) -> AnyElement {
        let SessionRow::Message { text, at, notice: word, .. } = notice else { return div().into_any_element() };
        let mut head = vec![
            div()
                .id(SharedString::from(format!("fleet-mark:{id}")))
                .child(glyph(look, "bell-ring", Size::Large, look.color(token::FG_SUBTLE)))
                .tooltip(tip("Fleet notice"))
                .into_any_element(),
            div().min_w_0().truncate().child(summary_of(text)).into_any_element(),
        ];
        if handled {
            head.push(
                div()
                    .id(SharedString::from(format!("fleet-handled:{id}")))
                    .child(glyph(look, "check", Size::Large, look.color(token::FG_SUBTLE)))
                    .tooltip(tip("Handled"))
                    .into_any_element(),
            );
        }
        let part = |text: String| {
            div()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(text)
        };
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .ml(look.length(token::SPACE_1))
            .pl(look.length(token::SPACE_4))
            .border_l(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(part(text.clone()))
            .children(rows.iter().map(|row| match row {
                SessionRow::Tool { text, .. } => part(text.clone()).into_any_element(),
                SessionRow::Message { id, text, .. } => self.prose(format!("fleet:{id}"), text, look),
                _ => div().into_any_element(),
            }))
            .into_any_element();
        let fold_key = format!("fleet-turn-fold:{id}");
        let spot = self.spot(page, &format!("fleet-turn:{id}"), cx);
        let back = self.spot(page, &fold_key, cx).focus;
        let said = page.cards.said.get(&format!("notice:{id}")).cloned();
        let word = word.clone();
        let entries_of = word.clone();
        let region = div()
            .id(SharedString::from(format!("fleet-turn:{id}")))
            .text_color(look.color(token::FG_SUBTLE))
            .child(self.fold(ix, fold_key, "Fleet turn".into(), head, Some(body), page, look, cx))
            .children(said.map(|why| refusal(look, &format!("notice:{id}"), why)));
        let region = self.menu_on(
            region,
            &spot,
            "Fleet notice",
            back,
            Rc::new(move || notice_entries(entries_of.as_deref())),
            self.notice_pick(&format!("notice:{id}"), text.clone(), Some(at.clone()), word),
            cx,
        );
        labelled(region, Role::Region, "Fleet turn").into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notice_is_summed_up_by_its_first_sentence() {
        assert_eq!(summary_of("Your checks failed. Look at the log.\nMore"), "Your checks failed");
        assert_eq!(summary_of("One line."), "One line");
    }
}
