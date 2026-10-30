//! The writes a saved rule makes to a forge's own issues: close, reopen, assign.
//!
//! **A forge's issue is open or closed, and nothing else Fleet writes.** A project column is a
//! field of a board, reached by ids this does not hold, so a move to any status but the forge's own
//! `Open` and `Closed` is refused, in a sentence, before anything is planned. The arguments are
//! built apart from the run, so what is asked of `gh` is read without it.

use adapter_traits::{IssueWrite, NotTracked, TrackerWrites};

use crate::delivery::{run_in, said, FORGE};
use crate::IssueTracker;

/// What `tracker_word` calls an issue that is open and one that is closed, where no board says otherwise.
const OPEN: &str = "Open";
const CLOSED: &str = "Closed";

impl TrackerWrites for IssueTracker {
    fn can(&self, write: &IssueWrite<'_>) -> Result<(), String> {
        arguments("1", write).map(drop)
    }

    fn write(&self, in_repo: &str, reference: &str, write: &IssueWrite<'_>) -> Result<(), NotTracked> {
        let doing = write.doing();
        let number = number_of(reference).ok_or_else(|| NotTracked::of(doing, format!("`{reference}` is not an issue number")))?;
        let args = arguments(&number, write).map_err(|why| NotTracked::of(doing, why))?;
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let run = run_in(in_repo, FORGE, &args).map_err(|why| NotTracked::of(doing, format!("`{FORGE}` would not run: {why}")))?;
        match run.status.success() {
            true => Ok(()),
            false => Err(NotTracked::of(doing, said(&run))),
        }
    }
}

/// `12` from `#12` or `12`, and nothing from any other text.
fn number_of(reference: &str) -> Option<String> {
    reference.trim().trim_start_matches('#').parse::<u64>().ok().map(|number| number.to_string())
}

/// The `gh` arguments that make `write` to issue `number`, or why `gh` cannot.
///
/// `=` joins an assignee to its flag, so a name that begins with a dash is a name and not a flag.
pub(crate) fn arguments(number: &str, write: &IssueWrite<'_>) -> Result<Vec<String>, String> {
    let to = |verb: &str| vec!["issue".to_string(), verb.to_string(), number.to_string()];
    match write {
        IssueWrite::Move { status } if *status == OPEN => Ok(to("reopen")),
        IssueWrite::Move { status } if *status == CLOSED => Ok(to("close")),
        IssueWrite::Move { status } => {
            Err(format!("GitHub issues move only between {OPEN} and {CLOSED}, and not to {status}"))
        }
        IssueWrite::Close { .. } => Ok(to("close")),
        IssueWrite::Assign { to: who } => match who.trim() {
            name if name.is_empty() || name.contains(char::is_whitespace) || name.contains(',') => {
                Err(format!("`{who}` is not a GitHub login"))
            }
            name => Ok(vec!["issue".to_string(), "edit".to_string(), number.to_string(), format!("--add-assignee={name}")]),
        },
    }
}
