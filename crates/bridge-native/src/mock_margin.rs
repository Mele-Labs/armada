//! The mock Fleet's margin threads: the quiet hours page's, and his comments, Send held and acts on them, kept as
//! Fleet keeps them (`crates/fleet/src/session_host/margin.rs`). The agent answers nothing here.

use ipc::{Instant, MarginAct, MarginAnchor, MarginMessage, MarginOn, MarginState, MarginThread, MarginThreads, MarginVoice, SessionId, SuggestionDecided, SuggestionHow};

/// The threads the mock Fleet holds on page `page` of Job `job`: the quiet hours page's, and none on any other.
pub fn threads(job: &str, page: &str) -> MarginThreads {
    let none = || MarginThreads { session_id: SessionId::carried(job), document: page.to_string(), threads: Vec::new() };
    if page != "quiet" {
        return none();
    }
    match ipc::decode::<MarginThreads>("MarginThreads", include_bytes!("../fixtures/planning-quiet-margin.json")) {
        Ok(mut read) => {
            read.session_id = SessionId::carried(job);
            read
        }
        Err(_) => none(),
    }
}

fn now() -> Instant {
    Instant::carried(chrono::Local::now().to_rfc3339())
}

fn open<'a>(kept: &'a mut MarginThreads, named: &str) -> Result<&'a mut MarginThread, String> {
    let thread = kept.threads.iter_mut().find(|one| one.id == named).ok_or_else(|| format!("this job has no margin thread `{named}`"))?;
    if thread.state == MarginState::Settled {
        return Err(format!("thread `{named}` is settled"));
    }
    Ok(thread)
}

/// `comment_margin`: a new thread on a passage, or his reply on one, sent or held.
pub fn comment(kept: &mut MarginThreads, on: &MarginOn, text: &str, hold: bool) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("a comment says something".into());
    }
    let thread = match on {
        MarginOn::Passage { anchor } => {
            let quote = anchor.quote.trim();
            if quote.is_empty() {
                return Err("a comment is on words of the page: the quote is empty".into());
            }
            let id = format!("m{}", kept.threads.len() + 1);
            let anchor = MarginAnchor { quote: quote.to_string(), block: anchor.block.clone(), section: anchor.section.clone() };
            kept.threads.push(MarginThread { id, document: kept.document.clone(), anchor, messages: Vec::new(), state: MarginState::Held, gone: false, suggestion: None, outcome: None });
            kept.threads.last_mut().ok_or_else(|| "the thread was not kept".to_string())?
        }
        MarginOn::Thread { thread } => open(kept, thread)?,
    };
    thread.messages.push(MarginMessage { by: MarginVoice::You, text: text.to_string(), at: now(), held: hold });
    // A suggested edit still waits on his Accept or Reject, whatever he says beside it.
    if thread.state != MarginState::Suggested {
        thread.state = if hold { MarginState::Held } else { MarginState::Open };
    }
    Ok(())
}

/// `send_held_margin`: every comment he holds on the page, sent together.
pub fn send_held(kept: &mut MarginThreads) -> Result<(), String> {
    let mut any = false;
    for thread in &mut kept.threads {
        if !thread.messages.iter().any(|one| one.held) {
            continue;
        }
        any = true;
        thread.messages.iter_mut().for_each(|one| one.held = false);
        if thread.state == MarginState::Held {
            thread.state = MarginState::Open;
        }
    }
    if any {
        Ok(())
    } else {
        Err("no comment is held on this page".into())
    }
}

/// `act_on_margin_thread` on a page at revision `newest`. Accept answers the words struck and the words added, for the
/// page's next revision.
pub fn act(kept: &mut MarginThreads, named: &str, act: &MarginAct, newest: u32) -> Result<Option<(String, String)>, String> {
    let thread = open(kept, named)?;
    let undecided = |thread: &MarginThread| thread.suggestion.as_ref().filter(|one| one.decided.is_none()).cloned().ok_or_else(|| format!("thread `{named}` holds no suggested edit to decide"));
    match act {
        MarginAct::MakeChange => {
            thread.suggestion = None;
            thread.state = MarginState::Open;
            Ok(None)
        }
        MarginAct::Accept => {
            let mut suggestion = undecided(thread)?;
            if thread.anchor.quote.contains(&suggestion.struck) {
                thread.anchor.quote = thread.anchor.quote.replacen(&suggestion.struck, &suggestion.added, 1);
            } else if suggestion.struck.contains(&thread.anchor.quote) {
                thread.anchor.quote = suggestion.added.clone();
            }
            suggestion.decided = Some(SuggestionDecided { how: SuggestionHow::Accepted, at: now(), revision: Some(newest + 1) });
            let edit = (suggestion.struck.clone(), suggestion.added.clone());
            thread.suggestion = Some(suggestion);
            thread.state = MarginState::Answered;
            Ok(Some(edit))
        }
        MarginAct::Reject => {
            let mut suggestion = undecided(thread)?;
            suggestion.decided = Some(SuggestionDecided { how: SuggestionHow::Rejected, at: now(), revision: None });
            thread.suggestion = Some(suggestion);
            thread.state = MarginState::Answered;
            Ok(None)
        }
        MarginAct::Settle { outcome } => {
            thread.outcome = outcome.as_deref().map(str::trim).filter(|said| !said.is_empty()).map(str::to_string);
            thread.state = MarginState::Settled;
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_of(kept: &MarginThreads, id: &str) -> MarginState {
        kept.threads.iter().find(|one| one.id == id).map(|one| one.state).expect("the thread is kept")
    }

    #[test]
    fn the_quiet_page_holds_its_threads_and_another_page_none() {
        let kept = threads("plan-job", "quiet");
        assert_eq!(kept.threads.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["m1", "m2", "m3", "m5", "m4"]);
        assert!(kept.threads.iter().any(|one| one.gone), "one passage is gone from the page");
        assert!(threads("plan-job", "design").threads.is_empty());
    }

    #[test]
    fn a_held_comment_waits_until_send_held_and_then_goes_with_the_others() {
        let mut kept = threads("plan-job", "quiet");
        let anchor = MarginAnchor { quote: "grouped by Job".into(), block: Some("b16".into()), section: Some("How it works".into()) };
        comment(&mut kept, &MarginOn::Passage { anchor }, "Could it group by project?", true).expect("a comment on words of the page");
        assert_eq!(state_of(&kept, "m6"), MarginState::Held);
        send_held(&mut kept).expect("two comments are held");
        assert_eq!((state_of(&kept, "m6"), state_of(&kept, "m3")), (MarginState::Open, MarginState::Open));
        assert!(kept.threads.iter().all(|one| one.messages.iter().all(|said| !said.held)));
        assert!(send_held(&mut kept).is_err(), "nothing is held now");
        assert!(comment(&mut kept, &MarginOn::Thread { thread: "m1".into() }, "  ", false).is_err(), "an empty comment says nothing");
    }

    #[test]
    fn accept_answers_the_edit_and_the_quote_reads_as_edited() {
        let mut kept = threads("plan-job", "quiet");
        let edit = act(&mut kept, "m2", &MarginAct::Accept, 2).expect("m2 holds an undecided edit").expect("Accept answers the edit");
        assert_eq!(edit.0, "A failed check still interrupts.");
        let thread = kept.threads.iter().find(|one| one.id == "m2").expect("m2");
        assert_eq!(thread.anchor.quote, edit.1);
        assert_eq!(thread.suggestion.as_ref().and_then(|one| one.decided.as_ref()).and_then(|one| one.revision), Some(3));
        assert!(act(&mut kept, "m2", &MarginAct::Reject, 3).is_err(), "a decided edit is not decided again");
        act(&mut kept, "m1", &MarginAct::Settle { outcome: Some(" Quiet hours end at your work start ".into()) }, 3).expect("m1 is open");
        assert_eq!(state_of(&kept, "m1"), MarginState::Settled);
        assert!(act(&mut kept, "m1", &MarginAct::MakeChange, 3).is_err(), "a settled thread takes nothing more");
    }

    /// The rounds come from JSON as the live app reads them: each card just after the row it replaces.
    #[test]
    fn the_planning_threads_read_their_rounds_from_json_each_after_the_row_it_replaces() {
        for (scenario, rounds) in [("planning-margin", 3), ("planning-today", 2)] {
            let read = crate::mock::threads(scenario).expect("the threads decode");
            let thread = read.get("plan-job").expect("the planning Job's thread");
            let mut seen = 0;
            for (at, item) in thread.items.iter().enumerate() {
                let ipc::ThreadItem::Margin(card) = item else { continue };
                seen += 1;
                let before = at.checked_sub(1).and_then(|one| thread.items.get(one));
                assert!(matches!(before, Some(ipc::ThreadItem::Row { row }) if Some(row.id()) == card.replaces.as_deref()), "{} stands after {:?}", card.id, card.replaces);
                assert_eq!(card.title, "Quiet hours for notices");
            }
            assert_eq!(seen, rounds, "{scenario}");
        }
    }
}
