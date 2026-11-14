//! The host's half of a sealed page: what the kit's script posts, the Fleet call each post makes, and Fleet's answer
//! handed back for the kit to draw. Pure, so it is read without a page. The messages and their channels are
//! `docs/practices/protocol/a-pages-sealed-serving.md`; the types are `ipc`'s (`PageToHost`, `HostToPage`).

use ipc::{CastPageVote, HostToPage, PageMessage, PagePress, PageToHost, PressPageAction, SessionId, VoteCast};

/// The route a press goes to (`press_page_action`).
pub const PRESS: &str = "/sessions/artifacts/press";
/// The route a vote goes to (`cast_page_vote`).
pub const VOTE: &str = "/sessions/artifacts/vote";

/// What the kit posted, read. `None` for anything else: a message whose `armada` is not 1, or of a kind it does not post.
pub fn read(posted: &str) -> Option<PageToHost> {
    ipc::decode::<PageMessage<PageToHost>>("page message", posted.as_bytes()).ok().map(|message| message.message)
}

/// The Fleet call a post makes for Job `job`: its route and its body.
pub fn call(job: &str, posted: &PageToHost) -> (&'static str, Result<String, ipc::Unencodable>) {
    let session_id = SessionId::carried(job.to_string());
    match posted.clone() {
        PageToHost::Press { artifact, revision, action, words } => (PRESS, ipc::encode(&PressPageAction { session_id, artifact, revision, action, words })),
        PageToHost::Vote { artifact, revision, vote, result } => (VOTE, ipc::encode(&CastPageVote { session_id, artifact, revision, vote, result })),
    }
}

/// Fleet's answer to a post, as the page draws it: the press or the cast it kept, or its refusal with its own code and
/// sentence. `answer` is the body Fleet answered, or its sentence and code where it refused.
pub fn reply(posted: &PageToHost, answer: Result<&[u8], (String, Option<String>)>) -> HostToPage {
    let read = match (posted, answer) {
        (PageToHost::Press { .. }, Ok(bytes)) => ipc::decode::<PagePress>("PagePress", bytes).map(|press| HostToPage::Pressed { press }).ok(),
        (PageToHost::Vote { .. }, Ok(bytes)) => ipc::decode::<VoteCast>("VoteCast", bytes).map(|cast| HostToPage::Voted { cast }).ok(),
        (_, Err((message, code))) => return refused(posted, code.unwrap_or_default(), message),
    };
    read.unwrap_or_else(|| refused(posted, String::new(), "Fleet's answer could not be read".into()))
}

fn refused(posted: &PageToHost, code: String, message: String) -> HostToPage {
    match posted {
        PageToHost::Press { action, .. } => HostToPage::Refused { action: Some(action.clone()), vote: None, code, message },
        PageToHost::Vote { vote, .. } => HostToPage::Refused { action: None, vote: Some(vote.clone()), code, message },
    }
}

/// The script that hands `reply` to the kit: `window.armadaHost(<message>)`, the message as JSON, which is a JavaScript
/// expression as written.
pub fn script(reply: &HostToPage) -> String {
    let message = ipc::encode(&PageMessage::new(reply.clone())).unwrap_or_else(|_| "null".into());
    format!("window.armadaHost && window.armadaHost({message}); undefined")
}

/// The Job a press started, where it started one.
pub fn started(reply: &HostToPage) -> Option<&SessionId> {
    match reply {
        HostToPage::Pressed { press: PagePress { did: ipc::PressDid::Started { job }, .. } } => Some(job),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{PageAct, PressDid, VoteForm, VoteMark, VoteResult};

    fn press() -> PageToHost {
        PageToHost::Press { artifact: "design".into(), revision: 2, action: "rebase-rate-limits".into(), words: None }
    }

    fn score() -> PageToHost {
        let marks = vec![VoteMark { take: "compact".into(), part: "age".into(), keep: true }, VoteMark { take: "full".into(), part: "author".into(), keep: false }];
        PageToHost::Vote { artifact: "design".into(), revision: 2, vote: "row-parts".into(), result: VoteResult::Score { marks, notes: Default::default() } }
    }

    #[test]
    fn a_kit_post_reads_as_its_message_and_anything_else_is_dropped() {
        let posted = read(r#"{"armada":1,"kind":"press","action":"discuss-dark-mode","words":"Why three days?","artifact":"design","revision":2}"#);
        assert_eq!(posted, Some(PageToHost::Press { artifact: "design".into(), revision: 2, action: "discuss-dark-mode".into(), words: Some("Why three days?".into()) }));
        let vote = read(r#"{"armada":1,"kind":"vote","vote":"mark","result":{"form":"pick","take":"fade"},"artifact":"design","revision":2}"#);
        assert!(matches!(vote, Some(PageToHost::Vote { result: VoteResult::Pick { ref take, .. }, .. }) if take == "fade"));
        assert_eq!(read(r#"{"armada":2,"kind":"press","action":"a","artifact":"design","revision":2}"#), None, "another armada");
        assert_eq!(read(r#"{"armada":1,"kind":"pressed","press":{}}"#), None, "the host's own kind");
        assert_eq!(read("not json"), None);
    }

    #[test]
    fn a_press_and_a_vote_make_fleets_calls_with_the_jobs_id() {
        let posted = PageToHost::Press { artifact: "design".into(), revision: 2, action: "discuss-dark-mode".into(), words: Some("Now".into()) };
        let (route, body) = call("mock-06", &posted);
        assert_eq!(route, PRESS);
        assert_eq!(
            ipc::decode::<PressPageAction>("PressPageAction", body.unwrap().as_bytes()).unwrap(),
            PressPageAction { session_id: SessionId::carried("mock-06"), artifact: "design".into(), revision: 2, action: "discuss-dark-mode".into(), words: Some("Now".into()) }
        );
        let (route, body) = call("mock-06", &score());
        assert_eq!(route, VOTE);
        let cast = ipc::decode::<CastPageVote>("CastPageVote", body.unwrap().as_bytes()).unwrap();
        assert_eq!((cast.session_id.as_str(), cast.vote.as_str(), cast.result.form()), ("mock-06", "row-parts", VoteForm::Score));
    }

    fn press_with(action: &str) -> PageToHost {
        PageToHost::Press { artifact: "design".into(), revision: 2, action: action.into(), words: None }
    }

    #[test]
    fn fleets_answer_goes_back_as_pressed_or_voted() {
        let kept = br#"{"revision":2,"action":"rebase-rate-limits","item":"api/rate-limits","act":"start","at":"2026-11-14T09:00:00Z","did":{"kind":"started","job":"mock-12"}}"#;
        let answer = reply(&press(), Ok(kept));
        assert!(matches!(&answer, HostToPage::Pressed { press } if press.act == PageAct::Start && press.did == PressDid::Started { job: SessionId::carried("mock-12") }));
        assert_eq!(started(&answer).map(SessionId::as_str), Some("mock-12"));
        let cast = br#"{"revision":2,"vote":"row-parts","form":"score","result":{"form":"score","marks":[{"take":"compact","part":"age","keep":true}]},"at":"2026-11-14T09:00:00Z"}"#;
        assert!(matches!(reply(&score(), Ok(cast)), HostToPage::Voted { cast } if cast.vote == "row-parts"));
    }

    #[test]
    fn a_refusal_goes_back_on_its_own_row_or_vote_with_fleets_code_and_sentence() {
        let answer = reply(&press_with("drop-checkout"), Err(("the Job no longer waits on `ask-checkout`".into(), Some("fleet.page_ask_not_open".into()))));
        assert_eq!(
            answer,
            HostToPage::Refused { action: Some("drop-checkout".into()), vote: None, code: "fleet.page_ask_not_open".into(), message: "the Job no longer waits on `ask-checkout`".into() }
        );
        assert_eq!(started(&answer), None);
        let answer = reply(&score(), Err(("`Stale branch design` is archived. Restore it to vote on it".into(), Some("fleet.page_archived".into()))));
        assert!(matches!(answer, HostToPage::Refused { action: None, vote: Some(ref vote), .. } if vote == "row-parts"));
        assert!(matches!(reply(&press(), Ok(b"{}")), HostToPage::Refused { ref message, .. } if message == "Fleet's answer could not be read"), "an answer that does not read");
    }

    #[test]
    fn the_reply_reaches_the_kit_as_its_own_message() {
        let script = script(&HostToPage::Refused { action: Some("drop-checkout".into()), vote: None, code: "fleet.page_ask_not_open".into(), message: "no".into() });
        assert_eq!(script, r#"window.armadaHost && window.armadaHost({"armada":1,"kind":"refused","action":"drop-checkout","code":"fleet.page_ask_not_open","message":"no"}); undefined"#);
    }
}
