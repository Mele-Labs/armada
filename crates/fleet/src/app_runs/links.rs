//! A link: one process of a run expecting another on a port it does not get. A frontend whose dev
//! server proxies `/api` to `http://127.0.0.1:8731` breaks once the backend is leased another
//! number, and Armada has to learn that to mend it. `docs/concepts/fleet.md`, *Servers*.
//!
//! | Module | Holds |
//! |---|---|
//! | [`scan`] | What a folder's config files say about ports |
//! | [`notice`] | A refused connection read from a line of output |
//! | [`keeping`] | What is remembered, and the Fleet half that reads, finds and tells |
//!
//! **Found two ways.** Drafting and listing read each process's folder: a `localhost` port there
//! that another process is known to listen on. A run in progress finds the rest, in the output of a
//! process refused a connection to a port a sibling is known on. **Known** is a number pinned, a
//! literal in the process's own command or config, or the port a Job's dev server came up on.
//!
//! **One link per pair**, and a found link is kept in run memory as a fact of its own kind, so the
//! next draft and start show it without finding it again. A fact is never deleted, because a place
//! that keeps a file merges it back: a link that is gone is rewritten with `succeeded` false, newer
//! than the one it ends.

pub(crate) mod keeping;
pub(crate) mod notice;
mod scan;

use std::path::Path;

use store::KeptRun;

/// The kind of the run-memory fact a link is kept as.
pub(crate) const KIND: &str = "port_link";

/// What `found_in` says for a link read from a process's output rather than a file.
pub(crate) const LOG: &str = "log";

/// One process expecting another on a port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Link {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) port: u16,
    /// The file it was read in, relative to the repository's root, or [`LOG`].
    pub(crate) found_in: String,
    /// The folder `from` runs in: the fact's workspace. `.` is the root.
    pub(crate) workspace: String,
}

/// A process as the scanner reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub(crate) name: String,
    pub(crate) dir: Option<String>,
    /// What starts it, as written: `${port.NAME}` is a leased port and names no number.
    pub(crate) command: String,
    /// The number it is pinned to.
    pub(crate) pinned: Option<u16>,
}

impl Candidate {
    fn folder(&self) -> String {
        self.dir.clone().unwrap_or_else(|| String::from("."))
    }
}

/// Every number each candidate is known to listen on, in the order `candidates` are in: pinned,
/// written in its command or its folder's config, or where a Job's dev server in that folder came up.
pub(crate) fn expects(root: &Path, candidates: &[Candidate], facts: &[KeptRun]) -> Vec<Vec<u16>> {
    candidates
        .iter()
        .map(|one| {
            let mut ports: Vec<u16> = one.pinned.into_iter().collect();
            ports.extend(scan::listens_in(&one.command));
            for config in scan::configs_in(&root.join(one.folder())) {
                ports.extend(scan::listens_in(&config.text));
            }
            let folder = one.folder();
            ports.extend(
                facts
                    .iter()
                    .filter(|fact| fact.kind == "dev_server" && fact.succeeded)
                    .filter(|fact| super::draft::in_the_repository(&fact.workspace).is_some_and(|at| at == folder))
                    .filter_map(|fact| fact.port),
            );
            ports.sort_unstable();
            ports.dedup();
            ports
        })
        .collect()
}

/// The links the folders of `candidates` hold, **at most one for a pair**. A candidate's own
/// number is never a link to itself.
pub(crate) fn found(root: &Path, candidates: &[Candidate], facts: &[KeptRun]) -> Vec<Link> {
    let known = expects(root, candidates, facts);
    let mut out: Vec<Link> = Vec::new();
    for (from, one) in candidates.iter().enumerate() {
        let folder = one.folder();
        for config in scan::configs_in(&root.join(&folder)) {
            for port in scan::expected_in(&config.text) {
                if known[from].contains(&port) {
                    continue;
                }
                let Some(to) = (0..candidates.len()).find(|to| *to != from && known[*to].contains(&port)) else {
                    continue;
                };
                let pair = (&one.name, &candidates[to].name);
                if out.iter().any(|link| (&link.from, &link.to) == pair) {
                    continue;
                }
                out.push(Link {
                    from: one.name.clone(),
                    to: candidates[to].name.clone(),
                    port,
                    found_in: if folder == "." { config.name.clone() } else { format!("{folder}/{}", config.name) },
                    workspace: folder.clone(),
                });
            }
        }
    }
    out
}

/// Which links to write and which to end, once `found` is read for the processes in `scanned`.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Changes {
    pub(crate) add: Vec<Link>,
    pub(crate) end: Vec<Link>,
}

/// `found` against what is `kept`. **A link from a file ends when the file no longer holds it**,
/// but only where `ending`: a bay that fixed its copy has not fixed the main checkout's, and a
/// memory shared by both must not flip as each is listed. A link read from output ends when a file
/// finds the same pair, since the file says where.
pub(crate) fn reconciled(kept: &[Link], found: &[Link], scanned: &[String], ending: bool) -> Changes {
    let mut changes = Changes::default();
    for link in found {
        if !kept.contains(link) {
            changes.add.push(link.clone());
        }
    }
    for link in kept {
        let same_pair = |other: &&Link| other.from == link.from && other.to == link.to;
        let replaced = link.found_in == LOG && found.iter().any(|other| same_pair(&other));
        let gone = ending && link.found_in != LOG && scanned.contains(&link.from) && !found.contains(link);
        if replaced || gone {
            changes.end.push(link.clone());
        }
    }
    changes
}

/// Every link in `facts` that is still live.
pub(crate) fn remembered(facts: &[KeptRun]) -> Vec<Link> {
    facts.iter().filter(|fact| fact.kind == KIND && fact.succeeded).filter_map(read).collect()
}

fn read(fact: &KeptRun) -> Option<Link> {
    let (from, rest) = fact.command.split_once(" -> ")?;
    let (to, found_in) = rest.split_once(" in ")?;
    Some(Link {
        from: from.to_string(),
        to: to.to_string(),
        port: fact.port?,
        found_in: found_in.to_string(),
        workspace: fact.workspace.clone(),
    })
}

/// `link` as a fact: live, or ended where `at` is a time after the fact it ends.
pub(crate) fn as_fact(link: &Link, live: bool, at: u64) -> KeptRun {
    KeptRun {
        workspace: link.workspace.clone(),
        kind: KIND.to_string(),
        command: format!("{} -> {} in {}", link.from, link.to, link.found_in),
        port: Some(link.port),
        succeeded: live,
        last_ran: at,
        source: String::from("links"),
        kept_in: None,
        why: None,
    }
}

/// What to hand a Job to mend `link`: Bridge opens Dispatch with it and Fleet starts nothing.
/// `to_port` is the name of the port `to` listens on, which is what `ARMADA_PORT_*` is named for.
pub(crate) fn brief(link: &Link, to_port: &str) -> String {
    let env = crate::ports::armada_port_env_name(to_port);
    let (from, to, port) = (&link.from, &link.to, link.port);
    match link.found_in == LOG {
        true => format!(
            "Make `{from}` read `{to}`'s port from `{env}`, falling back to `{port}`. Its output showed a refused connection to port {port}."
        ),
        false => format!(
            "Make `{from}` read `{to}`'s port from `{env}`, falling back to `{port}`, in `{}`.",
            link.found_in
        ),
    }
}
