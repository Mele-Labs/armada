//! Moving a Job to his other Mac: the act's offer, the words of a move under way and the step it is at
//! (`docs/practices/protocol/his-other-macs-fleets-half.md`, *Moving a Job between Macs*).
//!
//! One press sends it, with no sheet, and Fleet's move cannot be cancelled. A Job this Mac holds goes to the other Mac by
//! its id; a Job held from another Mac is asked of that Mac, to come here, and the body names no Mac.

use gpui::App;
use ipc::{Mac, MacLink, SessionRecord, SessionState};

/// Where the press is sent.
pub const ROUTE: &str = "/sessions/move_to_mac";

/// The key the act wears in a menu.
pub const KEY: &str = "\u{2325}\u{2318}M";

/// What the menu draws for the act: its words, why it is off where it is, and the Mac the body names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub label: String,
    pub off: Option<String>,
    /// The Mac to send the Job to by id. Absent for a Job held from another Mac, which comes here.
    pub to: Option<String>,
}

/// What a move says: the Mac it goes to, in the words a row and a card share.
pub fn moving_said(mac: &str) -> String {
    format!("Moving to {mac}")
}

fn this_mac(macs: &[Mac]) -> Option<&Mac> {
    macs.iter().find(|one| one.this_mac)
}

/// The name of the Mac with this id, where the list holds it. A Job held from another Mac that is moving here names
/// the Mac it is going to by the id that Mac knows this one by, so a name the list lacks is this Mac's.
pub fn name_of(macs: &[Mac], id: &str, held_away: bool) -> String {
    match macs.iter().find(|one| one.id == id) {
        Some(one) => one.name.clone(),
        None if held_away => this_mac(macs).map_or_else(|| "this Mac".to_string(), |one| one.name.clone()),
        None => "another Mac".to_string(),
    }
}

/// The act for `job`, or `None` where it is not offered: the Job has ended, is not hosted, or no other Mac is paired.
/// `paused` is the Job as the head shows it, which a press has moved ahead of Fleet's record.
pub fn offer(job: &SessionRecord, paused: bool, macs: &[Mac]) -> Option<Offer> {
    let hosted = job.hosted.as_ref()?;
    if job.state == SessionState::Ended || !crate::pairing::shows_chips(macs) {
        return None;
    }
    let held_away = job.mac.is_some();
    let (name, to) = if held_away {
        (this_mac(macs).map_or_else(|| "this Mac".to_string(), |one| one.name.clone()), None)
    } else {
        // An awake Mac before one asleep, where more than one other Mac is paired.
        let other = macs
            .iter()
            .filter(|one| !one.this_mac && matches!(one.link, Some(MacLink::Linked | MacLink::Asleep { .. })))
            .min_by_key(|one| matches!(one.link, Some(MacLink::Asleep { .. })))?;
        (other.name.clone(), Some(other.id.clone()))
    };
    let off = match &hosted.moving_to_mac {
        Some(target) => Some(moving_said(&name_of(macs, target, held_away))),
        None if paused => Some("Paused".to_string()),
        None => None,
    };
    Some(Offer { label: format!("Move to {name}"), off, to })
}

/// The body of the press.
pub fn body(job: &str, to: Option<String>) -> Result<String, ipc::Unencodable> {
    ipc::encode(&ipc::MoveJobToMac { session_id: ipc::SessionId::carried(job.to_string()), mac: to })
}

/// The Mac a Job on its way is going to, by id, where its record says so.
pub fn target_of(job: &SessionRecord) -> Option<&str> {
    job.hosted.as_ref().and_then(|hosted| hosted.moving_to_mac.as_deref())
}

/// One of a move's three steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    TurnEnds,
    BranchPushed,
    Resumed,
}

/// The steps in the order a move takes them.
pub const STEPS: [Step; 3] = [Step::TurnEnds, Step::BranchPushed, Step::Resumed];

impl Step {
    /// What the chip says. The last names the Mac the Job goes to.
    pub fn said(self, mac: &str) -> String {
        match self {
            Step::TurnEnds => "Turn ends".to_string(),
            Step::BranchPushed => "Branch pushed".to_string(),
            Step::Resumed => format!("Resumed on {mac}"),
        }
    }
}

/// The step a move is at. Fleet sends no progress, so this is read off the Job: the move waits for the turn, so while
/// the Job is working it is at Turn ends, and once it has stopped working the branch is being pushed. Resumed is the
/// other Mac's, and the row ends here when it is reached, so no record is ever at it.
pub fn step_of(working: bool) -> Step {
    if working { Step::TurnEnds } else { Step::BranchPushed }
}

/// A press from a row's menu or key, sent off the main thread. A refusal is said in a toast with Fleet's own sentence.
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

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::MacColour;

    fn mac(id: &str, name: &str, this: bool, link: Option<MacLink>) -> Mac {
        Mac { id: id.into(), name: name.into(), colour: MacColour::Teal, this_mac: this, link, ..Default::default() }
    }

    fn pair() -> Vec<Mac> {
        vec![mac("air", "Air", true, None), mac("studio", "Studio", false, Some(MacLink::Linked))]
    }

    fn hosted_job() -> SessionRecord {
        let mut jobs = crate::mock::jobs("jobs-screens").expect("the fixture decodes");
        let at = jobs.iter().position(|job| job.hosted.is_some() && job.state == SessionState::Live).expect("a live hosted Job");
        jobs.remove(at)
    }

    #[test]
    fn the_act_names_the_mac_it_sends_the_job_to() {
        let offer = offer(&hosted_job(), false, &pair()).expect("offered");
        assert_eq!(offer, Offer { label: "Move to Studio".into(), off: None, to: Some("studio".into()) });
    }

    #[test]
    fn a_job_held_on_the_other_mac_comes_here_and_names_no_mac() {
        let mut job = hosted_job();
        job.mac = Some("studio".into());
        let offer = offer(&job, false, &pair()).expect("offered");
        assert_eq!(offer.label, "Move to Air");
        assert_eq!(offer.to, None);
        assert_eq!(body("j", offer.to).expect("encodes"), r#"{"session_id":"j"}"#);
        assert_eq!(body("j", Some("studio".into())).expect("encodes"), r#"{"session_id":"j","mac":"studio"}"#);
    }

    #[test]
    fn a_paused_job_is_off_and_says_paused() {
        let offer = offer(&hosted_job(), true, &pair()).expect("offered");
        assert_eq!(offer.off.as_deref(), Some("Paused"));
    }

    #[test]
    fn a_job_already_moving_is_off_and_names_where() {
        let mut job = hosted_job();
        job.hosted.as_mut().expect("hosted").moving_to_mac = Some("studio".into());
        assert_eq!(offer(&job, false, &pair()).and_then(|offer| offer.off).as_deref(), Some("Moving to Studio"));
        assert_eq!(offer(&job, true, &pair()).and_then(|offer| offer.off).as_deref(), Some("Moving to Studio"), "moving is said before paused");
        assert_eq!(target_of(&job), Some("studio"));
    }

    #[test]
    fn nothing_is_offered_without_a_paired_mac_a_hosted_job_or_a_live_one() {
        assert_eq!(offer(&hosted_job(), false, &[mac("air", "Air", true, None)]), None, "one Mac");
        let waiting = mac("studio", "Studio", false, Some(MacLink::ClaimWaiting));
        assert_eq!(offer(&hosted_job(), false, &[mac("air", "Air", true, None), waiting]), None, "a claim waiting is not paired");
        let mut plain = hosted_job();
        plain.hosted = None;
        assert_eq!(offer(&plain, false, &pair()), None, "not hosted");
        let mut ended = hosted_job();
        ended.state = SessionState::Ended;
        assert_eq!(offer(&ended, false, &pair()), None, "ended");
    }

    #[test]
    fn an_awake_mac_is_chosen_before_one_asleep() {
        let macs = vec![mac("air", "Air", true, None), mac("mini", "Mini", false, Some(MacLink::Asleep { since: 5 })), mac("studio", "Studio", false, Some(MacLink::Linked))];
        assert_eq!(offer(&hosted_job(), false, &macs).and_then(|offer| offer.to).as_deref(), Some("studio"));
    }

    #[test]
    fn the_step_is_read_off_whether_the_job_still_works() {
        assert_eq!(step_of(true), Step::TurnEnds);
        assert_eq!(step_of(false), Step::BranchPushed);
        let said: Vec<String> = STEPS.iter().map(|step| step.said("Studio")).collect();
        assert_eq!(said, ["Turn ends", "Branch pushed", "Resumed on Studio"]);
    }
}
