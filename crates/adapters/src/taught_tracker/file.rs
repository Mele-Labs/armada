//! `tracker.toml`: how a tracker mod lists, searches and reads, read and checked without running
//! anything. `docs/concepts/mods.md`, *tracker.toml*.
//!
//! **Read as a table and walked by hand**, so every problem is said at once and in a sentence,
//! rather than serde's first. Each call section names its `transport`, and the keys it takes are
//! that transport's, so a third transport is another arm and not a new shape.

use adapter_traits::QueryGuide;
use toml::{Table, Value};

/// The most `tracker.toml` may weigh.
pub const MOST_BYTES: u64 = 32 * 1024;

const SECTIONS: &[&str] = &["guide", "credential", "search", "list", "read"];
const GUIDE_KEYS: &[&str] = &["language", "guide", "open", "assigned_to_me", "fields"];
const TRANSPORTS: &str = "cli, http";
const CLI_KEYS: &[&str] = &["transport", "argv", "map"];
const HTTP_KEYS: &[&str] = &["transport", "method", "url", "headers", "body", "map"];

/// The placeholders a call may hold. Any other `{word}` is text, so a body's own braces stand.
pub(crate) const QUERY: &str = "query";
pub(crate) const REFERENCE: &str = "reference";
pub(crate) const CREDENTIAL: &str = "credential";
const PLACEHOLDERS: &[&str] = &[QUERY, REFERENCE, CREDENTIAL];

/// A tracker taught by a mod, as its file says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackerFile {
    pub(crate) guide: QueryGuide,
    /// A command whose standard output is the secret, run for each call and never kept.
    pub(crate) credential: Option<Vec<String>>,
    pub(crate) search: Call,
    /// Absent is a search for the guide's `open`.
    pub(crate) list: Option<Call>,
    pub(crate) read: Call,
}

/// One way of asking, and the `jq` filter that turns its answer into Armada's issues.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Call {
    pub(crate) transport: Transport,
    pub(crate) map: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Transport {
    /// A program and its arguments, run with no shell.
    Cli { argv: Vec<String> },
    Http { method: String, url: String, headers: Vec<(String, String)>, body: Option<String> },
}

impl TrackerFile {
    /// What [`Tracker::query_guide`](adapter_traits::Tracker::query_guide) answers.
    pub fn guide(&self) -> &QueryGuide {
        &self.guide
    }

    /// Whether the open issues are listed by a call of their own, and not by a search for `open`.
    pub fn lists_apart(&self) -> bool {
        self.list.is_some()
    }
}

/// Everything wrong with `text`, none where it reads.
pub fn problems(text: &str) -> Vec<String> {
    read(text).err().unwrap_or_default()
}

/// `text` as a tracker, or every problem with it.
pub fn read(text: &str) -> Result<TrackerFile, Vec<String>> {
    let table: Table = toml::from_str(text).map_err(|why| vec![format!("tracker.toml is not TOML: {}", why.message())])?;
    let mut problems = Vec::new();
    for key in table.keys().filter(|key| !SECTIONS.contains(&key.as_str())) {
        problems.push(format!("`[{key}]` is not a section a tracker reads: {}", SECTIONS.join(", ")));
    }
    let guide = section(&table, "guide", &mut problems).map(|it| guide_of(it, &mut problems));
    let credential = match table.get("credential") {
        None => None,
        Some(Value::Table(it)) => credential_of(it, &mut problems),
        Some(_) => {
            problems.push("`credential` must be a section, `[credential]`".to_string());
            None
        }
    };
    let has_credential = credential.is_some();
    let search = section(&table, "search", &mut problems)
        .and_then(|it| call_of("search", it, &[QUERY], has_credential, &mut problems));
    let list = match table.get("list") {
        None => None,
        Some(Value::Table(it)) => call_of("list", it, &[], has_credential, &mut problems),
        Some(_) => {
            problems.push("`list` must be a section, `[list]`".to_string());
            None
        }
    };
    let read = section(&table, "read", &mut problems)
        .and_then(|it| call_of("read", it, &[REFERENCE], has_credential, &mut problems));
    match (problems.is_empty(), guide, search, read) {
        (true, Some(guide), Some(search), Some(read)) => Ok(TrackerFile { guide, credential, search, list, read }),
        _ => Err(problems),
    }
}

fn section<'a>(table: &'a Table, name: &str, problems: &mut Vec<String>) -> Option<&'a Table> {
    match table.get(name) {
        Some(Value::Table(it)) => Some(it),
        Some(_) => {
            problems.push(format!("`{name}` must be a section, `[{name}]`"));
            None
        }
        None => {
            problems.push(format!("tracker.toml has no `[{name}]`"));
            None
        }
    }
}

fn guide_of(table: &Table, problems: &mut Vec<String>) -> QueryGuide {
    unknown_keys("guide", table, GUIDE_KEYS, "a guide", problems);
    let mut text = |key: &str| text_of("guide", table, key, problems).unwrap_or_default();
    let (language, guide, open, assigned_to_me) = (text("language"), text("guide"), text("open"), text("assigned_to_me"));
    let fields = match table.get("fields") {
        None => Vec::new(),
        Some(value) => strings_of("guide", "fields", value, problems).unwrap_or_default(),
    };
    QueryGuide { language, guide, open, assigned_to_me, fields }
}

fn credential_of(table: &Table, problems: &mut Vec<String>) -> Option<Vec<String>> {
    unknown_keys("credential", table, &["argv"], "a credential", problems);
    let argv = argv_of("credential", table, problems)?;
    if let Some(used) = argv.iter().find_map(|one| placeholders_in(one).into_iter().next()) {
        problems.push(format!("`[credential]` uses `{{{used}}}`, which nothing fills before the credential is read"));
        return None;
    }
    Some(argv)
}

/// One call section. `fills` is what the section's own act supplies, beside `{credential}`, and
/// each must be used, or every call would ask the same.
fn call_of(name: &str, table: &Table, fills: &[&str], has_credential: bool, problems: &mut Vec<String>) -> Option<Call> {
    let before = problems.len();
    let map = match table.get("map") {
        Some(Value::String(map)) if !map.trim().is_empty() => Some(map.clone()),
        Some(Value::String(_)) | None => {
            problems.push(format!("`[{name}]` has no `map`: the jq filter that turns its answer into issues"));
            None
        }
        Some(_) => {
            problems.push(format!("`[{name}]` `map` must be a string"));
            None
        }
    };
    let transport = match table.get("transport") {
        Some(Value::String(word)) => match word.as_str() {
            "cli" => {
                unknown_keys(name, table, CLI_KEYS, "a cli call", problems);
                argv_of(name, table, problems).map(|argv| Transport::Cli { argv })
            }
            "http" => {
                unknown_keys(name, table, HTTP_KEYS, "an http call", problems);
                http_of(name, table, problems)
            }
            other => {
                problems.push(format!("`[{name}]` transport `{other}` is not one this build has: {TRANSPORTS}"));
                None
            }
        },
        Some(_) => {
            problems.push(format!("`[{name}]` `transport` must be a string: {TRANSPORTS}"));
            None
        }
        None => {
            problems.push(format!("`[{name}]` has no `transport`: {TRANSPORTS}"));
            None
        }
    };
    let transport = transport?;
    let mut used: Vec<String> = texts_of(&transport).iter().flat_map(|one| placeholders_in(one)).collect();
    used.sort();
    used.dedup();
    for word in &used {
        let fillable = fills.contains(&word.as_str()) || (word == CREDENTIAL && has_credential);
        if !fillable {
            let why = match word.as_str() {
                CREDENTIAL => "and there is no `[credential]` to fill it".to_string(),
                _ => format!("which only `[{}]` can fill", if word == QUERY { "search" } else { "read" }),
            };
            problems.push(format!("`[{name}]` uses `{{{word}}}`, {why}"));
        }
    }
    for needed in fills {
        if !used.iter().any(|one| one == needed) {
            problems.push(format!("`[{name}]` never uses `{{{needed}}}`, so every call would ask the same"));
        }
    }
    if let Transport::Cli { argv } = &transport {
        if argv.first().is_some_and(|program| !placeholders_in(program).is_empty()) {
            problems.push(format!("`[{name}]` names its program with a placeholder; the program is written as it is"));
        }
    }
    (problems.len() == before).then_some(Call { transport, map: map? })
}

fn http_of(name: &str, table: &Table, problems: &mut Vec<String>) -> Option<Transport> {
    let method = match table.get("method") {
        None => "GET".to_string(),
        Some(Value::String(method)) if ["GET", "POST"].contains(&method.to_ascii_uppercase().as_str()) => {
            method.to_ascii_uppercase()
        }
        Some(_) => {
            problems.push(format!("`[{name}]` `method` is `GET` or `POST`: a tracker is only read"));
            return None;
        }
    };
    let url = text_of(name, table, "url", problems)?;
    if let Some(why) = url_problem(&url) {
        problems.push(format!("`[{name}]` `url` {why}"));
        return None;
    }
    let mut headers = Vec::new();
    match table.get("headers") {
        None => {}
        Some(Value::Table(named)) => {
            for (header, value) in named {
                match value {
                    Value::String(value) if placeholders_in(header).is_empty() => headers.push((header.clone(), value.clone())),
                    Value::String(_) => problems.push(format!("`[{name}]` header `{header}` holds a placeholder in its name")),
                    _ => problems.push(format!("`[{name}]` header `{header}` must be a string")),
                }
            }
        }
        Some(_) => problems.push(format!("`[{name}]` `headers` must be a table of strings")),
    }
    let body = match table.get("body") {
        None => None,
        Some(Value::String(body)) if method == "POST" => Some(body.clone()),
        Some(Value::String(_)) => {
            problems.push(format!("`[{name}]` has a `body` and is not a `POST`"));
            None
        }
        Some(_) => {
            problems.push(format!("`[{name}]` `body` must be a string"));
            None
        }
    };
    Some(Transport::Http { method, url, headers, body })
}

/// Why `url` may not be asked, or `None`. **The host is written out and never filled**, so a
/// credential goes to the host named and nowhere else, and only over TLS off this machine.
fn url_problem(url: &str) -> Option<String> {
    let Some((scheme, rest)) = url.split_once("://") else {
        return Some("must start with `https://`".to_string());
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let loopback = ["localhost", "127.0.0.1", "[::1]"]
        .iter()
        .any(|one| host == *one || host.starts_with(&format!("{one}:")));
    if host.is_empty() || host.contains(['{', '}', '@']) {
        return Some("must name its host as written, with no placeholder or sign-in in it".to_string());
    }
    match (scheme, loopback) {
        ("https", _) | ("http", true) => None,
        _ => Some("must be `https://`, or `http://` to this machine".to_string()),
    }
}

fn unknown_keys(name: &str, table: &Table, known: &[&str], what: &str, problems: &mut Vec<String>) {
    for key in table.keys().filter(|key| !known.contains(&key.as_str())) {
        problems.push(format!("`[{name}]` has `{key}`, which {what} does not take: {}", known.join(", ")));
    }
}

fn text_of(name: &str, table: &Table, key: &str, problems: &mut Vec<String>) -> Option<String> {
    match table.get(key) {
        Some(Value::String(text)) => Some(text.clone()),
        Some(_) => {
            problems.push(format!("`[{name}]` `{key}` must be a string"));
            None
        }
        None => {
            problems.push(format!("`[{name}]` has no `{key}`"));
            None
        }
    }
}

fn argv_of(name: &str, table: &Table, problems: &mut Vec<String>) -> Option<Vec<String>> {
    let Some(value) = table.get("argv") else {
        problems.push(format!("`[{name}]` has no `argv`: the program and its arguments"));
        return None;
    };
    let argv = strings_of(name, "argv", value, problems)?;
    if argv.first().is_none_or(|program| program.trim().is_empty()) {
        problems.push(format!("`[{name}]` `argv` must start with the program to run"));
        return None;
    }
    Some(argv)
}

fn strings_of(name: &str, key: &str, value: &Value, problems: &mut Vec<String>) -> Option<Vec<String>> {
    let listed = value.as_array().and_then(|all| all.iter().map(|one| one.as_str().map(str::to_string)).collect());
    if listed.is_none() {
        problems.push(format!("`[{name}]` `{key}` must be a list of strings"));
    }
    listed
}

/// Every text a call fills a placeholder into.
pub(crate) fn texts_of(transport: &Transport) -> Vec<&str> {
    match transport {
        Transport::Cli { argv } => argv.iter().map(String::as_str).collect(),
        Transport::Http { url, headers, body, .. } => {
            let mut all = vec![url.as_str()];
            all.extend(headers.iter().map(|(_, value)| value.as_str()));
            all.extend(body.as_deref());
            all
        }
    }
}

/// The placeholders `text` holds, each `{word}` whose word is one of [`PLACEHOLDERS`].
pub(crate) fn placeholders_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('{') {
        rest = &rest[at + 1..];
        let word: String = rest.chars().take_while(|c| c.is_ascii_lowercase() || *c == '_').collect();
        if rest[word.len()..].starts_with('}') && PLACEHOLDERS.contains(&word.as_str()) {
            found.push(word);
        }
    }
    found
}
