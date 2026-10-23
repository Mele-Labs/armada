//! The Logbook note every new Session's process is started with: the habits first, then what the
//! repository's Sessions learnt, then this Mac. `docs/concepts/logbook.md`, *Recall*.

use store::LogSection;

use super::serving::Logged;

/// What the note holds, in tokens, header included. What the cap leaves out is found with `recall`.
pub(crate) const NOTE_TOKENS: usize = 2000;

/// What the header costs, in tokens, taken off the entries' share.
pub(crate) const HEADER_TOKENS: usize = 60;

const HEADER: &str = "The Logbook: what earlier Jobs learnt, kept for you. Each line stands. \
                      The `recall` tool searches what this leaves out.";

const SUBAGENT_HEADER: &str = "The Logbook, for where you work. Each line stands. The `recall` tool searches the rest.";

/// What the subagent header costs, in tokens.
pub(crate) const SUBAGENT_HEADER_TOKENS: usize = 30;

/// The sections as the note takes them, with the words that open each.
const SECTIONS: [(LogSection, &str); 3] = [
    (LogSection::Habits, "How the owner works:"),
    (LogSection::Repo, "How this repository runs, from what its Jobs learnt. A command here is to run, never a Check:"),
    (LogSection::Machine, "What this Mac has:"),
];

/// The note for the entries served, kept in the order they came within each section. `None` where
/// nothing was served.
pub(crate) fn note(served: &[Logged]) -> Option<String> {
    rendered(HEADER, served)
}

/// What a subagent is handed: the same lines under a shorter header, with no section of this Mac's.
pub(crate) fn subagent_note(served: &[Logged]) -> Option<String> {
    rendered(SUBAGENT_HEADER, served)
}

fn rendered(header: &str, served: &[Logged]) -> Option<String> {
    let mut parts = vec![header.to_string()];
    for (section, opening) in SECTIONS {
        let lines: Vec<String> =
            served.iter().filter(|one| one.entry.section == section).map(|one| format!("- {}", one.entry.text)).collect();
        if !lines.is_empty() {
            parts.push(format!("{opening}\n{}", lines.join("\n")));
        }
    }
    (parts.len() > 1).then(|| parts.join("\n\n"))
}
