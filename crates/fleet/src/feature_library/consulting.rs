//! The documents and tickets a Job read, filed under the features it then changed. **Nothing here is a Job's word**: the
//! mod hears the call and its result, `adapters::consulted` says whether it was a read of a doc or a ticket, and a read
//! becomes a link only when a write of the same Job reaches a file a feature owns afterwards.
//!
//! What Fleet can see is the order, not whether the Job quoted or acted on what it read. So a read that came before a
//! write to the feature's files is linked, and one that came after, or in another Job, is not. A read is kept the first
//! time and a repeat leaves it where it was, so reading again cannot move it before a write it followed.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::{Consulted, ConsultedKind};
use store::{feature_link_kind, KeptFeatureLink, KeptJobRead, Store, WriteError};

use super::reading::Loaded;
use crate::daemon::Fleet;

/// How the link was found, as `feature_links.via` names it.
pub(super) const READ: &str = "read";

/// The most of a read's title and of its first sentence kept, so the two stay about 300 characters.
const TITLE_MOST: usize = 100;
pub(super) const GIST_MOST: usize = 200;

/// The most reads a write links: the newest, so a Job that read forty pages does not hang forty links on one file.
const MOST_LINKED: usize = 20;

pub(super) fn cut(text: &str, most: usize) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match one_line.char_indices().nth(most) {
        Some((end, _)) => format!("{}…", one_line[..end].trim_end()),
        None => one_line,
    }
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
    /// A read the mod heard: kept where it is a doc or a ticket, and nothing otherwise. **Kept for any Job**, since the
    /// project's library is read when a write links it.
    pub(crate) fn feature_read_kept(&self, store: &mut Store, job: &str, tool: &str, target: &str, title: &str, gist: &str) -> Result<(), WriteError> {
        let Some(Consulted { kind, target }) = adapters::consulted(tool, target) else {
            return Ok(());
        };
        let kind = match kind {
            ConsultedKind::Doc => feature_link_kind::DOC,
            ConsultedKind::Ticket => feature_link_kind::TICKET,
        };
        // Where the result was not visible the read is named by what it was read by.
        let title = Some(cut(title, TITLE_MOST)).filter(|title| !title.is_empty()).unwrap_or_else(|| cut(&target, TITLE_MOST));
        let read = KeptJobRead { job: job.to_string(), kind: kind.to_string(), target, title, gist: cut(gist, GIST_MOST), gist_by: String::new(), at: self.now().as_str().to_string() };
        store.keep_feature_job_read(&read).map(drop)
    }

    /// `job` wrote `paths` in `manifest`: what it has read so far is linked to each feature that owns one. **A read that
    /// came after is not here yet**, which is the whole of the order.
    pub(crate) fn feature_reads_linked(&self, store: &mut Store, job: &str, manifest: &str, paths: &[String]) -> Result<(), WriteError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut reads = store.feature_job_reads(job)?;
        if reads.is_empty() {
            return Ok(());
        }
        reads.drain(..reads.len().saturating_sub(MOST_LINKED));
        for project in self.projects_of_job_in(store, job) {
            if !store.project_has_features(&project)? {
                continue;
            }
            let loaded = Loaded::read(store, &project)?;
            for path in paths {
                for owner in loaded.map.owners(Some(manifest), path) {
                    for read in &reads {
                        store.put_feature_link(&KeptFeatureLink {
                            feature_id: owner.feature.id.clone(),
                            kind: read.kind.clone(),
                            target: read.target.clone(),
                            job: job.to_string(),
                            title: read.title.clone(),
                            state: String::new(),
                            via: READ.to_string(),
                            at: read.at.clone(),
                            gist: read.gist.clone(),
                            gist_by: read.gist_by.clone(),
                        })?;
                    }
                }
            }
        }
        Ok(())
    }
}
