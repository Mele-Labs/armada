//! Moving a Job to his other Mac, and bringing one here (`move_job_to_mac`; `docs/practices/protocol/
//! his-other-macs-fleets-half.md`, *Moving a Job between Macs*).
//!
//! **One call, two directions.** A Job this Mac holds names the Mac it goes to; a Job the other Mac holds is
//! asked with no Mac, and comes here. Fleet answers the Job's row with `hosted.moving_to_mac` set while the move
//! waits for the turn to end, and the rows follow the records it publishes: the row on the Mac it left ends with
//! `end_reason` `moved_to_mac`, and the other Mac's row of the same id stands in its place.
//!
//! Every door asks [`offer`] what the act reads and whether it is on, and [`crate::jobs::JobsList::move_to_mac`]
//! to make it: the row's menu, `M`, the palette's panel and the Job head. A refusal is said in a toast in Fleet's
//! own sentence, as a refused snooze is.

use gpui::App;
use ipc::{Mac, MacLink, SessionRecord, SessionState};

/// The route the move is asked on. Fleet carries it to the Mac that holds the Job, so a Job held there asks it too.
pub const ROUTE: &str = "/sessions/move_to_mac";

/// The `end_reason` of a Job that went to the other Mac.
pub const ENDED_AS: &str = "moved_to_mac";

/// What the act reads for one Job, and where it goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    /// The words of the menu item and the palette's row: `Move to Air`, or `Bring here`.
    pub label: String,
    /// The Mac a Job of this Mac goes to. None for a Job held by the other Mac, which comes here.
    pub to: Option<String>,
    /// Why it is left off now, said in its tooltip.
    pub off: Option<String>,
}

/// The Mac a Job goes to from here: the first other Mac whose link is up, else the first asleep. Only one other
/// Mac is paired today, and a second would need a menu of its own.
fn other(macs: &[Mac]) -> Option<&Mac> {
    let others = || macs.iter().filter(|one| !one.this_mac);
    others().find(|one| one.link == Some(MacLink::Linked)).or_else(|| others().find(|one| matches!(one.link, Some(MacLink::Asleep { .. }))))
}

/// The id of the Mac a Job that went from here is on now: the other Mac.
pub fn went_to(macs: &[Mac]) -> Option<&str> {
    other(macs).map(|one| one.id.as_str())
}

/// The id of the Mac a Job moving to this one goes to: this Mac's own.
pub fn this_mac(macs: &[Mac]) -> Option<&str> {
    macs.iter().find(|one| one.this_mac).map(|one| one.id.as_str())
}

/// Whether the Job has gone to the other Mac and ended here, as its row last said.
pub fn moved(job: &SessionRecord) -> bool {
    job.state == SessionState::Ended && job.end_reason.as_deref() == Some(ENDED_AS)
}

/// The Mac a Job is moving to now, by id.
pub fn moving_to(job: &SessionRecord) -> Option<&str> {
    job.hosted.as_ref().and_then(|hosted| hosted.moving_to_mac.as_deref())
}

/// The act for `job`, or none where it cannot move: no other Mac is linked or asleep, the Job is not one Fleet
/// hosts, has ended, is paused or is moving already. A Job this Mac holds is left off, with why, while the Mac it
/// would go to sleeps. A Job the other Mac holds is asked of that Mac through Fleet's queue, so it stays on.
pub fn offer(job: &SessionRecord, macs: &[Mac]) -> Option<Offer> {
    if !crate::pairing::shows_chips(macs) || job.state == SessionState::Ended {
        return None;
    }
    let hosted = job.hosted.as_ref()?;
    if hosted.paused.is_some() || hosted.moving_to_mac.is_some() {
        return None;
    }
    if job.mac.is_some() {
        return Some(Offer { label: "Bring here".into(), to: None, off: None });
    }
    let to = other(macs)?;
    let off = match to.link {
        Some(MacLink::Asleep { since }) => Some(crate::pairing::asleep_said(since)),
        _ => None,
    };
    Some(Offer { label: format!("Move to {}", to.name), to: Some(to.id.clone()), off })
}

/// What is posted to `/sessions/move_to_mac`: `MoveJobToMac`, with the Mac to go to and none for a Job that comes here.
pub fn body(job: &str, to: Option<&str>) -> Result<String, ipc::Unencodable> {
    ipc::encode(&ipc::MoveJobToMac { session_id: ipc::SessionId::carried(job.to_string()), mac: to.map(str::to_string) })
}

/// The move asked of Fleet. It answers the row with the move marked and publishes it, so nothing is kept here; a
/// refusal is said in a toast in Fleet's words.
pub fn post(port: Option<u16>, body: Result<String, ipc::Unencodable>, cx: &mut App) {
    let Some(port) = port else {
        crate::toasts::tell("Fleet is not connected", cx);
        return;
    };
    let body = match body {
        Ok(body) => body,
        Err(why) => {
            crate::toasts::tell(format!("{why:?}"), cx);
            return;
        }
    };
    cx.spawn(async move |cx| {
        let answer = cx
            .background_executor()
            .spawn(async move {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                runtime.block_on(crate::transport::post(port, ROUTE, body))
            })
            .await;
        let said = match answer {
            Ok((status, _)) if (200..300).contains(&status) => return,
            Ok((status, bytes)) => crate::snooze::refusal_said(status, &bytes),
            Err(why) => why,
        };
        cx.update(|cx| crate::toasts::tell(said, cx));
    })
    .detach();
}

/// A walk's Fleet that refuses the next move, in the sentence Fleet's own refusal carries: another window asked
/// first. Set by a scene, spent by the press that meets it.
static REFUSE_NEXT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Make the mock Fleet refuse the next move, or forget that it was to.
pub fn refuse_next(on: bool) {
    REFUSE_NEXT.store(on, std::sync::atomic::Ordering::SeqCst);
}

/// Whether the mock Fleet refuses this move, and in what words: Fleet's `fleet.session_not_movable` for a Job that
/// is moving already, naming the Mac it goes to. Taken once.
pub fn mock_refusal(_job: &str) -> Option<String> {
    REFUSE_NEXT.swap(false, std::sync::atomic::Ordering::SeqCst).then(|| "This Job is already moving to Air.".to_string())
}
