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

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::MacColour;

    fn studio() -> Mac {
        Mac { id: "studio".into(), name: "Studio".into(), colour: MacColour::Teal, this_mac: true, ..Default::default() }
    }

    fn air(link: MacLink) -> Mac {
        Mac { id: "air".into(), name: "Air".into(), colour: MacColour::Orange, link: Some(link), ..Default::default() }
    }

    /// A live Job Fleet hosts, with no pause and no move, held by this Mac.
    fn job() -> SessionRecord {
        let job = crate::mock::jobs("jobs-screens").expect("the fixture decodes").into_iter().find(|job| job.id.as_str() == "mock-02").expect("a hosted Job");
        assert!(job.hosted.is_some() && job.state == SessionState::Live && job.mac.is_none());
        job
    }

    fn label(offer: Option<Offer>) -> Option<(String, Option<String>, Option<String>)> {
        offer.map(|one| (one.label, one.to, one.off))
    }

    #[test]
    fn a_job_this_mac_holds_is_offered_a_move_to_the_linked_mac() {
        let macs = [studio(), air(MacLink::Linked)];
        assert_eq!(label(offer(&job(), &macs)), Some(("Move to Air".into(), Some("air".into()), None)));
    }

    #[test]
    fn no_move_is_offered_while_no_other_mac_is_linked() {
        assert_eq!(offer(&job(), &[studio()]), None, "one Mac");
        assert_eq!(offer(&job(), &[studio(), air(MacLink::ClaimWaiting)]), None, "a claim waiting");
        assert_eq!(offer(&job(), &[studio(), air(MacLink::ConfirmWaiting)]), None, "a confirm waiting");
        assert_eq!(offer(&job(), &[]), None, "no Macs listed");
    }

    #[test]
    fn a_job_that_has_ended_is_paused_or_moving_is_not_offered_a_move() {
        let macs = [studio(), air(MacLink::Linked)];
        let mut ended = job();
        ended.state = SessionState::Ended;
        assert_eq!(offer(&ended, &macs), None, "ended");
        let mut paused = job();
        let held = crate::mock::jobs("jobs-screens").expect("the fixture decodes").into_iter().find(|job| job.id.as_str() == "mock-05").expect("a paused Job");
        paused.hosted.as_mut().expect("hosted").paused = held.hosted.expect("hosted").paused;
        assert!(paused.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some()));
        assert_eq!(offer(&paused, &macs), None, "paused");
        let mut moving = job();
        moving.hosted.as_mut().expect("hosted").moving_to_mac = Some("air".into());
        assert_eq!(offer(&moving, &macs), None, "moving");
        let mut unhosted = job();
        unhosted.hosted = None;
        assert_eq!(offer(&unhosted, &macs), None, "a Job Fleet does not host");
    }

    #[test]
    fn a_job_the_other_mac_holds_is_offered_bring_here_with_no_mac_to_name() {
        let mut held = job();
        held.mac = Some("air".into());
        for link in [MacLink::Linked, MacLink::Asleep { since: 5 }] {
            assert_eq!(label(offer(&held, &[studio(), air(link)])), Some(("Bring here".into(), None, None)), "it stays on while Air sleeps, as Fleet queues it");
        }
        let macs = [studio(), air(MacLink::Linked)];
        held.hosted.as_mut().expect("hosted").moving_to_mac = Some("studio".into());
        assert_eq!(offer(&held, &macs), None, "one already coming here is not offered again");
        held.hosted.as_mut().expect("hosted").moving_to_mac = None;
        held.state = SessionState::Ended;
        assert_eq!(offer(&held, &macs), None, "an ended Job is not brought");
    }

    #[test]
    fn an_asleep_mac_lists_the_move_off_with_when_it_fell_asleep() {
        let shown = offer(&job(), &[studio(), air(MacLink::Asleep { since: 99 })]).expect("the act is listed");
        assert_eq!((shown.label.as_str(), shown.to.as_deref()), ("Move to Air", Some("air")));
        assert_eq!(shown.off, Some(crate::pairing::asleep_said(99)));
        assert!(shown.off.is_some_and(|why| why.starts_with("Asleep since ")));
    }

    #[test]
    fn a_linked_mac_is_preferred_to_an_asleep_one() {
        let sleeper = Mac { id: "mini".into(), name: "Mini".into(), link: Some(MacLink::Asleep { since: 1 }), ..Default::default() };
        let shown = offer(&job(), &[studio(), sleeper, air(MacLink::Linked)]).expect("the act is listed");
        assert_eq!((shown.label.as_str(), shown.off), ("Move to Air", None));
    }

    #[test]
    fn the_mac_a_job_is_moving_to_is_read_from_its_hosted_record() {
        let mut moving = job();
        assert_eq!(moving_to(&moving), None);
        moving.hosted.as_mut().expect("hosted").moving_to_mac = Some("air".into());
        assert_eq!(moving_to(&moving), Some("air"));
        moving.hosted = None;
        assert_eq!(moving_to(&moving), None, "a Job with no hosted record is moving nowhere");
    }

    #[test]
    fn a_job_ended_as_moved_to_mac_reads_as_moved_to_the_mac_it_went_to() {
        let macs = [studio(), air(MacLink::Linked)];
        let mut gone = job();
        assert!(!moved(&gone), "a live Job has not moved");
        gone.state = SessionState::Ended;
        gone.end_reason = Some("merged".into());
        assert!(!moved(&gone), "ended for another reason");
        gone.end_reason = Some(ENDED_AS.into());
        assert!(moved(&gone));
        gone.state = SessionState::Live;
        assert!(!moved(&gone), "the reason alone is not an end");
        let went = went_to(&macs);
        assert_eq!(went, Some("air"));
        assert_eq!(crate::pairing::chip_of(&macs, went).map(|chip| chip.name), Some("Air".into()), "the row names it Moved to Air");
        assert_eq!(this_mac(&macs), Some("studio"));
    }

    #[test]
    fn the_mock_refuses_the_next_move_once_in_fleets_words() {
        assert_eq!(mock_refusal("mock-02"), None, "nothing is armed");
        refuse_next(true);
        assert_eq!(mock_refusal("mock-02").as_deref(), Some("This Job is already moving to Air."));
        assert_eq!(mock_refusal("mock-02"), None, "taken once");
        refuse_next(true);
        refuse_next(false);
        assert_eq!(mock_refusal("mock-02"), None, "forgotten before it was met");
    }
}
