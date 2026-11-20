//! What Now leads with: the act this second, then the last five before it, newest first, then the subagents at work. The
//! act this second is the agent's row (`now.rs`); this file is the five that came before it.

use gpui::{div, prelude::*, AnyElement, Role, SharedString};
use ipc::SessionRow;

use super::now::{beat, lead, named, since, words};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// How many acts Now keeps behind the one under way.
pub(super) const KEPT: usize = 5;

/// One act that is over: the call's own words and when it began.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Act {
    pub id: String,
    pub said: String,
    pub at: String,
}

/// The last [`KEPT`] calls of the thread, newest first. The call still open is the act this second, so it is not one of them;
/// a call that started a subagent is that subagent's row, and a message sent to another Job is not an act of this one.
pub(super) fn last_acts(rows: &[SessionRow], open: bool) -> Vec<Act> {
    let calls = rows.iter().filter_map(|row| match row {
        SessionRow::Tool { subagent: Some(_), .. } | SessionRow::Tool { message: Some(_), .. } => None,
        SessionRow::Tool { id, at, text, description, .. } => {
            let said = match description.as_deref().map(str::trim).filter(|said| !said.is_empty()) {
                Some(said) => said.to_string(),
                None => super::calls::call_line(text, false),
            };
            Some(Act { id: id.clone(), said, at: at.as_str().to_string() })
        }
        _ => None,
    });
    let mut done: Vec<Act> = calls.collect();
    if open {
        done.pop();
    }
    done.into_iter().rev().take(KEPT).collect()
}

impl JobView {
    /// The last five acts as rows, each with how long ago it began. None where the Job has done none.
    pub(super) fn now_last_acts(&self, acts: &[Act], look: &Look) -> Option<AnyElement> {
        if acts.is_empty() {
            return None;
        }
        let rows = acts.iter().map(|act| {
            let key = format!("act:{}", act.id);
            labelled(
                named(&key, beat(look, lead(look, &key, "check", token::FG_SUBTLE, "Done", false), words(look, act.said.clone(), token::FG_MUTED, false, false, false), since(look, &key, &act.at, "Began"), None)),
                Role::ListItem,
                act.said.clone(),
            )
            .into_any_element()
        });
        Some(labelled(div().id(SharedString::from("now-last-acts")).flex().flex_col().children(rows), Role::List, "Last acts").into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(id: &str, text: &str) -> SessionRow {
        SessionRow::Tool { id: id.into(), at: ipc::Instant::carried("2026-11-20T10:00:00Z"), text: text.into(), description: None, subagent: None, image: None, message: None }
    }

    fn said(acts: &[Act]) -> Vec<&str> {
        acts.iter().map(|act| act.id.as_str()).collect()
    }

    #[test]
    fn now_keeps_the_last_five_acts_newest_first_and_leaves_the_open_call_to_the_act_this_second() {
        let rows: Vec<SessionRow> = (1..=8).map(|n| call(&format!("c{n}"), "Read /a")).collect();
        assert_eq!(said(&last_acts(&rows, false)), ["c8", "c7", "c6", "c5", "c4"]);
        assert_eq!(said(&last_acts(&rows, true)), ["c7", "c6", "c5", "c4", "c3"], "c8 is under way, so it is the act this second");
        assert_eq!(said(&last_acts(&rows[..2], false)), ["c2", "c1"], "fewer than five are all there are");
        assert!(last_acts(&[], true).is_empty());
    }

    #[test]
    fn an_act_says_the_calls_own_phrase_where_it_has_one() {
        let mut rows = vec![call("c1", "Read /a"), call("c2", "Read /b")];
        if let SessionRow::Tool { description, .. } = &mut rows[1] {
            *description = Some("Reads the sync code".into());
        }
        let acts = last_acts(&rows, false);
        assert_eq!(acts[0].said, "Reads the sync code", "the call's own phrase where it has one");
    }
}
