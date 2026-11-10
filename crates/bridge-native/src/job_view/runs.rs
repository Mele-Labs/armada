//! The items that group rows: runs of calls, subagents, Check runs and Fleet turns (`job-view.md`,
//! A3 to A6).

use std::rc::Rc;

use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::{SessionCheckState, SessionRow, ToolSubagent, ToolSubagentState};

use super::acts::refusal;
use super::calls;
use super::tool_icons::{self, Kind};
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

/// What a call's row says: its kind, its phrase, and while it runs, what it works on.
struct Call {
    kind: Kind,
    phrase: String,
    working_on: Option<String>,
}

/// One call (take A, approved 10 Nov): a rail in its kind's colour, its kind's icon, the phrase and the
/// time it took. While it runs a band travels down the rail, the icon moves, the time counts and a
/// second line says what it works on. Done, it is one grey line, the icon on its last frame.
fn tool_row(key: &str, call: Call, at: &ipc::Instant, end: Option<&ipc::Instant>, running: bool, look: &Look) -> AnyElement {
    let Call { kind, phrase, working_on } = call;
    let ink = match kind {
        Kind::Read | Kind::Search | Kind::Web | Kind::Other => token::ACCENT,
        Kind::Run => token::STATUS_RUNNING,
        Kind::Edit => token::STATUS_COMPLETED_SUCCESS,
        Kind::Agent => token::STATUS_AWAITING_REVIEW,
    };
    let side = look.length(token::SPACE_6);
    let gap = side * (10. / 24.);
    let mut rail = div().flex_none().w(gpui::px(3.)).relative().overflow_hidden().bg(look.color(if running { ink } else { token::BORDER_DEFAULT }));
    if running {
        let band = look.color(token::FG_DEFAULT).opacity(0.35);
        rail = rail.child(
            div().absolute().left_0().w_full().h(gpui::relative(0.4)).bg(band).with_animation(
                SharedString::from(format!("rail:{key}")),
                Animation::new(Duration::from_millis(1400)).repeat().with_easing(gpui::ease_in_out),
                |band, delta| band.top(gpui::relative(-0.4 + 1.4 * delta)),
            ),
        );
    }
    let since = at.clone();
    let time = div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE));
    let time = if running {
        // The count goes on each frame the row is drawn.
        time.with_animation(SharedString::from(format!("count:{key}")), Animation::new(Duration::from_secs(1)).repeat(), move |time, _| {
            time.child(elapsed_words(since.as_str(), &Utc::now().to_rfc3339()).unwrap_or_default())
        })
        .into_any_element()
    } else {
        time.children(end.and_then(|end| elapsed_words(at.as_str(), end.as_str()))).into_any_element()
    };
    let icon_ink = look.color(if running { ink } else { token::FG_SUBTLE });
    // A brief is not a verb, so a running agent's row says it runs.
    let said = if running && kind == Kind::Agent { format!("{phrase}, running") } else { phrase.clone() };
    let first = div()
        .flex()
        .items_center()
        .gap(gap)
        .min_w_0()
        .child(tool_icons::icon(kind, running, SharedString::from(format!("icon:{key}")), icon_ink, side))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(if running { token::FG_DEFAULT } else { token::FG_MUTED }))
                .child(phrase),
        )
        .child(time);
    let second = working_on.map(|line| {
        div()
            .pl(side + gap)
            .min_w_0()
            .truncate()
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS) + gpui::px(0.5))
            .line_height(look.length(token::LEADING_2XS))
            .text_color(look.color(token::FG_SUBTLE))
            .child(line)
    });
    labelled(
        div()
            .id(SharedString::from(format!("call:{key}")))
            .flex()
            .min_w_0()
            .child(rail)
            .child(div().flex_1().min_w_0().flex().flex_col().pl(look.length(token::SPACE_3)).py(look.length(token::SPACE_1)).child(first).children(second)),
        Role::Row,
        said,
    )
    .into_any_element()
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

    #[allow(clippy::too_many_arguments)]
    pub(super) fn calls(
        &self,
        ix: usize,
        id: &str,
        rows: &[SessionRow],
        live: bool,
        end: Option<&ipc::Instant>,
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
        // Each call ended when the next began; the last, when the item after the run did.
        let last = rows.len().saturating_sub(1);
        let lines = rows.iter().enumerate().map(|(at, row)| {
            let running = live && at == last;
            let end = rows.get(at + 1).map(SessionRow::at).or(end);
            let one = match row {
                SessionRow::WindowAsk { question, .. } => Call { kind: Kind::Other, phrase: question.clone(), working_on: None },
                _ => Call {
                    kind: calls::kind_of(&texts[at]),
                    phrase: calls::call_line(&texts[at], running),
                    working_on: calls::working_on(&texts[at]).filter(|_| running),
                },
            };
            tool_row(&format!("{id}:{at}"), one, row.at(), if running { None } else { end }, running, look)
        });
        if live {
            return labelled(
                div().id(SharedString::from(format!("live:{id}"))).flex().flex_col().gap(look.length(token::SPACE_1)).children(lines),
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
        let body = div().flex().flex_col().gap(look.length(token::SPACE_1)).children(lines).into_any_element();
        self.fold(ix, format!("calls:{id}"), label, head, Some(body), page, look, cx)
    }

    pub(super) fn subagent(&self, id: &str, subagent: &ToolSubagent, look: &Look) -> AnyElement {
        let task = subagent.task.clone();
        if subagent.state == ToolSubagentState::Running {
            let one = Call { kind: Kind::Agent, phrase: task, working_on: None };
            return div().id(SharedString::from(format!("subagent:{id}"))).child(tool_row(&format!("subagent:{id}"), one, &subagent.since, None, true, look)).into_any_element();
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
