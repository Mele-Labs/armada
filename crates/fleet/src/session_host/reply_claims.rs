//! Whether a reply claims what the agent did not back with its call, read from how the reply ends.
//! `docs/concepts/session/waiting-on-the-person.md`.
//!
//! **Fleet takes a Job's word only through a call**: a done is `all_done`, a walk is `show_window`. A
//! reply that says either in prose leaves the person with no Done item and no walk window, so the turn
//! it ended is followed by one reminder through `told_by_fleet`, and the agent makes the call or says
//! what is left.
//!
//! **Phrase rules for now, not the language model**: #2692 is the model reader. The readers are pure,
//! text and facts in, so each rule is tested without a Fleet. **They learn from disputes**: a reminder
//! a job disputed and the model agreed was wrong keeps the sentences that set it off, and a near one
//! is not reminded again, so a misfire is corrected without a change to the rules.
//!
//! **One reminder a claim.** The reply to a reminder is not read again: the reminder is kept as the
//! session's tell, and the first reply after it is the one `keep_turn_end` marks. `session_tells`
//! holds one row a session, so a reminder replaces an earlier tell, which costs nothing while the
//! refusal reader is not in force (`tell_refusal::IN_FORCE`).

use std::collections::HashMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings as keys;
use ipc::{SessionRow, SessionVoice, WaitingSource};

use super::dedup::{left_off_as_misread_sentences, FOLDS_AT};
use super::disputing::Disputing;
use crate::daemon::Fleet;

/// What the agent reads when its reply says the work is done and `all_done` is not called.
pub(crate) const DONE_REMINDER: &str =
    "Your reply says the work is done, and all_done is not called. If nothing is left, call all_done; if something is, say what.";

/// What the agent reads when its reply asks the owner to walk something and no window of it is open.
pub(crate) const WALK_REMINDER: &str = "Your reply asks the owner to walk something, and no walk window of this job is open. Build each walk and call show_window for it.";

/// The tells as `store::KeptTell::kind` spells them.
pub(crate) const REMINDED_DONE: &str = "reminded_done";
pub(crate) const REMINDED_WALK: &str = "reminded_walk";

/// A claim a reply makes that the agent's call does not back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Claim {
    Done,
    Walk,
}

impl Claim {
    pub(crate) fn kind(self) -> &'static str {
        match self {
            Claim::Done => REMINDED_DONE,
            Claim::Walk => REMINDED_WALK,
        }
    }

    pub(crate) fn reminder(self) -> &'static str {
        match self {
            Claim::Done => DONE_REMINDER,
            Claim::Walk => WALK_REMINDER,
        }
    }
}

/// What stands for the Session as its turn ends.
pub(crate) struct Backed {
    /// `done_since` stands.
    pub done: bool,
    /// Anything is on its waiting list.
    pub waiting: bool,
    /// A walk window of it waits on the person.
    pub walk: bool,
    /// A subagent of it is still at work, so the turn that ended is not the end of its work.
    pub subagents: bool,
}

/// The claim of `reply` that nothing standing backs. A walk is owed before a done can be true, so
/// a reply making both is reminded of the walk. **Nothing is reminded while `all_done` stands**: the
/// agent has said nothing is left, and the turn a reminder starts would clear the done and take the
/// person's end panel away on a misread.
#[cfg(test)]
pub(crate) fn unbacked(reply: &str, backed: &Backed) -> Option<Claim> {
    decided(asks_to_walk(reply), claims_done(reply), backed)
}

/// [`unbacked`] from what the reply was read to say.
fn decided(walk: bool, done: bool, backed: &Backed) -> Option<Claim> {
    if backed.done || backed.subagents {
        return None;
    }
    if !backed.walk && walk {
        return Some(Claim::Walk);
    }
    (!backed.done && !backed.waiting && done).then_some(Claim::Done)
}

/// Words that put a done later or take it back: "once ci passes", "is not done".
const DEFERS: &[&str] = &["not", "once", "after", "when", "until", "if", "before"];
/// What a done is said of. Not "task": one of a plan's tasks done is not the job.
const SUBJECTS: &[&str] = &["work", "job", "everything"];
/// What it is said to be.
const OVER: &[&str] = &["done", "finished", "complete", "completed"];
/// The word before [`OVER`] that makes it a state: "is done", never "have done the work".
const STATES: &[&str] = &["is", "are", "was", "now", "all", "fully", "been"];
/// Phrases that say it whole. `all_done` is the tool's name and does not match "all done".
const NOTHING_LEFT: &[&str] = &["nothing left", "nothing more to do", "nothing else to do", "nothing remains", "all done"];

/// The phrases of `walk_asks::WALKING` that read the same in a reply. "In the window" is left out,
/// since a reply says it of much else.
const WALKING: &[&str] = &["?walk=", "walk window", "leave a note", "leave notes"];
/// What he is asked to do with a walk named in the same sentence.
/// Not "review" or "open": a reply describing walks says those as often as one asking for them.
const WALK_ACTS: &[&str] = &["approve", "leave notes"];
/// Words that say no walk is asked.
const NEGATES: &[&str] = &["not", "no", "never"];

/// Whether the reply's close says the work is done. A reply too short for the close's rules, such as
/// "All done.", is read whole.
pub(crate) fn claims_done(reply: &str) -> bool {
    !done_sentences(reply).is_empty()
}

/// The sentences of the close that say the work is done.
pub(crate) fn done_sentences(reply: &str) -> Vec<String> {
    let reply = unquoted(reply);
    let close = super::asks::closing_sentences(&reply);
    if close.is_empty() {
        return Some(reply.trim().to_string()).filter(|whole| says_done(whole)).into_iter().collect();
    }
    close.into_iter().filter(|sentence| says_done(sentence)).collect()
}

/// Whether the reply asks the owner to walk, approve or leave notes on something: in its close, or in
/// a cell of a table near its end, where an agent lists what he has to do.
pub(crate) fn asks_to_walk(reply: &str) -> bool {
    !walk_sentences(reply).is_empty()
}

/// The sentences and table cells of the close that ask the owner to walk something.
pub(crate) fn walk_sentences(reply: &str) -> Vec<String> {
    let reply = unquoted(reply);
    let reply = reply.as_str();
    let tail = super::standing::tail(reply);
    let cells = tail
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|'))
        .flat_map(|row| row.split('|').map(|cell| cell.trim().to_string()).collect::<Vec<_>>());
    super::asks::closing_sentences(reply).into_iter().chain(cells).filter(|sentence| asks_walk(sentence)).collect()
}

/// The sentences of `reply` that make `claim`.
fn claim_sentences(reply: &str, claim: Claim) -> Vec<String> {
    match claim {
        Claim::Walk => walk_sentences(reply),
        Claim::Done => done_sentences(reply),
    }
}

/// The claim whose reminder `notice` is, if it is one.
fn reminded(notice: &str) -> Option<Claim> {
    [Claim::Walk, Claim::Done].into_iter().find(|claim| notice.trim() == claim.reminder())
}

fn says_done(sentence: &str) -> bool {
    let lower = sentence.to_lowercase().replace('’', "'");
    if lower.trim_end().ends_with('?') {
        return false;
    }
    let words = words(&lower);
    if negated(&words, DEFERS) {
        return false;
    }
    NOTHING_LEFT.iter().any(|phrase| lower.contains(phrase))
        || words.iter().enumerate().skip(1).any(|(at, word)| {
            let before = words[at - 1];
            OVER.contains(word)
                && (STATES.contains(&before) || before.ends_with("'s"))
                && words[..at].iter().any(|one| SUBJECTS.contains(&one.trim_end_matches("'s")))
        })
}

fn asks_walk(sentence: &str) -> bool {
    let lower = sentence.to_lowercase().replace('’', "'");
    let words = words(&lower);
    if negated(&words, NEGATES) {
        return false;
    }
    WALKING.iter().any(|phrase| lower.contains(phrase))
        || (words.iter().any(|word| matches!(*word, "walk" | "walks")) && WALK_ACTS.iter().any(|act| lower.contains(act)))
}

/// The reply with what it quotes in backticks or double quotes taken out: words quoted are about a
/// reply, as when an agent explains this reader, not its own claim. A quoted walk url is the one thing
/// kept, since agents put the url they ask him to open in backticks.
fn unquoted(reply: &str) -> String {
    let mut kept = String::with_capacity(reply.len());
    let mut quoted: Option<(char, String)> = None;
    for c in reply.chars() {
        match &mut quoted {
            None if matches!(c, '`' | '"' | '“') => quoted = Some((if c == '“' { '”' } else { c }, String::new())),
            None => kept.push(c),
            Some((close, inside)) if c == *close => {
                kept.push_str(if inside.contains("?walk=") { " ?walk= " } else { " " });
                quoted = None;
            }
            // A quote left open at a line's end was no quote: the line is read as written.
            Some((open, inside)) if c == '\n' => {
                kept.push(if *open == '”' { '“' } else { *open });
                kept.push_str(inside);
                kept.push(c);
                quoted = None;
            }
            Some((_, inside)) => inside.push(c),
        }
    }
    if let Some((open, inside)) = quoted {
        kept.push(if open == '”' { '“' } else { open });
        kept.push_str(&inside);
    }
    kept
}

/// The agent's last reply before the row `notice_row`: the one its reminder was about.
fn reply_before<'a>(rows: &'a [SessionRow], notice_row: &str) -> Option<&'a str> {
    let at = rows.iter().position(|row| row.id() == notice_row)?;
    rows[..at].iter().rev().find_map(|row| match row {
        SessionRow::Message { from: SessionVoice::Agent, text, .. } => Some(text.as_str()),
        _ => None,
    })
}

/// What the model is asked of a disputed reminder; a test's model double answers on it.
pub(crate) const CHECKING: &str = "Did those sentences";
const MISREAD: &str = "misread:";

/// The question put to the model about a disputed reminder of `claim`.
fn checking(claim: Claim, sentences: &[String], why: &str, reply: &str) -> String {
    let asked = match claim {
        Claim::Walk => "ask the person, now, to open, walk, approve or leave notes on something shown to them",
        Claim::Done => "say that the agent's whole job is finished",
    };
    let listed: Vec<String> = sentences.iter().map(|one| format!("- {one}")).collect();
    format!(
        "A tool that supervises a coding agent sent it this reminder:\n\n{reminder}\n\n\
         It was set off by these sentences of the agent's reply:\n\n{listed}\n\n\
         The agent disputed it: {why}\n\n\
         The agent's whole reply:\n\n{reply}\n\n\
         {CHECKING} {asked}? A step described for later, a condition or a summary does not.\n\n\
         Answer in one line and nothing else:\n\
         {MISREAD} yes where they did not, so the reminder was wrong, or no where they did",
        reminder = claim.reminder(),
        listed = listed.join("\n"),
        why = why.trim(),
        reply = reply.trim(),
    )
}

/// The model's answer, read. Anything but a `yes` keeps the reminder as it was.
fn misread(answer: &str) -> bool {
    answer.lines().any(|line| {
        let line = line.trim().to_lowercase();
        line.strip_prefix(MISREAD).is_some_and(|said| said.trim().starts_with("yes"))
    })
}

/// The words of a lowercased sentence, an apostrophe kept inside one.
fn words(sentence: &str) -> Vec<&str> {
    sentence.split(|c: char| !c.is_alphanumeric() && c != '\'').filter(|word| !word.is_empty()).collect()
}

fn negated(words: &[&str], by: &[&str]) -> bool {
    words.iter().any(|word| by.contains(word) || word.ends_with("n't"))
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The claim of the reply a turn ended on at `ended` that nothing backs, where Fleet reminds of one:
    /// never for Helm, an ended Session or the reply to a reminder.
    pub(crate) async fn unbacked_claim(&self, id: &str, ended: &str, said: Option<&str>) -> Option<Claim> {
        // Most replies claim nothing, and the store is not read for them.
        let said = said.filter(|said| asks_to_walk(said) || claims_done(said))?;
        if self.hosts().of(id).state().helm {
            return None;
        }
        let (session, reads, done, subagents) = {
            let store = self.store().lock().await;
            let session = store.session(id).ok().flatten()?;
            if session.state == store::SessionState::Ended {
                return None;
            }
            let tell = store.tell_of(id).ok().flatten();
            let answers = tell.is_some_and(|tell| {
                [REMINDED_DONE, REMINDED_WALK].contains(&tell.kind.as_str()) && tell.first_end.as_deref() == Some(ended)
            });
            if answers {
                return None;
            }
            let done = store.done_since(id).ok().flatten().is_some();
            let subagents = store
                .attachments_of(&store::Holder::session(id))
                .ok()?
                .iter()
                .any(|one| one.kind == "subagent" && one.state == store::AttachmentState::Standing);
            let reads = self.ledger_reads(&store, &session).ok()?;
            (session, reads, done, subagents)
        };
        let items = self.ledger_waiting(&session, reads);
        let backed = Backed {
            done,
            waiting: !items.is_empty(),
            walk: items.iter().any(|item| item.source == WaitingSource::Walk),
            subagents,
        };
        let walk = self.not_misread(Claim::Walk, walk_sentences(said)).await;
        let done = self.not_misread(Claim::Done, done_sentences(said)).await;
        decided(!walk.is_empty(), !done.is_empty(), &backed)
    }

    /// `sentences` with every one near a sentence kept as not making `claim` left off: the same words,
    /// or within [`FOLDS_AT`] of one once the language model is loaded. A dispute the model agreed with
    /// keeps them, so the reminder learns without a change to these rules.
    async fn not_misread(&self, claim: Claim, sentences: Vec<String>) -> Vec<String> {
        if sentences.is_empty() {
            return sentences;
        }
        let misreads = self.store().lock().await.claim_misreads(claim.kind()).unwrap_or_default();
        let sentences: Vec<String> = sentences.into_iter().filter(|one| !misreads.contains(one)).collect();
        if sentences.is_empty() || misreads.is_empty() {
            return sentences;
        }
        let Some(model) = self.language_model().model() else { return sentences };
        // Read now, not on a later pass as the Needs you list is: the turn's end is the one look.
        let texts: Vec<String> = sentences.iter().chain(&misreads).cloned().collect();
        let embedded = tokio::task::spawn_blocking(move || {
            let borrowed: Vec<&str> = texts.iter().map(String::as_str).collect();
            model.embed(&borrowed).ok().filter(|vectors| vectors.len() == texts.len()).map(|vectors| texts.into_iter().zip(vectors).collect::<HashMap<_, _>>())
        })
        .await;
        let Ok(Some(vectors)) = embedded else { return sentences };
        left_off_as_misread_sentences(sentences, &misreads, |text| vectors.get(text).cloned(), FOLDS_AT)
    }

    /// Where `dispute` is of a walk or done reminder and the model agrees the sentences that set it off
    /// made no such claim, keep them as misread, so the reminder holds near ones off in every Session.
    /// Whether they were kept: a dispute learnt from needs no review job.
    pub(crate) async fn claim_learnt(&self, dispute: &Disputing<'_>, rows: &[SessionRow]) -> bool {
        let Some(claim) = reminded(dispute.notice) else { return false };
        let Some(reply) = reply_before(rows, dispute.notice_row) else { return false };
        let sentences = claim_sentences(reply, claim);
        if sentences.is_empty() {
            return false;
        }
        let model = self.machine_settings().now().get(keys::MODELS_DISPUTE);
        let agreed = match self.dispute_asked(&checking(claim, &sentences, dispute.why, reply), &model).await {
            Ok(answer) => misread(&answer),
            Err(why) => {
                eprintln!("job {}: a disputed reminder was not checked, so nothing was learnt: {why}", dispute.session);
                false
            }
        };
        if !agreed {
            return false;
        }
        let at = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        sentences.iter().all(|sentence| store.keep_claim_misread(dispute.session, claim.kind(), sentence, &at).is_ok())
    }

    /// Remind a hosted Session of `claim`, and keep that it was once the words are taken.
    pub(crate) async fn claim_reminded(self: &Arc<Self>, id: &str, claim: Claim) {
        if matches!(self.noticed_by_fleet(id, claim.reminder(), ipc::Notice::ReplyClaim).await, Ok(told) if told.sent()) {
            self.tell_kept(id, claim.kind()).await;
        }
    }

    /// [`Self::claim_reminded`] for a terminal Session: the words are held for its mod, as
    /// `told_by_fleet` holds them, and nothing is kept where the mod is not listening.
    pub(crate) async fn terminal_claim_reminded(&self, id: &str, claim: Claim) {
        let said = format!("{}: {}", super::serving::FLEET_NAME, claim.reminder());
        if self.hosts().terminals().hold(id, said, super::terminal::LISTENING_FOR) {
            self.tell_kept(id, claim.kind()).await;
        }
    }
}
