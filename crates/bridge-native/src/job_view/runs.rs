//! The items that group rows: runs of calls, subagents, Check runs and Fleet turns (`job-view.md`,
//! A3 to A6).

use std::rc::Rc;

use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, Context, Role, SharedString};
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

/// The clock a running call counts on: the wall's, or on mock data the thread's newest moment, moving
/// on from when the data was read, so a running call reads its own seconds and not the fixture's age.
/// Each mock thread's newest moment, by Job, and when the data was read.
static MOCK_CLOCK: std::sync::OnceLock<(std::collections::HashMap<String, DateTime<Utc>>, std::time::Instant)> = std::sync::OnceLock::new();

pub(super) fn start_mock_clock(threads: &std::collections::HashMap<String, ipc::JobThread>) {
    let newest = threads
        .iter()
        .filter_map(|(job, thread)| {
            let wire = ipc::encode(thread).ok()?;
            let newest = wire.split("\"at\":\"").skip(1).filter_map(|rest| rest.split('"').next().and_then(|at| DateTime::parse_from_rfc3339(at).ok())).map(|at| at.with_timezone(&Utc)).max()?;
            Some((job.clone(), newest))
        })
        .collect();
    let _ = MOCK_CLOCK.set((newest, std::time::Instant::now()));
}

pub(super) fn running_now(job: &str) -> DateTime<Utc> {
    match MOCK_CLOCK.get().and_then(|(newest, read)| newest.get(job).map(|at| (*at, read))) {
        Some((newest, read)) => newest + chrono::Duration::from_std(read.elapsed()).unwrap_or_default(),
        None => Utc::now(),
    }
}

/// One run of the band down a running call's rail.
const RAIL_MS: u64 = 1400;

thread_local! {
    /// When each running call's rail was first drawn, by its key.
    static RAIL_FROM: std::cell::RefCell<std::collections::HashMap<String, std::time::Instant>> = std::cell::RefCell::default();
}

/// Where the band is in its run, 0 to 1, from when the call's rail was first drawn: a new call's band
/// starts at the top, and a list item's redraw, which resets an animation's own start, moves nothing.
fn rail_phase(key: &str) -> f32 {
    let from = RAIL_FROM.with(|from| *from.borrow_mut().entry(key.to_string()).or_insert_with(std::time::Instant::now));
    (from.elapsed().as_millis() as u64 % RAIL_MS) as f32 / RAIL_MS as f32
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
/// `stop` ends the first line: the live fold's newest call carries Stop, as Electron's does.
#[allow(clippy::too_many_arguments)]
fn tool_row(job: &str, key: &str, call: Call, at: &ipc::Instant, end: Option<&ipc::Instant>, running: bool, stop: Option<AnyElement>, look: &Look) -> AnyElement {
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
        // The sketch's 35% read as a tint on a 3pt rail; at 80% it reads as a band at a glance.
        let band = look.color(token::FG_DEFAULT).opacity(0.8);
        let from = key.to_string();
        rail = rail.child(
            div().absolute().left_0().w_full().h(gpui::relative(0.4)).bg(band).with_animation(
                SharedString::from(format!("rail:{key}")),
                Animation::new(Duration::from_millis(RAIL_MS)).repeat(),
                move |band, _| band.top(gpui::relative(-0.4 + 1.4 * gpui::ease_in_out(rail_phase(&from)))),
            ),
        );
    }
    let (since, job) = (at.clone(), job.to_string());
    let time = div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE));
    let time = if running {
        // The count goes on each frame the row is drawn.
        time.with_animation(SharedString::from(format!("count:{key}")), Animation::new(Duration::from_secs(1)).repeat(), move |time, _| {
            time.child(elapsed_words(since.as_str(), &running_now(&job).to_rfc3339()).unwrap_or_default())
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
        .child(time)
        .children(stop);
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
            .child(div().flex_1().min_w_0().flex().flex_col().pl(look.length(token::SPACE_3)).pr(look.length(token::SPACE_2)).py(look.length(token::SPACE_1)).child(first).children(second)),
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
        let job = self.shown.clone().unwrap_or_default();
        let lines = rows.iter().enumerate().map(|(at, row)| {
            let running = live && at == last;
            let end = rows.get(at + 1).map(SessionRow::at).or(end);
            let one = match row {
                SessionRow::WindowAsk { question, .. } => Call { kind: Kind::Other, phrase: question.clone(), working_on: None },
                _ => Call {
                    kind: calls::kind_of(&texts[at]),
                    // Fleet's own phrase for the call, where its input carried one; else the one made from its line.
                    phrase: match row {
                        SessionRow::Tool { description: Some(said), .. } if !said.trim().is_empty() => said.clone(),
                        _ => calls::call_line(&texts[at], running),
                    },
                    working_on: calls::working_on(&texts[at]).filter(|_| running),
                },
            };
            let stop = running.then(|| self.stop_button(look, cx));
            let line = tool_row(&job, &format!("{id}:{at}"), one, row.at(), if running { None } else { end }, running, stop, look);
            // In a folded group each call is a Command, as Electron's: it takes focus and a right press offers Teach.
            if live || texts[at].is_empty() {
                return line;
            }
            let key = format!("call-command:{id}:{at}");
            let spot = self.spot(page, &key, cx);
            let command = div()
                .id(SharedString::from(key))
                .track_focus(&spot.focus)
                .rounded(look.length(token::RADIUS_SM))
                .focus_visible(|style| style.bg(look.color(token::BG_HOVER)))
                .child(line);
            self.menu_on(command, &spot, "Command", spot.focus.clone(), Rc::new(teach_entries), self.teach_pick("command", texts[at].clone(), Some(row.at().clone())), cx).into_any_element()
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

    /// The subagent an `Agent` call started, as a take A call row: running, the agent's moving icon, its
    /// brief, a counting time and its latest call on a second line; done, one grey line with the time it
    /// took. A press opens its own thread.
    pub(super) fn subagent(&self, id: &str, subagent: &ToolSubagent, look: &Look, cx: &Context<Self>) -> AnyElement {
        let task = subagent.task.clone();
        let word = match subagent.state {
            ToolSubagentState::Running => "running",
            ToolSubagentState::Finished => "finished",
            ToolSubagentState::CutOff => "cut off",
        };
        self.subagent_line(&format!("subagent:{id}"), subagent, format!("Subagent {task}, {word}"), look, cx)
    }

    /// One subagent's take A row, pressed to open its thread, labelled `said`. The thread's row and the
    /// card at the foot both draw it.
    pub(super) fn subagent_line(&self, key: &str, subagent: &ToolSubagent, said: String, look: &Look, cx: &Context<Self>) -> AnyElement {
        let running = subagent.state == ToolSubagentState::Running;
        let one = Call { kind: Kind::Agent, phrase: subagent.task.clone(), working_on: subagent.latest.clone().filter(|_| running) };
        let state = match subagent.state {
            ToolSubagentState::Running => "Subagent running",
            ToolSubagentState::Finished => "Subagent finished",
            ToolSubagentState::CutOff => "Subagent cut off",
        };
        let tooltip = match &subagent.agent_type {
            Some(kind) => format!("{state} \u{b7} {kind}"),
            None => state.to_string(),
        };
        // A cut-off one is marked as the registry's stopped state, so it never reads as finished.
        let cut = (subagent.state == ToolSubagentState::CutOff).then(|| {
            div().id(SharedString::from(format!("{key}:cut"))).flex_none().child(glyph(look, "power", Size::Small, look.color(token::STATUS_KILLED))).tooltip(tip("Cut off")).into_any_element()
        });
        let opens = subagent.clone();
        labelled(
            div()
                .id(SharedString::from(key.to_string()))
                .cursor_pointer()
                .rounded(look.length(token::RADIUS_SM))
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip(tooltip))
                .on_click(cx.listener(move |this, _, window, cx| this.read_subagent(opens.clone(), window, cx)))
                .child(tool_row(self.shown.as_deref().unwrap_or_default(), key, one, &subagent.since, subagent.ended.as_ref(), running, cut, look)),
            Role::Button,
            said,
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
                SessionRow::Message { id, text, .. } => self.prose(format!("fleet:{id}"), text, look, cx),
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
