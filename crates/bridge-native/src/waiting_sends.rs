//! Sends that wait for another of his Macs: an act on a Job that Mac holds, made while it could not be reached.
//! Fleet keeps each one and sends it when that Mac speaks again (`docs/concepts/pocket.md`, *Acting across Macs*).
//!
//! The window holds Fleet's list whole (`crate::fleet::Held::queued`); a Job's thread draws its own as the Waits
//! for card (`crate::job_view`), whose rows say what each send is in [`said_of`].

use ipc::{JobAct, Queued, QueuedList, SketchHow, WaitingItem};

/// Where a send is taken back before it goes.
pub const TAKE_BACK: &str = "/queued/take_back";

/// What a row of the card says: the act's kind, and the words it carried where it carried any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    pub kind: String,
    pub detail: Option<String>,
}

fn said(kind: &str, detail: Option<String>) -> Said {
    Said { kind: kind.to_string(), detail: detail.filter(|words| !words.is_empty()) }
}

/// The first line of some words, trimmed.
fn first_line(text: &str) -> String {
    text.trim().lines().next().unwrap_or_default().trim().to_string()
}

/// What a send is, in bare words: "Message" and its first line, "Answer" and what was picked, "Plan approved",
/// "Plan changes", "Stop", "Merge #N". `waiting` is the Job's Needs you, which names a picked option.
pub fn said_of(act: &JobAct, waiting: &[WaitingItem]) -> Said {
    let option = |item_id: &str, choice: u32| waiting.iter().find(|item| item.id == item_id).and_then(|item| item.options.get(choice as usize)).map(|option| option.label.clone());
    match act {
        JobAct::Message(body) => said("Message", Some(first_line(&body.message.text))),
        JobAct::JobMessage(message) => said(&format!("From {}", message.title), Some(first_line(&message.text))),
        JobAct::AskAnswer(answer) => {
            let chosen: Vec<&str> = answer.answers.iter().flat_map(|one| one.chosen.iter().map(String::as_str)).collect();
            let words = if chosen.is_empty() { answer.note.as_deref().map(first_line) } else { Some(chosen.join(", ")) };
            said("Answer", words)
        }
        JobAct::WaitingAnswer(answer) => {
            let picked = answer.choice.and_then(|choice| option(&answer.item_id, choice));
            said("Answer", picked.or_else(|| answer.text.as_deref().or(answer.fixed.as_deref()).map(first_line)))
        }
        JobAct::WaitingAnswerParts(answer) => {
            let parts: Vec<String> = answer.parts_picked.iter().map(|one| one.part.clone()).collect();
            said("Answer", Some(parts.join(", ")))
        }
        JobAct::SketchAnswer(answer) => {
            let title = answer.sketch.strip_prefix("sketch:").unwrap_or(&answer.sketch);
            let how = if answer.how == SketchHow::Approved { "approved" } else { "sent back" };
            said("Answer", Some(format!("{title}, {how}")))
        }
        JobAct::DocumentAnswer(answer) => {
            let page = answer.document.strip_prefix("document:").unwrap_or(&answer.document);
            let name = page.rsplit('/').next().unwrap_or(page);
            let how = if answer.how == ipc::DocumentHow::Approved { "approved" } else { "sent back" };
            said("Answer", Some(format!("{name}, {how}")))
        }
        JobAct::PlanApprove(_) => said("Plan approved", None),
        JobAct::PlanChanges(changes) if changes.approve => said("Plan approved", Some("with changes".into())),
        JobAct::PlanChanges(_) => said("Plan changes", None),
        JobAct::Stop(_) => said("Stop", None),
        JobAct::Merge { number, .. } => said(&format!("Merge #{number}"), None),
        JobAct::Start(_) => said("Start", None),
        JobAct::Dispatch(_) => said("Start", None),
        // Arrive and MoveSettled are the two Fleets' halves of the move he pressed.
        JobAct::Move { .. } | JobAct::Arrive(_) | JobAct::MoveSettled(_) => said("Move", None),
    }
}

/// The sends waiting on one Job, in Fleet's order, which is oldest first.
pub fn of_job<'a>(list: &'a QueuedList, job: &str) -> Vec<&'a Queued> {
    list.queued.iter().filter(|one| one.act.session_id().is_some_and(|id| id.as_str() == job)).collect()
}

/// The body that takes back the send kept under `id`.
pub fn take_back_body(id: &str) -> Result<String, ipc::Unencodable> {
    ipc::encode(&ipc::TakeBack { id: id.to_string() })
}

/// The act a route's body is, for the mock Fleet to keep as a real one keeps a send to a Mac asleep. `None` for a
/// route that does not cross, or a body that is not its route's.
pub fn act_of(path: &str, body: &str) -> Option<JobAct> {
    let bytes = body.as_bytes();
    Some(match path {
        "/sessions/message" => JobAct::Message(ipc::decode("SendSessionMessageBody", bytes).ok()?),
        "/sessions/ask/answer" => JobAct::AskAnswer(ipc::decode("AnswerSessionAsk", bytes).ok()?),
        "/sessions/waiting/answer" => JobAct::WaitingAnswer(ipc::decode("AnswerWaiting", bytes).ok()?),
        "/sessions/waiting/answer_parts" => JobAct::WaitingAnswerParts(ipc::decode("AnswerWaitingParts", bytes).ok()?),
        "/sessions/plan/approve" => JobAct::PlanApprove(ipc::decode("ApprovePlan", bytes).ok()?),
        "/sessions/plan/changes" => JobAct::PlanChanges(ipc::decode("SendPlanChanges", bytes).ok()?),
        "/sessions/sketch/answer" => JobAct::SketchAnswer(ipc::decode("AnswerSketch", bytes).ok()?),
        "/sessions/document/answer" => JobAct::DocumentAnswer(ipc::decode("AnswerDocument", bytes).ok()?),
        "/sessions/stop" => JobAct::Stop(ipc::decode("StopSession", bytes).ok()?),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(json: &str) -> JobAct {
        ipc::decode::<JobAct>("JobAct", json.as_bytes()).expect("the act decodes")
    }

    fn question() -> WaitingItem {
        let raw = r#"{"id":"agent-2","text":"Land on a branch or a pull request?","since":"2026-11-09T14:04:00Z","source":"agent","options":[{"label":"Pull request"},{"label":"Branch"}]}"#;
        ipc::decode::<WaitingItem>("WaitingItem", raw.as_bytes()).expect("the item decodes")
    }

    #[test]
    fn each_send_says_what_it_is_in_bare_words() {
        let words = |json: &str| {
            let said = said_of(&act(json), &[question()]);
            match said.detail {
                Some(detail) => format!("{}: {detail}", said.kind),
                None => said.kind,
            }
        };
        assert_eq!(words(r#"{"kind":"message","body":{"session_id":"j","text":"Check the tokens first\nthen the hue"}}"#), "Message: Check the tokens first");
        assert_eq!(words(r#"{"kind":"waiting_answer","body":{"session_id":"j","item_id":"agent-2","choice":1}}"#), "Answer: Branch");
        assert_eq!(words(r#"{"kind":"waiting_answer","body":{"session_id":"j","item_id":"gone","text":"Use the branch"}}"#), "Answer: Use the branch");
        assert_eq!(words(r#"{"kind":"plan_approve","body":{"session_id":"j","version":2}}"#), "Plan approved");
        assert_eq!(words(r#"{"kind":"plan_changes","body":{"session_id":"j","version":2,"marks":[]}}"#), "Plan changes");
        assert_eq!(words(r#"{"kind":"stop","body":{"session_id":"j"}}"#), "Stop");
        assert_eq!(words(r#"{"kind":"merge","body":{"session_id":"j","repository":"armada","number":3548}}"#), "Merge #3548");
        assert_eq!(words(r#"{"kind":"sketch_answer","body":{"session_id":"j","sketch":"sketch:Tail layout","drawn":"d","how":"approved"}}"#), "Answer: Tail layout, approved");
        assert_eq!(words(r#"{"kind":"message","body":{"session_id":"j","text":"  "}}"#), "Message", "a message of pictures alone says its kind");
    }

    #[test]
    fn a_jobs_sends_are_its_own_in_fleets_order() {
        let one = |id: &str, job: &str| Queued { id: id.into(), mac: "air".into(), act: act(&format!(r#"{{"kind":"stop","body":{{"session_id":"{job}"}}}}"#)), since: 1, refused: None };
        let list = QueuedList { queued: vec![one("a", "j1"), one("b", "j2"), one("c", "j1")] };
        assert_eq!(of_job(&list, "j1").iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["a", "c"]);
    }

    #[test]
    fn a_crossing_route_body_is_kept_as_its_act() {
        assert_eq!(act_of("/sessions/stop", r#"{"session_id":"j"}"#), Some(act(r#"{"kind":"stop","body":{"session_id":"j"}}"#)));
        assert_eq!(act_of("/sessions/pause", r#"{"session_id":"j"}"#), None);
    }
}
