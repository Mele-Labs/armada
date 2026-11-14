//! `get_needs_you`: a Job's Needs you worked out once, so each Bridge draws the same band.
//! `crates/ipc/operations/get_needs_you.toml` is the contract.
//!
//! **A Bridge draws this and derives nothing.** It reads the Job's `waiting_for`, after the person's
//! dismissals, and gives every item to exactly one section. [`rule`] is the one table of what each source
//! is; `docs/concepts/session/waiting-on-the-person.md` states it again, and a test holds the two to it.

use api::Refusal;
use ipc::{
    BuildSource, NeedsYou, NeedsYouAct, NeedsYouDismiss as Dismiss, NeedsYouGlyph, NeedsYouGlyphKind as Kind, NeedsYouRow,
    NeedsYouSection, NeedsYouSectionKind as Section, WaitingActKind, WaitingItem, WaitingSource, DOCUMENT_ITEM_PREFIX,
};

use super::drafting::DRAFT_PREFIX;
use super::sketching::TARGET_PREFIX as SKETCH_PREFIX;
use super::waiting::HELD_CHILD_SETTLED_BY;

/// Where a source's rows go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    /// A page he reads: one row of To review.
    Review,
    /// Always a question, whatever its act.
    Questions,
    /// Always a walk.
    Walks,
    /// By what pressing it does: a walk, a command or a pull request go to their own section, a restart to the
    /// band's key, and anything with choices or an answer is a question. An agent's item that is none of these is
    /// something he does, so it is To do.
    ByAct,
    /// The agent's all_done, which the band does not fold.
    Done,
}

struct Rule {
    place: Place,
    dismiss: Dismiss,
}

const fn rule(place: Place, dismiss: Dismiss) -> Rule {
    Rule { place, dismiss }
}

/// **The rule per source, in one table.** Exhaustive, so a new `WaitingSource` is not built until it is placed.
/// An `agent` item with choices or an `answer` act is a question, one with a `walk`, `run` or `approve_pr` act goes
/// to Walks, Commands or PRs, and any other, steps included, is To do and takes `done_waiting`.
/// `dismiss` is what the row offers: where it is not `None`, `dismiss_waiting` takes the item. The row of a
/// `replied` item that asks to open a pull request is `Plain`, not `NotAQuestion`: see [`dismissal_of`].
///
/// | source | place | dismiss |
/// |---|---|---|
/// | `agent` | by act: to do unless it asks or runs | reason |
/// | `ask_card`, `permission`, `window_ask`, `first_message` | questions | none: its card answers it |
/// | `walk` | walks | plain |
/// | `replied` | by act | not a question |
/// | `draft` | to review | reason |
/// | `handoff` | questions | plain: Stay |
/// | `sketch`, `document`, `plan` | to review | plain |
/// | `done` | done | plain |
/// | `slot_choice`, `delivery`, `task_done`, `held_child`, `gate` | questions | none: Fleet refuses it |
/// | `words_address`, `fleet_call`, `watch_not_ours`, `browser_ask` | questions | plain |
/// | `preview_offer` | by act | skip; Fleet no longer raises it |
/// | `rule_act`, `operator_suggestion` | questions | skip |
fn rule_of(source: WaitingSource) -> Rule {
    use Place::{ByAct, Done, Questions, Review, Walks};
    match source {
        WaitingSource::Agent => rule(ByAct, Dismiss::Reason),
        WaitingSource::AskCard => rule(Questions, Dismiss::None),
        WaitingSource::Walk => rule(Walks, Dismiss::Plain),
        WaitingSource::Permission => rule(Questions, Dismiss::None),
        WaitingSource::WindowAsk => rule(Questions, Dismiss::None),
        WaitingSource::Draft => rule(Review, Dismiss::Reason),
        WaitingSource::Handoff => rule(Questions, Dismiss::Plain),
        WaitingSource::Sketch => rule(Review, Dismiss::Plain),
        WaitingSource::Document => rule(Review, Dismiss::Plain),
        WaitingSource::FirstMessage => rule(Questions, Dismiss::None),
        WaitingSource::Replied => rule(ByAct, Dismiss::NotAQuestion),
        WaitingSource::Done => rule(Done, Dismiss::Plain),
        WaitingSource::Plan => rule(Review, Dismiss::Plain),
        WaitingSource::SlotChoice => rule(Questions, Dismiss::None),
        WaitingSource::Delivery => rule(Questions, Dismiss::None),
        WaitingSource::WordsAddress => rule(Questions, Dismiss::Plain),
        WaitingSource::FleetCall => rule(Questions, Dismiss::Plain),
        WaitingSource::TaskDone => rule(Questions, Dismiss::None),
        WaitingSource::WatchNotOurs => rule(Questions, Dismiss::Plain),
        WaitingSource::HeldChild => rule(Questions, Dismiss::None),
        WaitingSource::PreviewOffer => rule(ByAct, Dismiss::Skip),
        WaitingSource::RuleAct => rule(Questions, Dismiss::Skip),
        WaitingSource::OperatorSuggestion => rule(Questions, Dismiss::Skip),
        WaitingSource::BrowserAsk => rule(Questions, Dismiss::Plain),
        WaitingSource::Gate => rule(Questions, Dismiss::None),
    }
}

/// Why `dismiss_waiting` refuses an item of `source`: it is answered, not dismissed. **The one place that
/// says so**: `dismiss_waited` reads it, and the table above is held to it.
pub(crate) fn refused_dismissal(source: WaitingSource) -> Option<&'static str> {
    match source {
        WaitingSource::Delivery => Some("how the work is delivered is answered, not dismissed"),
        WaitingSource::SlotChoice => Some("whether a job waits for a slot or has one added is answered, not dismissed"),
        WaitingSource::TaskDone => Some("whether the tasks are done is answered, not dismissed"),
        WaitingSource::HeldChild => Some(HELD_CHILD_SETTLED_BY),
        WaitingSource::Gate => Some("a gate is approved or its group skipped, not dismissed"),
        _ => None,
    }
}

/// How the row is put away: the source's, except that a `replied` item asking to open a pull request is a
/// request and not a sentence, so there is no question to say it is not.
fn dismissal_of(item: &WaitingItem, act: &NeedsYouAct) -> Dismiss {
    match (item.source, act) {
        (WaitingSource::Replied, NeedsYouAct::ApprovePr { .. }) => Dismiss::Plain,
        (source, _) => rule_of(source).dismiss,
    }
}

/// The build a command switches to, as `scripts/preview --restart` and `scripts/restart --main` do, with the
/// key's words. `--adopt` may go with either; any other flag is some other command, a dry run among them.
fn restart_of(command: &str) -> Option<(&'static str, BuildSource, bool)> {
    let mut words = command.split_whitespace();
    let script = words.next().unwrap_or_default();
    let flags: Vec<&str> = words.collect();
    let (label, build, flag) = if script == "scripts/preview" || script.ends_with("/scripts/preview") {
        ("Refresh preview", BuildSource::Preview, "--restart")
    } else if script == "scripts/restart" || script.ends_with("/scripts/restart") {
        ("Update to main", BuildSource::Main, "--main")
    } else {
        return None;
    };
    (flags.contains(&flag) && flags.iter().all(|one| *one == flag || *one == "--adopt")).then_some((label, build, flags.contains(&"--adopt")))
}

/// Whether words go as an answer besides the choices. Fleet refuses them on a slot call and a page's ask has
/// no use for them, nor does what the Operator would do.
fn takes_words(source: WaitingSource) -> bool {
    !matches!(source, WaitingSource::SlotChoice | WaitingSource::BrowserAsk | WaitingSource::OperatorSuggestion | WaitingSource::Handoff)
}

/// What pressing the item does. **An agent's item that asks nothing and runs nothing is something he does**, so
/// its act is Done, carrying its steps.
fn act_of(item: &WaitingItem, job: &str) -> NeedsYouAct {
    let act = pressed(item, job);
    if is_to_do(item, &act) {
        NeedsYouAct::Done { steps: item.steps.clone() }
    } else {
        act
    }
}

/// Whether an agent's item is a thing for him to do: no choices, no answer asked, and no walk, command, restart
/// or pull request to press. A procedure with its command is a command; one with none is for him to follow.
fn is_to_do(item: &WaitingItem, act: &NeedsYouAct) -> bool {
    let answers = item.act.as_ref().is_some_and(|act| act.kind == WaitingActKind::Answer);
    item.source == WaitingSource::Agent
        && item.options.is_empty()
        && !answers
        && !matches!(
            act,
            NeedsYouAct::Walk { .. }
                | NeedsYouAct::Run { .. }
                | NeedsYouAct::Restart { .. }
                | NeedsYouAct::ApprovePr { .. }
                | NeedsYouAct::Steps { command: Some(_), .. }
        )
}

/// What pressing the item does, before it is known to be a thing to do. A `message` act naming another Job opens
/// that Job, and an agent's or a read sentence's `answer` with no choices has no card to go to, so it takes words.
fn pressed(item: &WaitingItem, job: &str) -> NeedsYouAct {
    let named = |prefix: &str| item.id.strip_prefix(prefix).unwrap_or(&item.id).to_string();
    match item.source {
        WaitingSource::Plan => return NeedsYouAct::OpenPlan,
        WaitingSource::Sketch => return NeedsYouAct::OpenSketch { title: named(SKETCH_PREFIX) },
        WaitingSource::Document => return NeedsYouAct::OpenDocument { path: named(DOCUMENT_ITEM_PREFIX) },
        WaitingSource::Draft => return NeedsYouAct::OpenDraft { row: named(DRAFT_PREFIX) },
        _ => {}
    }
    if !item.options.is_empty() {
        return NeedsYouAct::Answer { options: item.options.clone(), other: takes_words(item.source) };
    }
    let command = item.act.as_ref().filter(|act| act.kind == WaitingActKind::Run).map(|act| act.target.clone());
    // A restart is the band's key even where the agent wrote steps for it, as Classic draws it.
    if let Some((label, build, adopt)) = command.as_deref().and_then(restart_of) {
        return NeedsYouAct::Restart { label: label.into(), build, adopt, command: command.clone().unwrap_or_default() };
    }
    if !item.steps.is_empty() {
        return NeedsYouAct::Steps { steps: item.steps.clone(), command };
    }
    let words = matches!(item.source, WaitingSource::Agent | WaitingSource::Replied);
    let Some(act) = &item.act else {
        return if words { NeedsYouAct::Reply } else { NeedsYouAct::None };
    };
    match act.kind {
        WaitingActKind::Walk => NeedsYouAct::Walk { url: act.target.clone() },
        WaitingActKind::Answer if words => NeedsYouAct::Reply,
        WaitingActKind::Answer => NeedsYouAct::Answer { options: Vec::new(), other: false },
        WaitingActKind::ApprovePr => NeedsYouAct::ApprovePr { target: act.target.clone() },
        WaitingActKind::Run => match restart_of(&act.target) {
            Some((label, build, adopt)) => NeedsYouAct::Restart { label: label.into(), build, adopt, command: act.target.clone() },
            None => NeedsYouAct::Run { command: act.target.clone() },
        },
        WaitingActKind::Message if act.target != job => NeedsYouAct::OpenJob { id: act.target.clone() },
        WaitingActKind::Message => NeedsYouAct::Message,
    }
}

fn glyph(kind: Kind, said: &str) -> NeedsYouGlyph {
    NeedsYouGlyph { kind, said: said.to_string() }
}

/// The mark a row carries and what its tooltip names it. A page he reads has its own; the rest are read as
/// Bridge reads a row: the Operator's, a sentence read from a reply, a question, a linked Job, a reply, then
/// by what pressing does.
fn glyph_of(item: &WaitingItem, act: &NeedsYouAct) -> NeedsYouGlyph {
    match item.source {
        WaitingSource::Plan => return glyph(Kind::Plan, "Plan"),
        WaitingSource::Sketch => return glyph(Kind::Sketch, "Sketch"),
        WaitingSource::Draft => return glyph(Kind::Draft, "Draft"),
        WaitingSource::Document => return glyph(Kind::Document, "Document"),
        WaitingSource::Done => return glyph(Kind::Done, "Done"),
        WaitingSource::OperatorSuggestion => return glyph(Kind::Operator, "What the Operator would do"),
        _ => {}
    }
    let asks = matches!(act, NeedsYouAct::Answer { options, .. } if !options.is_empty());
    let replies = matches!(act, NeedsYouAct::Reply);
    match act {
        NeedsYouAct::Done { .. } => glyph(Kind::Todo, "To do"),
        NeedsYouAct::Restart { label, .. } => glyph(Kind::Restart, label),
        _ if item.source == WaitingSource::Replied && (asks || replies) => glyph(Kind::ReadFromReply, "Read from the reply"),
        _ if asks => glyph(Kind::Question, "Question"),
        _ if item.source == WaitingSource::HeldChild => glyph(Kind::LinkedJob, "Opens the linked Job"),
        NeedsYouAct::Reply => glyph(Kind::Reply, "Reply"),
        NeedsYouAct::Walk { .. } => glyph(Kind::Walk, "Opens the walk"),
        NeedsYouAct::Answer { .. } => glyph(Kind::Answer, "Goes to the question"),
        NeedsYouAct::ApprovePr { .. } => glyph(Kind::PullRequest, "Opens the pull request"),
        NeedsYouAct::Run { .. } => glyph(Kind::Run, "Copies the command"),
        NeedsYouAct::Message | NeedsYouAct::OpenJob { .. } => glyph(Kind::Message, "Goes to the message"),
        NeedsYouAct::Steps { .. } => glyph(Kind::Steps, "Steps"),
        _ => glyph(Kind::Waiting, "Waiting on you"),
    }
}

/// The section a row is drawn in.
fn section_of(item: &WaitingItem, act: &NeedsYouAct) -> Section {
    match rule_of(item.source).place {
        Place::Review => Section::ToReview,
        Place::Questions => Section::Questions,
        Place::Walks => Section::Walks,
        Place::Done => Section::Done,
        Place::ByAct => match act {
            // Fleet's offer may be put away, so it is listed with its sentence. Any other restart is the band's key.
            NeedsYouAct::Restart { .. } if item.source == WaitingSource::PreviewOffer => Section::Commands,
            NeedsYouAct::Restart { .. } => Section::Restart,
            NeedsYouAct::Walk { .. } => Section::Walks,
            NeedsYouAct::Done { .. } => Section::ToDo,
            NeedsYouAct::Run { .. } | NeedsYouAct::Steps { .. } => Section::Commands,
            NeedsYouAct::ApprovePr { .. } => Section::PullRequests,
            _ => Section::Questions,
        },
    }
}

const ORDER: [(Section, &str); 8] = [
    (Section::Questions, "Questions"),
    (Section::ToDo, "To do"),
    (Section::ToReview, "To review"),
    (Section::Walks, "Walks"),
    (Section::Commands, "Commands"),
    (Section::PullRequests, "PRs"),
    (Section::Restart, "Restart"),
    (Section::Done, "Done"),
];

/// The Job's Needs you from its `waiting_for`. **Every item is one row of exactly one section.**
pub(crate) fn needs_you_of(job: &str, items: &[WaitingItem]) -> NeedsYou {
    let rows: Vec<(Section, NeedsYouRow)> = items
        .iter()
        .map(|item| {
            let act = act_of(item, job);
            let row = NeedsYouRow {
                id: item.id.clone(),
                source: item.source,
                text: item.text.clone(),
                since: item.since.clone(),
                glyph: glyph_of(item, &act),
                dismiss: dismissal_of(item, &act),
                act,
            };
            (section_of(item, &row.act), row)
        })
        .collect();
    let owed = rows.iter().filter(|(section, _)| *section != Section::Done).count();
    let reviewing = rows.iter().filter(|(section, _)| *section == Section::ToReview).count();
    let alone_is_a_document = owed == 1 && rows.iter().any(|(_, row)| row.source == WaitingSource::Document);
    let review_in_band = reviewing > 1 || alone_is_a_document;
    let listed = |section: Section| matches!(section, Section::Questions | Section::ToDo | Section::Walks | Section::Commands | Section::PullRequests);

    let mut glyphs: Vec<NeedsYouGlyph> =
        rows.iter().filter(|(section, _)| listed(*section)).map(|(_, row)| glyph(row.glyph.kind, &row.text)).collect();
    if review_in_band {
        let mut pages: Vec<&NeedsYouRow> = rows.iter().filter(|(section, _)| *section == Section::ToReview).map(|(_, row)| row).collect();
        pages.sort_by(|a, b| a.since.as_str().cmp(b.since.as_str()));
        glyphs.extend(pages.into_iter().map(|row| row.glyph.clone()));
    }
    let fold = rows.iter().any(|(section, _)| listed(*section));
    let sections = ORDER
        .iter()
        .filter_map(|(kind, label)| {
            let mut mine: Vec<NeedsYouRow> = rows.iter().filter(|(section, _)| section == kind).map(|(_, row)| row.clone()).collect();
            if *kind == Section::ToReview {
                mine.sort_by(|a, b| a.since.as_str().cmp(b.since.as_str()));
            }
            (!mine.is_empty()).then(|| NeedsYouSection { kind: *kind, label: (*label).into(), rows: mine })
        })
        .collect();
    NeedsYou { session_id: ipc::SessionId::carried(job), sections, fold, review_in_band, glyphs }
}

impl<H, V, W> crate::daemon::Fleet<H, V, W>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_needs_you`: the Job's `waiting_for` as the sections the band draws.
    pub(crate) async fn needs_you_read(
        &self,
        caller: Option<api::Caller>,
        named: Option<ipc::SessionId>,
    ) -> Result<NeedsYou, Refusal> {
        let id = self.waiting_session(caller, named.as_ref())?;
        let own = {
            let store = self.store().lock().await;
            match store.session(&id).map_err(|why| self.ledger_fault(why))? {
                Some(session) => Some(self.ledger_row(&store, &session)?.waiting_for),
                None => None,
            }
        };
        // A Job his other Mac shares is read off the row held from it, as it last said it.
        let waiting = match own {
            Some(waiting) => waiting,
            None => self
                .sessions_from_macs()
                .await?
                .into_iter()
                .find(|one| one.id.as_str() == id)
                .map(|one| one.waiting_for)
                .ok_or_else(|| self.hosted_refusal(super::waiting::SESSION_WAITING_UNNAMED, "a job Fleet knows"))?,
        };
        Ok(needs_you_of(&id, &waiting))
    }
}
