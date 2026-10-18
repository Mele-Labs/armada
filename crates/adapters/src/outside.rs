//! Which of a Session's calls leave the repository, and for what.
//!
//! **A call is named by what it asked and never by what it was given.** A `gh` command is its
//! subcommand words and no argument; a page fetched is its host and no path or query; a search is
//! the word `search` and not the query; a server's tool is its own name. The timeline keeps an entry
//! 30 days and anyone who can see the Cockpit reads it, so nothing here depends on a later scrub.
//!
//! **A call that never leaves the machine is not one**: Armada's own servers and the code-graph
//! helper. They are left out, not given a service of their own.
//!
//! The tool names (`WebFetch`, `mcp__server__tool`) are the harness's vocabulary and the shell
//! commands are the forge's, which is why this is read here and Fleet is handed a [`Reach`].

use adapter_traits::Reach;

const FORGE: &str = "forge";
/// The forge's own MCP server, by the name [`server_of`] gives it.
const FORGE_SERVER: &str = "github";
const TRACKER: &str = "tracker";
const WEB: &str = "web";

/// MCP servers that reach an issue tracker, by the name [`server_of`] gives them.
const TRACKERS: [&str; 6] = ["linear", "jira", "atlassian", "asana", "clickup", "monday"];

/// MCP servers whose calls stay on this machine. Armada's own are told by their prefix.
const STAYS_HERE: [&str; 1] = ["gitnexus"];

/// The most subcommand words a `gh` line names: `gh pr view` is a group and a verb.
const SUBCOMMANDS: usize = 2;

fn reach(service: &str, name: &str, line: String) -> Reach {
    Reach { service: service.to_string(), name: name.to_string(), line }
}

/// A shell command that calls the forge's command line, as `gh pr view`. `None` for any other command.
///
/// Only a command whose **first word** is `gh` is read, and its line is `gh` and the subcommand words
/// after it: the words that are plain lowercase, at most [`SUBCOMMANDS`], stopping at the first flag
/// or argument. `gh issue …` is the issue tracker's, every other is the forge's.
pub fn command_reach(command: &str) -> Option<Reach> {
    let mut words = command.split_whitespace();
    if words.next()? != "gh" {
        return None;
    }
    let mut line = vec!["gh"];
    for word in words.take(SUBCOMMANDS) {
        if !is_subcommand(word) {
            break;
        }
        line.push(word);
        // `gh api` takes an endpoint where a verb would be.
        if word == "api" {
            break;
        }
    }
    Some(match line.get(1) {
        Some(&"issue") => reach(TRACKER, "the issue tracker", line.join(" ")),
        _ => reach(FORGE, "GitHub", line.join(" ")),
    })
}

fn is_subcommand(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_lowercase()) && word.chars().all(|c| c.is_ascii_lowercase() || c == '-')
}

/// A tool call that leaves the repository, out of the tool's name and the detail the call carried.
/// `None` for every other tool, and for a server whose calls stay on this machine.
pub fn tool_reach(tool: &str, detail: &str) -> Option<Reach> {
    match tool {
        "WebFetch" => Some(reach(WEB, "the web", host_of(detail)?)),
        "WebSearch" => Some(reach(WEB, "the web", "search".to_string())),
        _ => server_reach(tool),
    }
}

fn server_reach(tool: &str) -> Option<Reach> {
    let (server, name) = tool.strip_prefix("mcp__")?.split_once("__")?;
    let server = server_of(server);
    if name.is_empty() || server.is_empty() || server.starts_with("armada") || STAYS_HERE.contains(&server.as_str()) {
        return None;
    }
    let line = name.to_string();
    Some(if server == FORGE_SERVER {
        reach(FORGE, "GitHub", line)
    } else if TRACKERS.contains(&server.as_str()) {
        reach(TRACKER, "the issue tracker", line)
    } else {
        let name = spoken(&server);
        reach(&server, &name, line)
    })
}

/// A server as the person knows it: lowercase, without the prefix the harness adds for a connector
/// (`claude_ai_Figma`) or for a plugin's (`plugin_productivity_linear`).
fn server_of(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if let Some(rest) = lower.strip_prefix("claude_ai_") {
        return rest.to_string();
    }
    match lower.strip_prefix("plugin_").and_then(|rest| rest.split_once('_')) {
        Some((_, server)) => server.to_string(),
        None => lower,
    }
}

/// A server's name as a row says it: `Figma`, `Claude docs`.
fn spoken(server: &str) -> String {
    let words = server.replace(['_', '-'], " ");
    let mut chars = words.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// The host of an address and nothing after it: no scheme, user, port, path, query or fragment.
/// `None` where there is none to read.
fn host_of(address: &str) -> Option<String> {
    let address = address.trim();
    let rest = address.split_once("://").map_or(address, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    let host = match host.strip_prefix('[') {
        Some(v6) => format!("[{}]", v6.split(']').next()?),
        None => host.split(':').next()?.to_string(),
    };
    let host = host.trim_matches('.').to_lowercase();
    (!host.is_empty() && !host.contains(char::is_whitespace)).then_some(host)
}
