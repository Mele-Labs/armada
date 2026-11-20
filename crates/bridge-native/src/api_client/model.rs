//! What the API Client works out from Fleet's reads, with no drawing: which `{{name}}` an address holds and what each fills to
//! in the environment picked, which saved requests an address finds, what the list holds under each filter, a thread row
//! made into a request to start from, and an answer's body set out. Classic's `requests-model.ts`.

use std::ops::Range;

use ipc::{Placed, RequestBy, RequestEnvironmentRead, RequestHeader, Requests, VariableRead};

use crate::look::{token, Token};

/// The methods Fleet sends.
pub const METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// What a method is drawn in: a read in the success hue, a write in the accent, a change in the caution hue, a removal in the
/// failure hue, and the rest quiet.
pub fn method_token(method: &str) -> Token {
    match method.to_ascii_uppercase().as_str() {
        "GET" => token::STATUS_COMPLETED_SUCCESS,
        "POST" => token::ACCENT,
        "PUT" | "PATCH" => token::STATUS_AWAITING_REVIEW,
        "DELETE" => token::STATUS_COMPLETED_FAILED,
        _ => token::FG_MUTED,
    }
}

/// Each `{{name}}` a text holds: where it stands and its name. A name is letters, digits, `_`, `.` and `-`, with room round it.
pub fn names(text: &str) -> Vec<(Range<usize>, String)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at + 1 < bytes.len() {
        if bytes[at] == b'{' && bytes[at + 1] == b'{' {
            let mut end = at + 2;
            while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            let from = end;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'_' | b'.' | b'-')) {
                end += 1;
            }
            let to = end;
            while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            if to > from && end + 1 < bytes.len() && bytes[end] == b'}' && bytes[end + 1] == b'}' {
                found.push((at..end + 2, text[from..to].to_string()));
                at = end + 2;
                continue;
            }
        }
        at += 1;
    }
    found
}

fn variable_name(variable: &VariableRead) -> &str {
    match variable {
        VariableRead::Plain { name, .. } | VariableRead::Secret { name, .. } => name,
    }
}

/// A text with each plain variable of the environment in place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filled {
    pub text: String,
    /// The names with no value in this environment, once each, in order: none there, or a secret nothing was kept for.
    pub missing: Vec<String>,
}

/// `text` filled from `environment`. **A secret stays `{{name}}`**, since Bridge never holds one; so does a name the environment
/// lacks, which `missing` lists.
pub fn filled(text: &str, environment: Option<&RequestEnvironmentRead>) -> Filled {
    let mut out = String::with_capacity(text.len());
    let mut missing: Vec<String> = Vec::new();
    let mut at = 0;
    for (range, name) in names(text) {
        out.push_str(&text[at..range.start]);
        at = range.end;
        let found = environment.and_then(|one| one.variables.iter().find(|variable| variable_name(variable) == name));
        match found {
            Some(VariableRead::Plain { value, .. }) => out.push_str(value),
            Some(VariableRead::Secret { set: true, .. }) => out.push_str(&text[range]),
            _ => {
                if !missing.contains(&name) {
                    missing.push(name);
                }
                out.push_str(&text[range]);
            }
        }
    }
    out.push_str(&text[at..]);
    Filled { text: out, missing }
}

/// An address as a path: what follows the host of one that has a scheme, else the text as it is.
pub fn path_of(url: &str) -> String {
    match url.split_once("://") {
        Some((_, rest)) => match rest.find('/') {
            Some(at) => rest[at..].to_string(),
            None => "/".to_string(),
        },
        None => url.to_string(),
    }
}

/// A time as it is read: milliseconds under a second, seconds to one place past it.
pub fn time_of(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{:.1} s", ms as f64 / 1000.)
    }
}

/// How an answer fared: a 2xx or 3xx answered, a 4xx or 5xx failed, and nothing answering is none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Answered,
    Failed,
    None,
}

pub fn outcome_of(status: Option<u16>) -> Outcome {
    match status {
        Some(status) if status < 400 => Outcome::Answered,
        Some(_) => Outcome::Failed,
        None => Outcome::None,
    }
}

impl Outcome {
    /// The glyph and the ink it is drawn in.
    pub fn look(self) -> (&'static str, Token) {
        match self {
            Outcome::Answered => ("check", token::STATUS_COMPLETED_SUCCESS),
            Outcome::Failed => ("x", token::STATUS_COMPLETED_FAILED),
            Outcome::None => ("unplug", token::STATUS_COMPLETED_FAILED),
        }
    }
}

/// The word a status goes by in a tooltip: its code, or why nothing answered.
pub fn status_said(status: Option<u16>, error: Option<&str>) -> String {
    match (status, error) {
        (Some(status), _) => status.to_string(),
        (None, Some(error)) => format!("No answer: {error}"),
        (None, None) => "No answer".to_string(),
    }
}

/// One request of this Job's thread, without the answer's body, which only an opened row needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentLine {
    pub id: String,
    pub by: RequestBy,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub error: Option<String>,
}

impl SentLine {
    pub fn who(&self) -> &'static str {
        if self.by == RequestBy::Owner {
            "You"
        } else {
            "Agent"
        }
    }
}

/// A request to start from: a thread row's, or a saved one's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Start {
    pub method: String,
    pub url: String,
    pub environment: Option<String>,
    pub headers: Vec<RequestHeader>,
    pub body: Option<String>,
}

impl Start {
    /// A row's address is filled on the row, so it reads `{{base}}` again where it starts with its environment's base.
    pub fn rebased(mut self, environments: &[Placed<RequestEnvironmentRead>]) -> Start {
        let base = self.environment.as_deref().and_then(|name| environments.iter().find(|one| one.item.name == name)).and_then(|one| {
            one.item.variables.iter().find_map(|variable| match variable {
                VariableRead::Plain { name, value } if name == ipc::SERVER_BASE_VARIABLE => Some(value.clone()),
                _ => None,
            })
        });
        if let Some(base) = base.filter(|base| !base.is_empty() && self.url.starts_with(base.as_str())) {
            self.url = format!("{{{{{}}}}}{}", ipc::SERVER_BASE_VARIABLE, &self.url[base.len()..]);
        }
        self
    }
}

/// What the list shows: which of the Job's requests and the saved ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    ThisJob,
    Saved,
    Agent,
    Mine,
}

impl Filter {
    pub const ALL: [Filter; 4] = [Filter::ThisJob, Filter::Saved, Filter::Agent, Filter::Mine];

    pub fn word(self) -> &'static str {
        match self {
            Filter::ThisJob => "This Job",
            Filter::Saved => "Saved",
            Filter::Agent => "Agent",
            Filter::Mine => "Mine",
        }
    }

    /// What the chip says in its tooltip.
    pub fn tip(self) -> &'static str {
        match self {
            Filter::ThisJob => "Saved requests and this Job's sends",
            Filter::Saved => "Saved requests only",
            Filter::Agent => "What this Job's agent sent",
            Filter::Mine => "What you sent from this Job",
        }
    }
}

/// One row of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    Heading(String),
    /// A saved request, by its place in the read: the collection, then the request.
    Saved(usize, usize),
    /// A send of this Job, by its place in the thread's lines.
    Sent(usize),
}

/// The list under `filter`: each collection's saved requests under its heading, then this Job's sends newest first under theirs.
/// A section with nothing in it has no heading.
pub fn lines(read: Option<&Requests>, sent: &[SentLine], filter: Filter) -> Vec<Line> {
    let mut lines = Vec::new();
    if matches!(filter, Filter::ThisJob | Filter::Saved) {
        for (at, collection) in read.map(|read| read.collections.as_slice()).unwrap_or_default().iter().enumerate() {
            if collection.item.requests.is_empty() {
                continue;
            }
            lines.push(Line::Heading(format!("Saved in {}", collection.item.name)));
            lines.extend((0..collection.item.requests.len()).map(|request| Line::Saved(at, request)));
        }
    }
    if matches!(filter, Filter::ThisJob | Filter::Agent | Filter::Mine) {
        let keep = |line: &SentLine| match filter {
            Filter::Agent => line.by == RequestBy::Agent,
            Filter::Mine => line.by == RequestBy::Owner,
            _ => true,
        };
        let held: Vec<usize> = sent.iter().enumerate().filter(|(_, line)| keep(line)).map(|(at, _)| at).rev().collect();
        if !held.is_empty() {
            lines.push(Line::Heading("Sent in this Job".to_string()));
            lines.extend(held.into_iter().map(Line::Sent));
        }
    }
    lines
}

/// The saved requests whose name an address holds as typed, up to `limit`, for an address that reads as a name: no scheme, no
/// slash and no `{{name}}`.
pub fn found(read: &Requests, typed: &str, limit: usize) -> Vec<(usize, usize)> {
    let needle = typed.trim().to_lowercase();
    if needle.is_empty() || needle.contains('/') || needle.contains("{{") || needle.contains("://") {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for (at, collection) in read.collections.iter().enumerate() {
        for (request, one) in collection.item.requests.iter().enumerate() {
            if one.name.to_lowercase().contains(&needle) {
                hits.push((at, request));
                if hits.len() == limit {
                    return hits;
                }
            }
        }
    }
    hits
}

/// An answer's body set out two spaces deep where it is JSON, and as it came otherwise.
pub fn body_shown(body: &str) -> (String, bool) {
    let trimmed = body.trim();
    if (trimmed.starts_with('{') || trimmed.starts_with('[')) && ipc::decode::<serde::de::IgnoredAny>("body", trimmed.as_bytes()).is_ok() {
        (indented(trimmed), true)
    } else {
        (body.to_string(), false)
    }
}

/// JSON text that is known to be valid, set out two spaces deep with its keys in the order they came.
fn indented(json: &str) -> String {
    fn line(out: &mut String, depth: usize) {
        out.push('\n');
        out.extend(std::iter::repeat_n(' ', depth * 2));
    }
    let chars: Vec<char> = json.chars().collect();
    let mut out = String::with_capacity(json.len() * 2);
    let (mut depth, mut inside, mut escaped, mut at) = (0usize, false, false, 0usize);
    while at < chars.len() {
        let one = chars[at];
        if inside {
            out.push(one);
            if escaped {
                escaped = false;
            } else if one == '\\' {
                escaped = true;
            } else if one == '"' {
                inside = false;
            }
            at += 1;
            continue;
        }
        match one {
            '"' => {
                inside = true;
                out.push(one);
            }
            '{' | '[' => {
                let close = if one == '{' { '}' } else { ']' };
                let mut next = at + 1;
                while next < chars.len() && chars[next].is_whitespace() {
                    next += 1;
                }
                if chars.get(next) == Some(&close) {
                    out.push(one);
                    out.push(close);
                    at = next + 1;
                    continue;
                }
                depth += 1;
                out.push(one);
                line(&mut out, depth);
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                line(&mut out, depth);
                out.push(one);
            }
            ',' => {
                out.push(one);
                line(&mut out, depth);
            }
            ':' => out.push_str(": "),
            one if one.is_whitespace() => {}
            one => out.push(one),
        }
        at += 1;
    }
    out
}

/// The first header whose name holds a space, which no header name can.
pub fn bad_header(names: &[String]) -> Option<usize> {
    names.iter().position(|name| name.trim().chars().any(char::is_whitespace))
}
