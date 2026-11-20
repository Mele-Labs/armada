//! The steps a librarian run takes, named before a press and told as each moves, and Stop.
//! `docs/concepts/feature-catalog.md`, *Librarian*.

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use ipc::{Instant, LibrarianPart, LibrarianRunning, LibrarianState, LibrarianStep, LibrarianStepNow, LibrarianStepState};

/// The first step: what merged since the last run, and the sections Jobs wrote.
pub(super) const READ_MERGES: &str = "read_merges";
/// The drift part's step, free: linked documents read against the code. Its id is not the part's name.
pub(super) const CHECK_DRIFT: &str = "check_drift";
/// The last step: the catalog read through the newest merge, and the run's record kept.
pub(super) const READ_THROUGH: &str = "read_through";
/// What a step skipped for a press of Stop says.
pub(super) const STOPPED: &str = "Stopped";
/// What the paragraph step says where nothing moved.
pub(super) const NOTHING_NEW: &str = "Nothing new since the last run, so no call";
/// The most pull requests the first step's detail names. The rest are counted.
const MOST_NAMED: usize = 5;

/// What a built part's step is called. `None` for a part with no work behind it, which has no step.
fn title_of(part: LibrarianPart) -> Option<&'static str> {
    match part {
        LibrarianPart::SinceYouLooked => Some("Write the Since you looked paragraph"),
        LibrarianPart::Drift => Some("Check docs against the code"),
        _ => None,
    }
}

/// The steps a run would take now: what merged, each of `parts` that is `built`, then reading through. `spent` is what
/// tonight's runs took of `share`, and `docs` is how many documents are linked to features, which the drift step reads.
pub(super) fn steps_of(parts: &[LibrarianPart], built: &[LibrarianPart], state: &LibrarianState, share: u32, spent: u32, docs: usize) -> Vec<LibrarianStep> {
    let mut steps = vec![LibrarianStep { id: READ_MERGES.to_string(), title: "Read what merged".to_string(), detail: merges_detail(state), model: false }];
    for part in parts.iter().copied().filter(|part| built.contains(part)) {
        let Some(title) = title_of(part) else { continue };
        if part == LibrarianPart::Drift {
            steps.push(LibrarianStep { id: CHECK_DRIFT.to_string(), title: title.to_string(), detail: drift_detail(docs), model: false });
            continue;
        }
        let model = part.spends_model_share();
        let detail = if model { model_detail(share, spent) } else { String::new() };
        steps.push(LibrarianStep { id: part.name().to_string(), title: title.to_string(), detail, model });
    }
    let newest = state.merged_since.last().copied().or(state.read_through);
    steps.push(LibrarianStep { id: READ_THROUGH.to_string(), title: "Mark the catalog read".to_string(), detail: through_said(newest), model: false });
    steps
}

/// What the drift step reads, from the count of documents linked to features.
fn drift_detail(docs: usize) -> String {
    match docs {
        0 => "No docs are linked to features yet".to_string(),
        1 => "The doc linked to a feature, against the code at HEAD".to_string(),
        n => format!("The {n} docs linked to features, against the code at HEAD"),
    }
}

/// What the drift step found: how many documents name something the code no longer has.
pub(super) fn drift_said(flagged: usize) -> String {
    match flagged {
        0 => "No doc names what the code no longer has".to_string(),
        1 => "1 doc names what the code no longer has".to_string(),
        n => format!("{n} docs name what the code no longer has"),
    }
}

fn merges_detail(state: &LibrarianState) -> String {
    match (state.merged_since.as_slice(), state.read_through) {
        ([], Some(number)) => format!("Nothing merged since #{number}"),
        ([], None) => "Nothing merged yet".to_string(),
        ([one], _) => format!("Pull request #{one}, and the sections its Job wrote"),
        (many, _) => format!("Pull requests {}, and the sections their Jobs wrote", named(many)),
    }
}

/// `#1, #2 and #3`, or the first few and how many more.
fn named(numbers: &[u64]) -> String {
    let shown: Vec<String> = numbers.iter().take(MOST_NAMED).map(|number| format!("#{number}")).collect();
    let more = numbers.len().saturating_sub(MOST_NAMED);
    if more > 0 {
        return format!("{} and {more} more", shown.join(", "));
    }
    match shown.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, before)) => format!("{} and {last}", before.join(", ")),
        None => String::new(),
    }
}

fn model_detail(share: u32, spent: u32) -> String {
    match share.saturating_sub(spent) {
        0 => format!("Skipped: {}", share_spent(share)),
        left => format!("One model call, skipped if nothing is new. {left} of {share} calls left tonight"),
    }
}

/// Why a model step is skipped for the share, in lower case.
fn share_spent(share: u32) -> String {
    match share {
        0 => "the share allows no calls".to_string(),
        1 => "tonight's 1 call is spent".to_string(),
        n => format!("tonight's {n} calls are spent"),
    }
}

/// [`share_spent`] as a step's line.
pub(super) fn share_spent_said(share: u32) -> String {
    capitalised(&share_spent(share))
}

/// What the first step read: `3 merges, 5 sections`.
pub(super) fn merges_said(merges: usize, sections: u64) -> String {
    capitalised(&format!("{}, {}", counted(merges as u64, "merge"), counted(sections, "section")))
}

/// What the paragraph step wrote, and the calls tonight once it was asked.
pub(super) fn wrote_said(paragraph: &str, used: u32, share: u32) -> String {
    format!("Wrote {}, {used} of {share} calls tonight", counted(sentences(paragraph), "sentence"))
}

/// The paragraph step asked and had nothing readable back.
pub(super) fn failed_said(used: u32, share: u32) -> String {
    format!("The model wrote nothing, {used} of {share} calls tonight")
}

/// The last step, done.
pub(super) fn through_said(newest: Option<u64>) -> String {
    newest.map_or_else(|| "Nothing merged to read through".to_string(), |number| format!("Read through #{number}"))
}

fn counted(n: u64, word: &str) -> String {
    match n {
        0 => format!("no {word}s"),
        1 => format!("1 {word}"),
        n => format!("{n} {word}s"),
    }
}

fn sentences(paragraph: &str) -> u64 {
    let ends = paragraph.char_indices().filter(|(at, c)| matches!(c, '.' | '!' | '?') && paragraph[at + 1..].chars().next().is_none_or(char::is_whitespace)).count();
    ends.max(usize::from(!paragraph.trim().is_empty())) as u64
}

fn capitalised(line: &str) -> String {
    let mut chars = line.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// The projects whose librarian is running now: each run as told so far, and whether Stop was pressed on it. In memory.
#[derive(Default)]
pub(crate) struct Running(Mutex<BTreeMap<String, Going>>);

struct Going {
    stop: bool,
    now: LibrarianRunning,
}

impl Running {
    /// Hold `project` for the run `run`, or `None` where one is running.
    pub(super) fn begin(&self, project: &str, run: &Instant) -> Option<Held<'_>> {
        let mut running = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if running.contains_key(project) {
            return None;
        }
        running.insert(project.to_string(), Going { stop: false, now: LibrarianRunning { run: run.clone(), steps: Vec::new() } });
        Some(Held { running: self, project: project.to_string() })
    }

    /// Stop was pressed on `project`'s run. Whether one was running.
    pub(super) fn stop(&self, project: &str) -> bool {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).get_mut(project).map(|going| going.stop = true).is_some()
    }

    /// `project`'s run going now, as told so far.
    pub(super) fn now(&self, project: &str) -> Option<LibrarianRunning> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).get(project).map(|going| going.now.clone())
    }
}

/// One project's run, held until it is dropped.
pub(super) struct Held<'a> {
    running: &'a Running,
    project: String,
}

impl Held<'_> {
    pub(super) fn stop_asked(&self) -> bool {
        self.running.0.lock().unwrap_or_else(PoisonError::into_inner).get(&self.project).is_some_and(|going| going.stop)
    }

    /// A step moved: it replaces what was held of that step, or joins the end.
    pub(super) fn moved(&self, step: &str, state: LibrarianStepState, said: &str) {
        let mut running = self.running.0.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(going) = running.get_mut(&self.project) else { return };
        let now = LibrarianStepNow { step: step.to_string(), state, said: said.to_string() };
        match going.now.steps.iter_mut().find(|held| held.step == step) {
            Some(held) => *held = now,
            None => going.now.steps.push(now),
        }
    }
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        self.running.0.lock().unwrap_or_else(PoisonError::into_inner).remove(&self.project);
    }
}
