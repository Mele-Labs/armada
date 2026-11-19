//! The librarian's `since_you_looked` part: the one paragraph at the top of Since you looked.
//! `docs/concepts/feature-catalog.md`, *Librarian* and *Since you looked*.
//!
//! **Facts in, a paragraph out, and nothing is asked where there are no facts.** The facts are the page's own list, which
//! costs no model: the features new, changed or waiting on him since this part last ran, each with its merged pull request
//! titles. A small model, the one that sorts his messages (`models.dispute`), turns them into at most three short sentences.
//! It is one call, and it is spent from the night's share (`features.librarianModelShare`).

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{LookedFeature, LookedTag};

use crate::daemon::Fleet;

/// The most features the prompt names. The rest are counted, so a first run over a whole catalog stays one short call.
const MOST_FEATURES: usize = 20;
/// The most sentences kept of the answer.
const MOST_SENTENCES: usize = 3;
/// The longest paragraph kept, in characters.
const MOST_CHARS: usize = 600;

/// What the part did in one run.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Since {
    /// Nothing moved since it last ran. No call was made and no paragraph written.
    NoFacts,
    /// There were facts and the night's share had no call left.
    ShareSpent,
    /// One call was made and it answered `paragraph`.
    Wrote(String),
    /// One call was made and it failed or answered nothing readable.
    Failed,
}

fn tag_word(tag: LookedTag) -> &'static str {
    match tag {
        LookedTag::New => "new",
        LookedTag::Changed => "changed",
        LookedTag::NeedsYou => "waiting on the owner",
    }
}

/// What the model is asked: the rules, then each feature on a line.
pub(super) fn question(facts: &[LookedFeature]) -> String {
    let mut lines: Vec<String> = facts.iter().take(MOST_FEATURES).map(|one| format!("- [{}] {}: {}", tag_word(one.tag), one.title, one.what)).collect();
    if facts.len() > MOST_FEATURES {
        lines.push(format!("- and {} more", facts.len() - MOST_FEATURES));
    }
    format!(
        "Below is what moved in a software project's feature catalog since its owner last looked. Write the one paragraph that opens the page.\n\n\
         Rules:\n\
         - At most three short sentences.\n\
         - Facts only: what is new, what changed, what waits on the owner. Use the feature names as written.\n\
         - No advice, no narration, no opinion, no greeting.\n\
         - No dashes of any kind.\n\n\
         Features:\n{}\n\n\
         Answer with the paragraph and nothing else.",
        lines.join("\n")
    )
}

/// The model's answer as the paragraph kept: one line, no dashes, at most three sentences. `None` where nothing is left.
pub(crate) fn paragraph_of(reply: &str) -> Option<String> {
    let flat = reply.split_whitespace().collect::<Vec<_>>().join(" ");
    let flat = flat.trim_matches(|c: char| c == '"' || c == '\'').replace(" - ", ", ").replace(['\u{2014}', '\u{2013}'], ",").replace(" ,", ",");
    let mut kept = String::new();
    let mut sentences = 0;
    let mut rest = flat.as_str();
    while sentences < MOST_SENTENCES && !rest.trim().is_empty() {
        let end = rest.char_indices().find(|(at, c)| matches!(c, '.' | '!' | '?') && rest[at + 1..].chars().next().is_none_or(char::is_whitespace)).map_or(rest.len(), |(at, _)| at + 1);
        kept.push_str(rest[..end].trim_start());
        kept.push(' ');
        rest = &rest[end..];
        sentences += 1;
    }
    let kept: String = kept.trim().chars().take(MOST_CHARS).collect();
    (!kept.is_empty()).then_some(kept)
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
    /// The part for `project`: what moved since `since` (the instant it last ran, none where it never has), asked once
    /// where it did and `calls_left` allows a call.
    pub(super) async fn since_paragraph(&self, project: &str, since: Option<&str>, calls_left: u32) -> Since {
        let facts = match self.since_facts(project, since).await {
            Ok(facts) if !facts.is_empty() => facts,
            _ => return Since::NoFacts,
        };
        if calls_left == 0 {
            return Since::ShareSpent;
        }
        match self.story_asked(&question(&facts)).await {
            Ok(reply) => paragraph_of(&reply).map_or(Since::Failed, Since::Wrote),
            Err(why) => {
                eprintln!("fleet: the librarian's paragraph for project {project} was not written: {why}");
                Since::Failed
            }
        }
    }
}
