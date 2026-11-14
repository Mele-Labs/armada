//! A web page a Job showed or serves, opened from here: a walk's tab, and the live page in Now, which is the dev
//! server the Job started or the page it showed last (`SessionLive`). Both are drawn in a tab of the Job's panes
//! (`web_walk.rs`). A Job on this Mac opens its address as it is. A Job another Mac holds asks Fleet first,
//! `POST /sessions/other_mac_page`, whose answer is an address on this Mac's loopback that is drawn in place of
//! the Job's own (`open_other_mac_page.toml`).
//!
//! Until Fleet answers, and where it refuses, the tab draws the walk's name, brief and address with Open, and
//! what happened in its words: since when that Mac sleeps, or the refusal as Fleet or the link says it.

use std::collections::HashMap;

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString, Window};
use ipc::{OpenOtherMacPage, OtherMacPageOpened, SessionId, WireError, WireValue};

use super::now::lead;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The refusal that says the Mac holding the Job is asleep, with `since` in Unix seconds.
const ASLEEP: &str = "fleet.other_mac_asleep";

/// Where a page not yet drawn stands.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum PageOpen {
    Opening,
    /// That Mac fell asleep at `since`, in Unix seconds; `said` is Fleet's sentence.
    Asleep { said: String, since: i64 },
    Refused(String),
}

/// Each page's last Open that did not open it, by Job and the page's address as the Job named it.
#[derive(Default)]
pub(super) struct PagesOpen(HashMap<(String, String), PageOpen>);

/// What a refusal of `open_other_mac_page` comes to: asleep with its time, else its sentence.
fn refused(status: u16, bytes: &[u8]) -> PageOpen {
    let Ok(error) = ipc::decode::<WireError>("WireError", bytes) else { return PageOpen::Refused(format!("Fleet answered {status}")) };
    match error.fields.get("since") {
        Some(WireValue::Int(since)) if error.code == ASLEEP => PageOpen::Asleep { said: error.message, since: *since },
        _ => PageOpen::Refused(error.message),
    }
}

/// What a walk's mock Fleet answers for a page of Air's Job: asleep while Air sleeps, the link's sentence while it
/// does not carry, else an address standing in for the one on this Mac's loopback.
fn mock_answer(cx: &gpui::App) -> Result<String, PageOpen> {
    if let Some(since) = crate::pairing::chip(Some("air"), cx).and_then(|chip| chip.asleep_since) {
        return Err(PageOpen::Asleep { said: "Air is asleep. Its pages open when it wakes.".into(), since });
    }
    if crate::terminal::shells::walk_link_down(cx) {
        return Err(PageOpen::Refused(crate::terminal::shells::UNREACHED.into()));
    }
    Ok(crate::mock::AIR_PAGE.to_string())
}

impl JobView {
    /// What the last Open of Job `job`'s page `url` came to, where it did not open it.
    pub(super) fn page_open(&self, job: &str, url: &str) -> Option<&PageOpen> {
        self.pages_open.0.get(&(job.to_string(), url.to_string()))
    }

    /// Open Job `job`'s page `url` in its tab: as it is for a Job of this Mac, from the address Fleet answers for one
    /// another Mac holds. A page already drawn is left as it is.
    pub(super) fn open_page(&mut self, job: &str, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        let key = (job.to_string(), url.to_string());
        if self.page_open(job, url) == Some(&PageOpen::Opening) || self.web_page_drawn(job, url) {
            return;
        }
        if self.mac_of(job, cx).is_none() {
            return self.page_answered(key, Ok(url.to_string()), window, cx);
        }
        if self.fleet.is_none() {
            return self.page_answered(key, mock_answer(cx), window, cx);
        }
        let Some(port) = self.port(cx) else {
            return self.page_answered(key, Err(PageOpen::Refused("Fleet is not connected".into())), window, cx);
        };
        self.pages_open.0.insert(key.clone(), PageOpen::Opening);
        cx.notify();
        let body = ipc::encode(&OpenOtherMacPage { session_id: SessionId::carried(job.to_string()), url: url.to_string() });
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let body = body.map_err(|why| PageOpen::Refused(format!("{why:?}")))?;
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| PageOpen::Refused(why.to_string()))?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, "/sessions/other_mac_page", body)).map_err(PageOpen::Refused)?;
                    if !(200..300).contains(&status) {
                        return Err(refused(status, &bytes));
                    }
                    ipc::decode::<OtherMacPageOpened>("OtherMacPageOpened", &bytes).map(|opened| opened.url).map_err(|why| PageOpen::Refused(why.to_string()))
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| this.page_answered(key, answer, window, cx));
        })
        .detach();
    }

    /// An Open came to `answer`: the page is drawn from the address, or what stopped it is kept for its tab to say.
    fn page_answered(&mut self, key: (String, String), answer: Result<String, PageOpen>, window: &mut Window, cx: &mut Context<Self>) {
        match answer {
            Ok(here) => {
                self.pages_open.0.remove(&key);
                self.show_web_page(&key.0.clone().into(), &key.1, &here, window, cx);
            }
            Err(open) => {
                self.pages_open.0.insert(key, open);
            }
        }
        cx.notify();
    }

    /// What the last Open came to, as a glyph with its tooltip and a sentence read as a status or an alert. Nothing
    /// where it has not been tried, or has opened.
    fn page_state(&self, job: &str, url: &str, look: &Look) -> Option<(AnyElement, String, Role)> {
        let key = format!("page:{url}");
        let (icon, ink, tip, said, role, pulse) = match self.page_open(job, url)? {
            PageOpen::Opening => ("circle-dot", token::STEP_RUNNING, "Opening".to_string(), "Opening the page".to_string(), Role::Status, true),
            PageOpen::Asleep { said, since } => ("moon", token::FG_MUTED, crate::pairing::asleep_said(*since), said.clone(), Role::Status, false),
            PageOpen::Refused(why) => ("triangle-alert", token::STATUS_AWAITING_REVIEW, "Not opened".to_string(), why.clone(), Role::Alert, false),
        };
        Some((lead(look, &key, icon, ink, &tip, pulse), said, role))
    }

    /// The line under a page's row in Now: what the last Open came to, in a sentence, else nothing.
    pub(super) fn page_words(&self, job: &str, url: &str) -> Option<String> {
        match self.page_open(job, url)? {
            PageOpen::Opening => None,
            PageOpen::Asleep { said, .. } => Some(said.clone()),
            PageOpen::Refused(why) => Some(why.clone()),
        }
    }

    /// What the last Open came to, in a line beside Open in the page's tab.
    pub(super) fn page_said(&self, job: &str, url: &str, look: &Look) -> Option<AnyElement> {
        let (mark, said, role) = self.page_state(job, url, look)?;
        Some(
            labelled(
                div()
                    .id(SharedString::from(format!("page-said:{url}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .min_w_0()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(mark)
                    .child(div().min_w_0().child(SharedString::from(said.clone()))),
                role,
                said,
            )
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_asleep_refusal_keeps_its_time_and_any_other_its_sentence() {
        let asleep = br#"{"code":"fleet.other_mac_asleep","message":"Air is asleep. Its pages open when it wakes.","run_id":"r","fields":{"since":1763024000,"mac":"air"},"chain":[]}"#;
        assert_eq!(refused(409, asleep), PageOpen::Asleep { said: "Air is asleep. Its pages open when it wakes.".into(), since: 1763024000 });
        let gone = br#"{"code":"fleet.no_such_session","message":"No Job s-1 is held here.","run_id":"r","fields":{},"chain":[]}"#;
        assert_eq!(refused(404, gone), PageOpen::Refused("No Job s-1 is held here.".into()));
        assert_eq!(refused(500, b"not json"), PageOpen::Refused("Fleet answered 500".into()));
    }
}
