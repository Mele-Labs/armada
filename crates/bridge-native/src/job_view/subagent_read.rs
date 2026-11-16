//! A subagent's own thread, read in its tab beside the thread, as Electron's `session-subagent.tsx` draws
//! it: its type and model, the context figure beside them, and the thread drawn as the Job's is. Read
//! again every 1.5 s while it runs, and each time Fleet says its thread moved (`session.subagent`).
//! Esc or the tab's × shut it.

use std::time::Duration;

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString, Task, Window};
use ipc::{RingBand, SessionRow, SessionSubagentChanged, ThreadItem, ToolSubagent, ToolSubagentState};
use serde::Deserialize;

use super::rows::{mono, tip};
use super::JobView;
use crate::context_ring::{ContextRing, Fill};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// How often a running subagent's transcript is read again (`SUBAGENT_EVERY_MS`).
const EVERY: Duration = Duration::from_millis(1500);

/// How many subagent threads are kept past the sheet that read them (`SUBAGENTS_KEPT`).
const KEPT: usize = 20;

/// Where the sheet numbers the subagent's items: far past any list element, so a fold one of them
/// opens measures nothing in the Job's list.
const ITEMS: usize = usize::MAX / 4;

/// A subagent's thread as `GET /sessions/subagent` answers it. `items` are its rows folded as the
/// Job's thread is; until Fleet serves them, `rows` are drawn one by one.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SubagentRead {
    #[serde(default)]
    items: Vec<ThreadItem>,
    #[serde(default)]
    rows: Vec<SessionRow>,
    finished: bool,
    #[serde(default)]
    report: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    context_tokens: Option<u64>,
    #[serde(default)]
    context_window: Option<u64>,
}

impl SubagentRead {
    fn items(&self) -> Vec<ThreadItem> {
        if !self.items.is_empty() {
            return self.items.clone();
        }
        self.rows.iter().map(|row| ThreadItem::Row { row: row.clone() }).collect()
    }

    /// The figure beside the title, as `ContextFigure` draws one with no breakdown and no Compact.
    fn fill(&self) -> Option<Fill> {
        let (tokens, window) = (self.context_tokens?, self.context_window?);
        if window == 0 {
            return None;
        }
        let percent = ((tokens as f64 / window as f64) * 100.).round() as u32;
        Some(Fill { tokens, window, percent: percent.min(100) as u8, band: RingBand::of(percent), breakdown: None, compact: None, refused: None })
    }
}

/// The name a model is shown by, as `modelSaid` reads it: a full id as "Opus 5.5" or "Sonnet 3.5",
/// an alias capitalised, and anything else as it came.
pub(crate) fn model_said(model: &str) -> String {
    let family = |word: &str| matches!(word, "opus" | "sonnet" | "haiku");
    let digits = |word: &str, most: usize| !word.is_empty() && word.len() <= most && word.bytes().all(|b| b.is_ascii_digit());
    let named = |family: &str, version: String| format!("{}{} {version}", family[..1].to_uppercase(), &family[1..]);
    if family(model) {
        return named(model, String::new()).trim_end().to_string();
    }
    let mut parts: Vec<&str> = model.split('-').collect();
    if parts.last().is_some_and(|last| last.len() == 8 && digits(last, 8)) {
        parts.pop();
    }
    let vendor = parts.first().is_some_and(|first| !first.is_empty() && first.bytes().all(|b| b.is_ascii_lowercase()));
    if !vendor {
        return model.to_string();
    }
    let version = |major: &str, minor: Option<&str>| minor.map_or_else(|| major.to_string(), |minor| format!("{major}.{minor}"));
    match &parts[1..] {
        [kind, major] if family(kind) && digits(major, usize::MAX) => named(kind, version(major, None)),
        [kind, major, minor] if family(kind) && digits(major, usize::MAX) && digits(minor, 2) => named(kind, version(major, Some(minor))),
        [major, kind] if family(kind) && digits(major, usize::MAX) => named(kind, version(major, None)),
        [major, minor, kind] if family(kind) && digits(major, usize::MAX) && digits(minor, 2) => named(kind, version(major, Some(minor))),
        _ => model.to_string(),
    }
}

impl JobView {
    /// A press on a subagent's card or row: its thread opens as a tab beside the Job's, from the last read of it
    /// at once, then read again while it runs.
    pub(super) fn read_subagent(&mut self, subagent: ToolSubagent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let id = subagent.id.clone();
        self.subagents_opened.insert(Self::subagent_key(&job, &id), subagent);
        self.open_subagent_tab(&id, window, cx);
    }

    pub(super) fn subagent_key(job: &str, id: &str) -> String {
        format!("{job}/{id}")
    }

    fn keep_subagent(&mut self, key: String, read: SubagentRead) {
        self.subagents_read.retain(|(one, _)| *one != key);
        self.subagents_read.push((key, read));
        let over = self.subagents_read.len().saturating_sub(KEPT);
        self.subagents_read.drain(..over);
    }

    /// Read the subagent's thread, and where it is `running`, again every [`EVERY`] until a read says
    /// it finished. On the mock the scenario's own read stands for Fleet's.
    fn subagent_poll(&mut self, job: &SharedString, subagent: &str, running: bool, cx: &mut Context<Self>) -> Option<Task<()>> {
        let key = Self::subagent_key(job, subagent);
        let Some(port) = self.port(cx) else {
            if let Some(read) = crate::mock::subagent::<SubagentRead>(job, subagent) {
                self.keep_subagent(key, read);
            }
            return None;
        };
        let path = format!("/sessions/subagent?session_id={job}&subagent_id={subagent}");
        Some(cx.spawn(async move |this, cx| loop {
            let path = path.clone();
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<SubagentRead>("SessionSubagent", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let finished = answer.as_ref().is_ok_and(|read| read.finished);
            let kept = this.update(cx, |this, cx| {
                if let Ok(read) = answer {
                    this.keep_subagent(key.clone(), read);
                    cx.notify();
                }
            });
            if kept.is_err() || !running || finished {
                return;
            }
            cx.background_executor().timer(EVERY).await;
        }))
    }

    /// A subagent's thread as the Job's is drawn, from the last read of it; the report alone only
    /// where no row could be read (`SubagentThread`). Nothing until a read arrives.
    pub(super) fn subagent_thread(&self, job: &SharedString, subagent: &str, window: &mut Window, cx: &Context<Self>) -> Option<AnyElement> {
        let look = cx.look();
        let look = &look;
        let key = Self::subagent_key(job, subagent);
        let read = self.subagents_read.iter().find(|(one, _)| *one == key).map(|(_, read)| read)?;
        let page = self.pages.get(job)?;
        let items = read.items();
        let now = chrono::Local::now();
        if items.is_empty() {
            return read.report.clone().map(|report| div().p(look.length(token::SPACE_4)).child(self.prose(format!("subagent-report:{key}"), &report, look, cx)).into_any_element());
        }
        let drawn: Vec<AnyElement> = items
            .iter()
            .enumerate()
            .filter_map(|(at, item)| {
                let after = items.get(at + 1);
                self.item(ITEMS + at, item, false, after, page, look, now, window, cx)
            })
            .collect();
        Some(div().flex().flex_col().gap(look.length(token::SPACE_4)).p(look.length(token::SPACE_4)).children(drawn).into_any_element())
    }

    /// Read a subagent's thread for its tab beside the thread, and again while it runs.
    pub(super) fn watch_subagent(&mut self, subagent: &ToolSubagent, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let key = Self::subagent_key(&job, &subagent.id);
        if let Some(poll) = self.subagent_poll(&job, &subagent.id, subagent.state == ToolSubagentState::Running, cx) {
            self.subagent_tab_polls.insert(key, poll);
        }
    }

    /// Fleet says these subagents' threads moved: each one open as a tab is read once more, beside any
    /// re-read already running for it.
    pub(super) fn subagents_moved(&mut self, moved: &[SessionSubagentChanged], cx: &mut Context<Self>) {
        for one in moved {
            let job = SharedString::from(one.session_id.as_str().to_string());
            let id = one.subagent_id.as_str();
            if self.subagent_tab_open(&job, id) {
                if let Some(read) = self.subagent_poll(&job, id, false, cx) {
                    read.detach();
                }
            }
        }
    }

    /// What stands over a subagent's thread in its tab: its type and its model, each left out until it is known
    /// (`SubagentMarks`), and the context figure beside them. Nothing where there is neither.
    pub(super) fn subagent_head(&self, job: &SharedString, subagent: &ToolSubagent, look: &Look) -> Option<AnyElement> {
        let key = Self::subagent_key(job, &subagent.id);
        let read = self.subagents_read.iter().find(|(one, _)| *one == key).map(|(_, read)| read);
        let model = read.and_then(|read| read.model.as_deref()).map(model_said);
        let marks = (subagent.agent_type.is_some() || model.is_some()).then(|| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x(look.length(token::SPACE_3))
                .gap_y(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(look.color(token::FG_SUBTLE))
                .children(subagent.agent_type.clone().map(|kind| {
                    labelled(div().id("subagent-type").font_family(mono(look)).child(kind.clone()).tooltip(tip("Agent type")), Role::Label, kind)
                }))
                .children(model.map(|model| labelled(div().id("subagent-model").child(model.clone()).tooltip(tip("Model")), Role::Label, model)))
        });
        let figure = read.and_then(SubagentRead::fill).map(|fill| ContextRing::new(SharedString::from(format!("subagent-context:{key}")), Some(fill)));
        (marks.is_some() || figure.is_some()).then(|| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_2))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .children(marks)
                .children(figure)
                .into_any_element()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::model_said;

    #[test]
    fn a_model_is_named_as_electron_names_it() {
        assert_eq!(model_said("claude-opus-5-5"), "Opus 5.5");
        assert_eq!(model_said("claude-3-5-sonnet-20241022"), "Sonnet 3.5");
        assert_eq!(model_said("claude-haiku-5-5-20260101"), "Haiku 5.5");
        assert_eq!(model_said("opus"), "Opus");
        assert_eq!(model_said("gpt-x"), "gpt-x");
    }
}
