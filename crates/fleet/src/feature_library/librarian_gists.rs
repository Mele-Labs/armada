//! The librarian's `gists` part: a one-line gist for a linked doc that has none. `docs/concepts/feature-catalog.md`,
//! *Librarian*.
//!
//! **The model only fills gaps.** A Job's own words and the doc's first sentence say it for free, so a doc is sent only
//! where no read link of it says anything, and the doc is a file of the project's repository. Up to ten docs go in one
//! call, the oldest links first, thirty docs a run, and each call is spent from the night's share
//! (`features.librarianModelShare`). The small model that sorts his messages (`models.dispute`) answers one line a number.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{feature_link_kind, Store, WriteError};

use super::consulting::{cut, GIST_MOST, READ};
use super::doc_drift::{git, inside};
use crate::daemon::Fleet;

/// The most docs gisted in one run.
pub(super) const MOST_DOCS: usize = 30;
/// The most docs one call gists, which is what makes a night's share go further.
const PER_CALL: usize = 10;
/// How much of a doc's opening the prompt carries, in bytes.
const OPENING: u64 = 4000;

/// A doc the model is asked about: its address and the start of its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Opening {
    pub(super) target: String,
    pub(super) text: String,
}

/// What the part did in one run.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Gisted {
    /// The linked docs that lacked a gist, files or not.
    pub(super) lacking: usize,
    /// The docs that lacked a gist and could be read.
    pub(super) found: usize,
    /// The docs a gist was kept for.
    pub(super) wrote: usize,
    /// The calls made, failed ones too.
    pub(super) calls: u32,
    /// The docs left for want of a call in the night's share.
    pub(super) left: usize,
}

/// The linked docs with no gist, oldest link first, each once: addresses are not here, and a doc is left out where any link
/// of it, read or filed any other way, says something.
pub(super) fn lacking(store: &Store, project: &str) -> Result<Vec<String>, WriteError> {
    let mut order: Vec<String> = Vec::new();
    let mut state: BTreeMap<String, (bool, bool)> = BTreeMap::new();
    // Newest first, so the last one seen of a target is its oldest.
    let links = store.project_links(project, feature_link_kind::DOC)?;
    for link in links.iter().rev() {
        if link.target.contains("://") {
            continue;
        }
        let held = state.entry(link.target.clone()).or_insert_with(|| {
            order.push(link.target.clone());
            (false, false)
        });
        held.0 |= link.via == READ;
        held.1 |= !link.gist.is_empty() || link.gist_by == store::feature_gist_by::JOB;
    }
    Ok(order.into_iter().filter(|target| state.get(target).is_some_and(|(read, said)| *read && !said)).collect())
}

/// The start of each of `targets` that is a file of a repository in `roots` at `HEAD` and can be read, at most
/// [`MOST_DOCS`]. **Blocking**: the caller runs it off the main thread.
pub(super) fn openings(roots: &[PathBuf], targets: &[String]) -> Vec<Opening> {
    let mut found = Vec::new();
    for target in targets {
        if found.len() >= MOST_DOCS {
            break;
        }
        let text = roots.iter().find_map(|root| {
            let path = inside(root, target)?;
            git(root, &["cat-file".to_string(), "-e".to_string(), format!("HEAD:{path}")], &[0])?;
            let mut bytes = Vec::new();
            std::fs::File::open(root.join(&path)).ok()?.take(OPENING).read_to_end(&mut bytes).ok()?;
            let text = String::from_utf8_lossy(&bytes);
            let text = text.trim_end_matches('\u{fffd}').trim().to_string();
            (!text.is_empty()).then_some(text)
        });
        if let Some(text) = text {
            found.push(Opening { target: target.clone(), text });
        }
    }
    found
}

/// What the model is asked: the rules, then each doc as a numbered item with its path and its opening text.
pub(super) fn question(batch: &[Opening]) -> String {
    let docs: Vec<String> = batch.iter().enumerate().map(|(at, one)| format!("{}. Path: {}\nOpening text:\n{}", at + 1, one.target, one.text)).collect();
    format!(
        "Below are documents from a software project, each numbered with its path and its opening text. Write one gist for each.\n\n\
         Rules:\n\
         - One line a number, as `1. the gist`, and nothing else.\n\
         - Each gist is a plain fact of at most 20 words about what the document covers.\n\
         - No advice, no opinion, no greeting.\n\
         - No dashes of any kind.\n\n\
         Documents:\n\n{}\n\n\
         Answer with the numbered lines and nothing else.",
        docs.join("\n\n")
    )
}

/// The lines of `reply` by the number they open with, 1 to `count`: one gist kept, one line, no dashes, cut to 200
/// characters. A number answered twice keeps the first, and a line with no number or no words is nothing.
pub(super) fn gists_of(reply: &str, count: usize) -> BTreeMap<usize, String> {
    let mut gists = BTreeMap::new();
    for line in reply.lines() {
        let line = line.trim().trim_start_matches(['*', '-', '#']).trim_start();
        let digits = line.chars().take_while(char::is_ascii_digit).count();
        let (number, rest) = line.split_at(digits);
        let Some(rest) = rest.strip_prefix(['.', ':', ')']) else { continue };
        let Ok(number) = number.parse::<usize>() else { continue };
        let flat = rest.split_whitespace().collect::<Vec<_>>().join(" ");
        let flat = flat.trim_matches(|c: char| c == '"' || c == '\'' || c == '`').replace(" - ", ", ").replace(['\u{2014}', '\u{2013}'], ",").replace(" ,", ",");
        let gist = cut(&flat, GIST_MOST);
        if (1..=count).contains(&number) && !gist.is_empty() {
            gists.entry(number).or_insert(gist);
        }
    }
    gists
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
    /// The part for `project`: each batch of docs that lack a gist asked once while `calls_left` allows a call. A doc
    /// the model gave no line for is left as it was.
    pub(super) async fn gist_docs(&self, project: &str, calls_left: u32) -> Result<Gisted, WriteError> {
        let targets = lacking(&*self.store().lock().await, project)?;
        let lacking = targets.len();
        let roots = self.seed_roots(project).unwrap_or_default();
        let found = tokio::task::spawn_blocking(move || openings(&roots, &targets)).await.unwrap_or_default();
        let mut done = Gisted { lacking, found: found.len(), ..Gisted::default() };
        for batch in found.chunks(PER_CALL) {
            if done.calls >= calls_left {
                done.left += batch.len();
                continue;
            }
            done.calls += 1;
            let reply = match self.story_asked(&question(batch)).await {
                Ok(reply) => reply,
                Err(why) => {
                    eprintln!("fleet: the librarian's gists for project {project} were not written: {why}");
                    continue;
                }
            };
            let mut store = self.store().lock().await;
            for (number, gist) in gists_of(&reply, batch.len()) {
                if store.keep_feature_librarian_gist(project, &batch[number - 1].target, &gist)? > 0 {
                    done.wrote += 1;
                }
            }
        }
        Ok(done)
    }
}
