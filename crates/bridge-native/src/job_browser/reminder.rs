//! The thread's line where the Job reached for a browser outside Bridge: a sentence and a press that shows the Job's own and tells
//! the Job to use it (Classic's `useBrowserReminders`, `job-browser/JobBrowserPane.tsx`).
//!
//! Fleet keeps the reminder as a row, where it was written, and nothing says it was answered: **the thread does**, in the owner's
//! message after it that holds the words the press sends.

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString, Window};
use ipc::{SessionRow, SessionVoice, ThreadItem};

use super::super::acts::{button, Tone};
use super::super::rows::{glyph, Size};
use super::super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};

/// What the press tells the Job, as the owner's message: where the page it left was, when the reminder knew.
pub fn said(address: Option<&str>) -> String {
    match address {
        Some(address) => format!("Use the Job browser for this. You were on {address}."),
        None => "Use the Job browser for this.".to_string(),
    }
}

/// Whether the owner's words after item `ix` are the ones the press sends: a later message from him, exact.
pub fn answered(items: &[ThreadItem], ix: usize, said: &str) -> bool {
    items.iter().skip(ix + 1).any(|item| match item {
        ThreadItem::Row { row: SessionRow::Message { from: SessionVoice::You, text, .. } } => text == said,
        _ => false,
    })
}

impl JobView {
    /// The line for a reminder row, item `ix` of the thread of the Job shown.
    pub(in super::super) fn browser_reminder(&self, ix: usize, id: &str, tool: &str, address: Option<&str>, _page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let words = said(address);
        let sent = self.browsers.told.contains(id)
            || self.shown.as_ref().and_then(|job| self.thread(job, cx)).is_some_and(|held| answered(&held.items, ix, &words));
        let sentence = format!("Opened a browser outside Bridge with {tool}. This job has a Job browser.");
        let end = if sent {
            labelled(div().id(SharedString::from(format!("browser-reminder-sent:{id}"))).flex_none().child("Sent to the job"), Role::Status, "Sent to the job").into_any_element()
        } else {
            let key = id.to_string();
            labelled(
                button(look, SharedString::from(format!("browser-reminder-open:{id}")), "Open Job browser", Tone::Secondary, false, None, true)
                    .tab_stop(true)
                    .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                    .on_click(cx.listener(move |this, _, window, cx| this.tell_to_use_the_browser(&key, window, cx))),
                Role::Button,
                "Open Job browser",
            )
            .into_any_element()
        };
        labelled(
            div()
                .id(SharedString::from(format!("browser-reminder:{id}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph(look, "globe", Size::Small, look.color(token::FG_SUBTLE)))
                .child(div().flex_1().min_w_0().whitespace_normal().child(SharedString::from(sentence.clone())))
                .child(end),
            Role::Note,
            format!("Browser outside Bridge: {tool}"),
        )
        .into_any_element()
    }

    /// The press: the Job browser comes to the front, and the Job is told once to use it.
    fn tell_to_use_the_browser(&mut self, row: &str, window: &mut Window, cx: &mut Context<Self>) {
        // A second press before the line redraws would send it again.
        if !self.browsers.told.insert(row.to_string()) {
            return;
        }
        let Some(job) = self.shown.clone() else { return };
        let address = self.thread(&job, cx).and_then(|held| {
            held.items.iter().find_map(|item| match item {
                ThreadItem::Row { row: SessionRow::BrowserReminder { id, address, .. } } if id == row => Some(address.clone()),
                _ => None,
            })
        });
        self.add_tab("browser", window, cx);
        // A Job with no Fleet behind it (the mock) is sent nothing.
        if self.fleet.is_some() {
            let body = ipc::SendSessionMessageBody {
                message: ipc::SendSessionMessage {
                    session_id: ipc::SessionId::carried(job.to_string()),
                    text: said(address.flatten().as_deref()),
                    attachments: Vec::new(),
                    mentions: Vec::new(),
                    sketches: Vec::new(),
                },
                aim: None,
            };
            let done: super::super::wire::Done = Box::new(|_, _, _, _| {});
            self.post("/sessions/message".into(), ipc::encode(&body), window, cx, done);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn you(id: &str, text: &str) -> ThreadItem {
        let wire = format!(r#"{{"kind":"row","row":{{"kind":"message","id":"{id}","at":"2026-10-28T10:01:00Z","from":{{"kind":"you"}},"text":"{text}"}}}}"#);
        ipc::decode("an item", wire.as_bytes()).expect("an item")
    }

    fn agent(id: &str, text: &str) -> ThreadItem {
        let wire = format!(r#"{{"kind":"row","row":{{"kind":"message","id":"{id}","at":"2026-10-28T10:01:00Z","from":{{"kind":"agent"}},"text":"{text}"}}}}"#);
        ipc::decode("an item", wire.as_bytes()).expect("an item")
    }

    #[test]
    fn the_press_says_where_the_job_was_only_when_the_reminder_knew() {
        assert_eq!(said(Some("http://localhost:5173/checkout")), "Use the Job browser for this. You were on http://localhost:5173/checkout.");
        assert_eq!(said(None), "Use the Job browser for this.");
    }

    #[test]
    fn a_reminder_is_answered_by_the_owners_exact_words_after_it_and_by_nothing_else() {
        let words = said(Some("http://localhost:5173/"));
        let items = vec![agent("a0", "Opening it"), you("m0", &words), agent("a1", "Done")];
        assert!(!answered(&items, 2, &words), "the owner's message before the row answers nothing");
        assert!(answered(&items, 0, &words));
        assert!(!answered(&[agent("a2", "ok"), you("m1", "Use the Job browser for this."), agent("a4", "ok")], 0, &words), "other words are not the press");
        assert!(!answered(&[agent("a2", "ok"), agent("a3", &words)], 0, &words), "the agent saying them is not the owner");
    }
}
