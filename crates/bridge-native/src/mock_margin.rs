//! The mock Fleet's margin threads: the quiet hours page's, and his comments, Send held and acts on them, kept as
//! Fleet keeps them (`crates/fleet/src/session_host/margin.rs`). The agent answers nothing here.

use ipc::{Instant, JobThread, MarginAct, MarginRoundKind, ThreadItem, ThreadMargin, ThreadMarginNote, MarginAnchor, MarginMessage, MarginOn, MarginState, MarginThread, MarginThreads, MarginVoice, SessionId, SuggestionDecided, SuggestionHow};

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

/// The rounds of talk on the quiet hours page, as Fleet folds them into the Job's thread: each card just after the
/// message that sent it. Built here rather than read from JSON, as a margin card's `kind` field and the item's tag share
/// one key.
pub fn rounds(scenario: &str, thread: &mut JobThread) {
    let note = |section: &str, passage: &str, words: &str| ThreadMarginNote { thread: None, section: Some(section.into()), passage: Some(passage.into()), note: words.into() };
    let cards: Vec<(&str, MarginRoundKind, bool, Vec<&str>, &str, Vec<ThreadMarginNote>)> = match scenario {
        "planning-margin" => vec![
            ("r-1", MarginRoundKind::Settled, false, vec!["m4"], "Settled: Weekends are quiet all day", Vec::new()),
            ("r-2", MarginRoundKind::Comments, true, vec!["m1", "m5"], "Your comments are answered in the margin, and a passage you commented on is gone from the page", Vec::new()),
            ("r-3", MarginRoundKind::Change, true, vec!["m2"], "A change waits for your Accept", Vec::new()),
        ],
        "planning-today" => vec![
            (
                "m-2",
                MarginRoundKind::SentBack,
                false,
                Vec::new(),
                "Sent back with your notes",
                vec![note("How it works", "Notices that arrive wait until 9:00 and come in as one digest", "Why 9:00? I start at 8, and on Fridays I want the digest at 7:30 so I can clear it before standup.")],
            ),
            (
                "m-4",
                MarginRoundKind::SentBack,
                false,
                Vec::new(),
                "Sent back with your notes",
                vec![
                    note("How it works", "Notices that arrive wait until 9:00 and come in as one digest", "Yes, tie it to my work start."),
                    note("How it works", "A failed check still interrupts.", "Should a failed check on a branch wake me? I only care about main."),
                ],
            ),
        ],
        _ => return,
    };
    for (at, (row, kind, waits_on_you, threads, summary, notes)) in cards.into_iter().enumerate() {
        let Some(place) = thread.items.iter().position(|item| matches!(item, ThreadItem::Row { row: one } if one.id() == row)) else { continue };
        let ThreadItem::Row { row: one } = &thread.items[place] else { continue };
        let card = ThreadMargin {
            id: format!("margin:{}", at + 1),
            replaces: Some(row.to_string()),
            at: one.at().clone(),
            page: "quiet".into(),
            title: "Quiet hours for notices".into(),
            kind,
            summary: summary.into(),
            waits_on_you,
            threads: threads.into_iter().map(str::to_string).collect(),
            notes,
        };
        thread.items.insert(place + 1, ThreadItem::Margin(card));
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
}
