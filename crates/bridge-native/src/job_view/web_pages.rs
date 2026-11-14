//! A web page a Job showed or serves, opened from here: a walk's tab, and the live page in Now, which is the dev
//! server the Job started or the page it showed last (`SessionLive`). A Job on this Mac opens its address as it
//! is. A Job another Mac holds opens through `POST /sessions/other_mac_page`, whose answer is an address on this
//! Mac's loopback, opened in place of the Job's own (`open_other_mac_page.toml`).
//!
//! **Native draws no web page yet**: the address goes to the browser until Chrome draws it here
//! (`docs/concepts/native-bridge.md`, *Chrome for all web content*). Where that Mac is asleep, nothing opens and the
//! row says since when; any other refusal is said in Fleet's or the link's own words.

use std::collections::HashMap;

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString};
use ipc::{OpenOtherMacPage, OtherMacPageOpened, SessionId, WireError, WireValue};

use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The refusal that says the Mac holding the Job is asleep, with `since` in Unix seconds.
const ASLEEP: &str = "fleet.other_mac_asleep";

/// What the last Open of a page came to.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum PageOpen {
    Opening,
    Opened,
    /// That Mac fell asleep at this time, in Unix seconds.
    Asleep(i64),
    Refused(String),
}

/// Each page's last Open, by Job and the page's address as the Job named it.
#[derive(Default)]
pub(super) struct PagesOpen(HashMap<(String, String), PageOpen>);

/// What a refusal of `open_other_mac_page` comes to: asleep with its time, else its sentence.
fn refused(status: u16, bytes: &[u8]) -> PageOpen {
    let Ok(error) = ipc::decode::<WireError>("WireError", bytes) else { return PageOpen::Refused(format!("Fleet answered {status}")) };
    match error.fields.get("since") {
        Some(WireValue::Int(since)) if error.code == ASLEEP => PageOpen::Asleep(*since),
        _ => PageOpen::Refused(error.message),
    }
}

/// What a walk's mock Fleet answers for a page of Air's Job: asleep while Air sleeps, the link's sentence while it
/// does not carry, else opened, with nothing handed to the browser.
fn mock_answer(job: &str, cx: &gpui::App) -> PageOpen {
    let air = crate::pairing::chip(Some("air"), cx);
    if !crate::mock::AIR_JOBS.contains(&job) {
        return PageOpen::Opened;
    }
    match air.and_then(|chip| chip.asleep_since) {
        Some(since) => PageOpen::Asleep(since),
        None if crate::terminal::shells::walk_link_down(cx) => PageOpen::Refused(crate::terminal::shells::UNREACHED.into()),
        None => PageOpen::Opened,
    }
}

impl JobView {
    /// What the last Open of Job `job`'s page `url` came to.
    pub(super) fn page_open(&self, job: &str, url: &str) -> Option<&PageOpen> {
        self.pages_open.0.get(&(job.to_string(), url.to_string()))
    }

    /// Open Job `job`'s page `url`: as it is for a Job of this Mac, through the link for one another Mac holds.
    pub(super) fn open_page(&mut self, job: &str, url: &str, cx: &mut Context<Self>) {
        let key = (job.to_string(), url.to_string());
        if self.page_open(job, url) == Some(&PageOpen::Opening) {
            return;
        }
        if self.fleet.is_none() {
            let answer = mock_answer(job, cx);
            self.pages_open.0.insert(key, answer);
            cx.notify();
            return;
        }
        if self.mac_of(job, cx).is_none() {
            cx.open_url(url);
            self.pages_open.0.insert(key, PageOpen::Opened);
            cx.notify();
            return;
        }
        let Some(port) = self.port(cx) else {
            self.pages_open.0.insert(key, PageOpen::Refused("Fleet is not connected".into()));
            cx.notify();
            return;
        };
        self.pages_open.0.insert(key.clone(), PageOpen::Opening);
        cx.notify();
        let body = ipc::encode(&OpenOtherMacPage { session_id: SessionId::carried(job.to_string()), url: url.to_string() });
        let asking = cx.background_executor().spawn(async move {
            let body = body.map_err(|why| PageOpen::Refused(format!("{why:?}")))?;
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| PageOpen::Refused(why.to_string()))?;
            let (status, bytes) = runtime.block_on(crate::transport::post(port, "/sessions/other_mac_page", body)).map_err(PageOpen::Refused)?;
            if !(200..300).contains(&status) {
                return Err(refused(status, &bytes));
            }
            ipc::decode::<OtherMacPageOpened>("OtherMacPageOpened", &bytes).map(|opened| opened.url).map_err(|why| PageOpen::Refused(why.to_string()))
        });
        cx.spawn(async move |this, cx| {
            let answer = asking.await;
            let _ = this.update(cx, |this, cx| {
                let open = match answer {
                    // The answered address opens in place of the Job's own; a refusal opens nothing.
                    Ok(here) => {
                        cx.open_url(&here);
                        PageOpen::Opened
                    }
                    Err(open) => open,
                };
                this.pages_open.0.insert(key, open);
                cx.notify();
            });
        })
        .detach();
    }

    /// The words for what the last Open came to, and the role they are read with: since when that Mac sleeps, in
    /// his time format, or the refusal.
    pub(super) fn page_words(&self, job: &str, url: &str) -> Option<(String, Role)> {
        match self.page_open(job, url)? {
            PageOpen::Asleep(since) => Some((crate::pairing::asleep_said(*since), Role::Status)),
            PageOpen::Refused(why) => Some((why.clone(), Role::Alert)),
            PageOpen::Opening | PageOpen::Opened => None,
        }
    }

    /// What the last Open came to, in a line under the page's row: since when that Mac sleeps, or the refusal.
    /// Nothing while it opens or once it has.
    pub(super) fn page_said(&self, job: &str, url: &str, look: &Look) -> Option<AnyElement> {
        let (said, role) = self.page_words(job, url)?;
        Some(
            labelled(
                div()
                    .id(SharedString::from(format!("page-said:{url}")))
                    .min_w_0()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(said.clone())),
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
        assert_eq!(refused(409, asleep), PageOpen::Asleep(1763024000));
        let gone = br#"{"code":"fleet.no_such_session","message":"No Job s-1 is held here.","run_id":"r","fields":{},"chain":[]}"#;
        assert_eq!(refused(404, gone), PageOpen::Refused("No Job s-1 is held here.".into()));
        assert_eq!(refused(500, b"not json"), PageOpen::Refused("Fleet answered 500".into()));
    }
}
