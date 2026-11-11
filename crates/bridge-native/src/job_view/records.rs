//! The rows that carry a body: a command the person ran, a request, a saved request, the retro, words
//! sent on, changes to the plan, a disputed notice (`job-view.md`, A7). A call put to the person is a
//! card the next slice draws, and here a row naming its kind.

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::{RetroStage, SessionRow};

use super::rows::{eyebrow, glyph, mono, tip, Size};
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};

impl JobView {
    pub(super) fn body_row(&self, ix: usize, row: &SessionRow, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        match row {
            SessionRow::Ran { id, command, code, tail, .. } => self.ran(ix, id, command, *code, tail, page, look, cx),
            SessionRow::Requested { id, by, method, url, status, ms, error, response, truncated, .. } => {
                let (icon, ink) = match status {
                    Some(status) if *status < 400 => ("check", token::STEP_ADVANCED),
                    Some(_) => ("x", token::STATUS_COMPLETED_FAILED),
                    None => ("unplug", token::STATUS_COMPLETED_FAILED),
                };
                let who = if matches!(by, ipc::RequestBy::Owner) { "You" } else { "Agent" };
                let answered = status.map_or("No answer".to_string(), |status| status.to_string());
                let took = if *ms < 1000 { format!("{ms} ms") } else { format!("{:.1} s", *ms as f64 / 1000.) };
                let why = match (status, error) {
                    (None, Some(error)) => format!("No answer: {error}"),
                    _ => answered.clone(),
                };
                let head = vec![
                    div()
                        .id(SharedString::from(format!("req-mark:{id}")))
                        .child(glyph(look, icon, Size::Small, look.color(ink)))
                        .tooltip(tip(why))
                        .into_any_element(),
                    div().flex_none().text_color(look.color(token::FG_SUBTLE)).child(who).into_any_element(),
                    div().flex_none().font_family(mono(look)).text_color(look.color(token::FG_DEFAULT)).child(method.clone()).into_any_element(),
                    div().flex_1().min_w_0().truncate().font_family(mono(look)).child(url.clone()).into_any_element(),
                    div().flex_none().font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).child(took).into_any_element(),
                ];
                let body = (error.is_some() || !response.is_empty()).then(|| {
                    div()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .children(error.clone().map(|error| {
                            div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).child(error)
                        }))
                        .when(!response.is_empty(), |column| column.child(self.log(response, look)))
                        .when(*truncated, |column| column.child(div().text_size(look.length(token::TEXT_2XS)).child("Cut at 16 KiB")))
                        .into_any_element()
                });
                self.fold(ix, format!("requested:{id}"), format!("{who}: {answered} {method} {url}"), head, body, page, look, cx)
            }
            SessionRow::Saved { id, at, name, collection, .. } => labelled(
                Self::padded(look, div())
                    .id(SharedString::from(format!("saved:{id}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_SUNKEN))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(
                        div()
                            .id(SharedString::from(format!("saved-mark:{id}")))
                            .child(glyph(look, "save", Size::Small, look.color(token::FG_MUTED)))
                            .tooltip(tip("Saved request")),
                    )
                    .child(div().flex_1().min_w_0().truncate().child(format!("Saved {name} to {collection}")))
                    .child(self.stamp(at, look, now, true, cx)),
                Role::Region,
                format!("Saved {name} to {collection}"),
            )
            .into_any_element(),
            SessionRow::Retro { id, stage, characters, retro, why, items, .. } => {
                let head = match stage {
                    RetroStage::AskingAgent => "Asking the agent what got in the way".to_string(),
                    RetroStage::ReadingRecord => "Reading what Fleet kept of the Job".to_string(),
                    RetroStage::Writing => "Writing the retro".to_string(),
                    RetroStage::Written => {
                        let n = items.len();
                        let made = retro.map_or("Retro written".to_string(), |number| format!("Retro {number} written"));
                        if n == 0 {
                            made
                        } else {
                            format!("{made} \u{b7} {n} {}", if n == 1 { "item" } else { "items" })
                        }
                    }
                    RetroStage::Failed => "The retro was not written".to_string(),
                };
                let titles: Vec<String> =
                    if *stage == RetroStage::Written { items.iter().filter_map(|item| item.title.clone()).collect() } else { Vec::new() };
                labelled(
                    div()
                        .id(SharedString::from(format!("retro:{id}")))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_4) + look.length(token::SPACE_1))
                        .py(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_l(look.length(token::BORDER_WIDTH) * 2.)
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .bg(look.color(token::BG_RAISED))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_MUTED))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_2))
                                .child(glyph(look, "rewind", Size::Small, look.color(token::FG_MUTED)))
                                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(head))
                                .when(*stage == RetroStage::Writing && *characters > 0, |line| {
                                    line.child(div().flex_1().flex().justify_end().child(format!("{characters} characters")))
                                }),
                        )
                        .children(
                            why.clone()
                                .filter(|_| *stage == RetroStage::Failed)
                                .map(|why| div().text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(why)),
                        )
                        .children(titles.into_iter().map(|title| div().pl(look.length(token::SPACE_4)).child(title))),
                    Role::Region,
                    "Retro",
                )
                .into_any_element()
            }
            SessionRow::Sent { id, at, text, target, files, tags } => {
                let you = self.you(ix, page, id, text, files, tags, Some(at), None, false, false, None, look, now, cx);
                let chip = div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_SM))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(glyph(look, "square-terminal", Size::Small, look.color(token::FG_MUTED)))
                    .child(target.title.clone());
                let to = div()
                    .flex()
                    .justify_end()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child("Sent to")
                    .child(self.opens(&target.id, format!("sent-to:{id}"), format!("Sent to Job {}", target.title), cx, chip));
                div().flex().flex_col().gap(look.length(token::SPACE_1)).child(you).child(to).into_any_element()
            }
            SessionRow::PlanChanges { id, at, lines, .. } => labelled(
                Self::padded(look, div())
                    .id(SharedString::from(format!("suggest:{id}")))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_3))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::YOU_EDGE))
                    .bg(look.color(token::YOU).opacity(0.14))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(eyebrow(look, "Plan changes"))
                    .child(div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1)).children(lines.iter().map(|line| {
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(line.verb.clone()))
                            .child(div().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).child(line.place.clone()))
                            .children(line.from.clone().map(|from| div().opacity(0.7).line_through().child(from)))
                            .children(line.text.clone())
                    })))
                    .child(self.stamp(at, look, now, true, cx)),
                Role::Region,
                "Changes to the plan",
            )
            .into_any_element(),
            SessionRow::Disputed { id, at, why, spotted, review, .. } => {
                let spotted = match spotted {
                    ipc::DisputeSpotted::Called => "Said by the agent",
                    ipc::DisputeSpotted::Read => "Read by Fleet from the agent's replies",
                };
                let line = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_dashed()
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(
                        div()
                            .id(SharedString::from(format!("disputed-mark:{id}")))
                            .child(glyph(look, "message-square-warning", Size::Small, look.color(token::FG_MUTED)))
                            .tooltip(tip("Disputed")),
                    )
                    .child(div().w_full().text_color(look.color(token::FG_DEFAULT)).child(why.clone()))
                    .child(spotted)
                    .children(review.clone().map(|review| {
                        self.opens(
                            &review,
                            format!("review:{id}"),
                            "Open the review job".into(),
                            cx,
                            div().flex().items_center().text_color(look.color(token::ACCENT)).child("Review job"),
                        )
                    }))
                    .child(self.stamp(at, look, now, true, cx));
                labelled(div().id(SharedString::from(format!("disputed:{id}"))).child(line), Role::Region, "Notice disputed")
                    .into_any_element()
            }
            // The open question is on the window's bar and under Waiting on you, so the thread keeps one line.
            SessionRow::WindowAsk { id, question, answer, .. } => {
                let said = match answer {
                    Some(answer) => format!("Asked in the window: {question} \u{2014} {}", answer.replace('\n', " \u{b7} ")),
                    None => format!("Asked in the window: {question}"),
                };
                quiet_line(id, said, look)
            }
            SessionRow::Draft { id, at, title, text, before, settled, change } => {
                self.draft(ix, id, at, title, text, before.as_deref(), settled.as_ref(), *change, false, page, look, now, cx)
            }
            // A call that waits is the permission card in the panel; once settled the thread keeps one line.
            SessionRow::Ask { id, ask, state, .. } => match asked_said(ask, *state) {
                Some(said) => quiet_line(id, said, look),
                None => div().into_any_element(),
            },
            _ => div().into_any_element(),
        }
    }

    /// A block of output, in a sunken well.
    fn log(&self, text: &str, look: &Look) -> gpui::Div {
        div()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_SUNKEN))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .text_color(look.color(token::FG_MUTED))
            .child(text.trim_end().to_string())
    }

    /// A command the person ran, ended (A3.6): its mark and first line, the last lines it printed
    /// under it, and all of them on a press. A failed one is a card.
    #[allow(clippy::too_many_arguments)]
    fn ran(&self, ix: usize, id: &str, command: &str, code: Option<i32>, tail: &str, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let first = command.lines().next().unwrap_or("").to_string();
        let (icon, ink, word) = match code {
            Some(0) => ("circle-check", token::RUN_PASSED, "Exit 0".to_string()),
            Some(code) => ("circle-x", token::RUN_FAILED, format!("Exit {code}")),
            None => ("circle-x", token::RUN_STOPPED, "Stopped".to_string()),
        };
        let failed = code != Some(0);
        let label = match code {
            Some(code) => format!("Exit {code}: {first}"),
            None => format!("Stopped: {first}"),
        };
        let head = vec![
            div()
                .id(SharedString::from(format!("ran-mark:{id}")))
                .child(glyph(look, icon, Size::Mark, look.color(ink)))
                .tooltip(tip(word.clone()))
                .into_any_element(),
            div().flex_1().min_w_0().truncate().child(first).into_any_element(),
            div()
                .flex_none()
                .text_color(look.color(token::RUN_FAILED))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .child(if failed { word } else { String::new() })
                .into_any_element(),
        ];
        let lines: Vec<&str> = tail.trim_end().lines().collect();
        let peek = lines[lines.len().saturating_sub(3)..].join("\n");
        let open = page.opened.contains(&format!("ran:{id}"));
        let fold = self.fold(ix, format!("ran:{id}"), label, head, Some(self.log(tail, look).into_any_element()), page, look, cx);
        div()
            .flex()
            .flex_col()
            .when(failed, |card| {
                card.px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .bg(look.color(token::RUN_FAILED).opacity(0.07))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::RUN_FAILED).opacity(0.18))
            })
            .child(fold)
            .when(!open && !peek.is_empty(), |card| {
                card.child(
                    div()
                        .mt(look.length(token::SPACE_1))
                        .ml(look.length(token::SPACE_4))
                        .pl(look.length(token::SPACE_2))
                        .border_l(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child(peek),
                )
            })
            .into_any_element()
    }
}

/// One muted line of the thread, as a tool row is drawn.
fn quiet_line(id: &str, said: String, look: &Look) -> AnyElement {
    div()
        .id(SharedString::from(format!("line:{id}")))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_SM))
        .text_color(look.color(token::FG_MUTED))
        .child(said)
        .into_any_element()
}

/// What a settled call says: what was asked, and how it was answered. A call still waiting says nothing.
pub(super) fn asked_said(ask: &ipc::CallInFlight, state: ipc::SessionAskState) -> Option<String> {
    use ipc::SessionAskState::*;
    let how = match state {
        Waiting => return None,
        RanUnasked => "ran without asking",
        AllowedOnce => "allowed once",
        AllowedAndRemembered => "allowed and remembered",
        AlwaysAllowed => "always allowed",
        AlwaysBlocked => "always blocked",
        AllowedForSession => "allowed for this job",
        Refused => "refused",
        Unanswered => "no answer",
        Kept => "kept for you",
        SessionGone => "job ended",
    };
    let command = ask.whole.clone().unwrap_or_else(|| if ask.detail.is_empty() { ask.tool.clone() } else { ask.detail.clone() });
    Some(format!("{command}: {how}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::SessionAskState;

    fn call(detail: &str, whole: Option<&str>) -> ipc::CallInFlight {
        let whole = whole.map_or("null".to_string(), |whole| format!("\"{whole}\""));
        let wire = format!(
            r#"{{"call":"c-1","manifest_id":"armada","asked_at":"2026-11-09T14:03:00Z","tool":"Bash","detail":"{detail}","truncated":false,"rule":"ask","offers":[],"holding_for_seconds":0,"whole":{whole}}}"#
        );
        ipc::decode("CallInFlight", wire.as_bytes()).expect("a call decodes")
    }

    #[test]
    fn a_settled_call_is_one_line_with_what_was_asked_and_how_it_went() {
        let ask = call("rm -rf target", None);
        assert_eq!(asked_said(&ask, SessionAskState::AllowedOnce).as_deref(), Some("rm -rf target: allowed once"));
        assert_eq!(asked_said(&ask, SessionAskState::SessionGone).as_deref(), Some("rm -rf target: job ended"));
    }

    #[test]
    fn a_call_that_waits_says_nothing_and_reads_whole_where_it_was_cut() {
        assert_eq!(asked_said(&call("rm -rf", Some("rm -rf target")), SessionAskState::Waiting), None);
        assert_eq!(asked_said(&call("rm -rf", Some("rm -rf target")), SessionAskState::Refused).as_deref(), Some("rm -rf target: refused"));
        assert_eq!(asked_said(&call("", None), SessionAskState::Refused).as_deref(), Some("Bash: refused"));
    }
}
