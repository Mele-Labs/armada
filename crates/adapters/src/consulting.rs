//! Which of a Session's reads are a document or a ticket, as the feature catalog files them: a page, an MCP server's
//! answer, an issue, or a doc in the repository. `docs/concepts/feature-catalog.md`, *Docs and tickets a Job read*.
//!
//! **Read from the call and never from the agent's word.** The mod hears the call and its result; this says only what
//! kind of read it was, so a call that is not one is not filed. Code is not a doc: a source file, a search and a server
//! that stays on this machine are left out.

use crate::outside::{server_of, FORGE_SERVER, STAYS_HERE, TRACKERS};

/// What a read is, as `feature_links.kind` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsultedKind {
    Doc,
    Ticket,
}

/// A read worth filing: its kind and the address, path or id it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Consulted {
    pub kind: ConsultedKind,
    pub target: String,
}

/// Folders whose `.md` files are source, not documents. A `docs/` folder wins over this wherever it stands first.
const SOURCE: [&str; 11] = ["crates", "src", "packages", "apps", "plugins", "xtask", "tests", "scripts", "node_modules", ".claude", ".armada"];

/// Files every agent is handed already, which a read of is not a read for the work.
const INSTRUCTIONS: [&str; 2] = ["CLAUDE.md", "AGENTS.md"];

/// Words of a tool's name that say it reads, and that say it changes something. A name with both changes.
const READS: [&str; 6] = ["get", "fetch", "read", "view", "retrieve", "download"];
const CHANGES: [&str; 12] = ["create", "update", "delete", "add", "edit", "set", "write", "post", "comment", "send", "move", "remove"];

/// The read a tool call was, from the tool's name and the address, path or id the call named. `None` for any other.
pub fn consulted(tool: &str, target: &str) -> Option<Consulted> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }
    let kind = match tool {
        "WebFetch" => match is_address(target) {
            true => Some(if names_an_issue(target) { ConsultedKind::Ticket } else { ConsultedKind::Doc }),
            false => None,
        },
        "Read" => is_a_doc(target).then_some(ConsultedKind::Doc),
        // The mod names the ticket a `gh issue view` or `bd show` opened.
        "Bash" => Some(ConsultedKind::Ticket),
        _ => server_read(tool),
    }?;
    Some(Consulted { kind, target: target.to_string() })
}

fn is_address(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

/// An address of an issue as a tracker serves it: `/issues/123` or `/browse/KEY-123`.
fn names_an_issue(address: &str) -> bool {
    let path = address.split(['?', '#']).next().unwrap_or(address);
    path.contains("/issues/") || path.contains("/browse/")
}

/// A file of the repository that is a document: anything under `docs/`, or a markdown file in no source folder.
fn is_a_doc(path: &str) -> bool {
    let path = path.trim_start_matches("./");
    if path.starts_with('/') || path.split('/').any(|part| part == "..") {
        return false;
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    if INSTRUCTIONS.contains(&name) {
        return false;
    }
    let lower = name.to_lowercase();
    if path.starts_with("docs/") {
        return [".md", ".mdx", ".html", ".txt", ".rst"].iter().any(|end| lower.ends_with(end));
    }
    let markdown = lower.ends_with(".md") || lower.ends_with(".mdx");
    markdown && !path.split('/').any(|part| SOURCE.contains(&part))
}

/// A server's read: a tracker's is a ticket, the forge's is a ticket where the call is about an issue, and any other
/// server's is a document. Armada's own servers and the ones that stay on this machine are not reads from outside.
fn server_read(tool: &str) -> Option<ConsultedKind> {
    let (server, name) = tool.strip_prefix("mcp__")?.split_once("__")?;
    let server = server_of(server);
    if server.is_empty() || server.starts_with("armada") || STAYS_HERE.contains(&server.as_str()) {
        return None;
    }
    let words = words_of(name);
    if !words.iter().any(|word| READS.contains(&word.as_str())) || words.iter().any(|word| CHANGES.contains(&word.as_str())) {
        return None;
    }
    if TRACKERS.contains(&server.as_str()) {
        return Some(ConsultedKind::Ticket);
    }
    if server == FORGE_SERVER {
        return words.iter().any(|word| word == "issue").then_some(ConsultedKind::Ticket);
    }
    Some(ConsultedKind::Doc)
}

/// The lowercase words of a tool's name, split at `_`, `-` and a capital: `getJiraIssue` is `get`, `jira`, `issue`.
fn words_of(name: &str) -> Vec<String> {
    let mut words = vec![String::new()];
    for letter in name.chars() {
        if !letter.is_alphanumeric() {
            words.push(String::new());
            continue;
        }
        if letter.is_uppercase() {
            words.push(String::new());
        }
        if let Some(last) = words.last_mut() {
            last.extend(letter.to_lowercase());
        }
    }
    words.retain(|word| !word.is_empty());
    words
}
