//! A Job's own gist of a doc or ticket it read, written with `write_feature_sections`. **Never required**: `all_done` does
//! not ask for one. A gist applies to the read links this Job made, under any feature its projects hold, and a first
//! sentence heard later does not replace it. `docs/concepts/feature-catalog.md`, *Docs and tickets a Job read*.

use std::collections::BTreeSet;

use ipc::DocGist;
use store::{feature_link_kind, Store, WriteError};

use super::consulting::{cut, GIST_MOST, READ};

pub(super) const BLANK: &str = "fleet.gist_blank";
pub(super) const NOT_READ: &str = "fleet.gist_not_read";

/// The first address in `gists` this Job never read and linked, if any.
pub(super) fn unread<'a>(store: &Store, job: &str, gists: &'a [DocGist]) -> Result<Option<&'a str>, WriteError> {
    if gists.is_empty() {
        return Ok(None);
    }
    let mut read: BTreeSet<String> = BTreeSet::new();
    for kind in [feature_link_kind::DOC, feature_link_kind::TICKET] {
        read.extend(store.feature_links_of_job(job, kind)?.into_iter().filter(|link| link.via == READ).map(|link| link.target));
    }
    Ok(gists.iter().map(|one| one.target.trim()).find(|target| !read.contains(*target)))
}

/// Keep each gist, cut to one line, on the read links of `job`. How many were kept.
pub(super) fn gists_kept(store: &mut Store, job: &str, gists: &[DocGist]) -> Result<usize, WriteError> {
    let mut kept = 0;
    for one in gists {
        if store.keep_feature_job_gist(job, one.target.trim(), &cut(&one.gist, GIST_MOST))? > 0 {
            kept += 1;
        }
    }
    Ok(kept)
}
