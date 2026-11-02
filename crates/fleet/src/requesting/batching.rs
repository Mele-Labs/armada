//! `send_requests` as a function of values: a template expanded over body files and values, then
//! each request through [`sent`] with a few in flight and a stop after a run of failures.
//!
//! **`vary`'s name is replaced before the environment is filled**, so the Job writes the
//! placeholder and never sees a secret, and every other `{{name}}` is left for [`sent`].

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Mutex;

use adapter_traits::{RequestSecrets, RequestSender};
use ipc::{RequestBy, RequestHeader, RequestLine, RequestToSend, RequestsSent, SendRequest, SendRequests};

use super::{sent, Bounds, Filling, Unsent};

/// The most requests one call sends.
pub(crate) const MOST: usize = 1000;
const CONCURRENCY: u32 = 5;
const MOST_CONCURRENCY: u32 = 10;
const STOP_AFTER: u32 = 5;
/// The most of an error a line keeps, so four hundred lines fit the door's cap.
const ERROR_CAP: usize = 160;

/// One request of the batch and the file name its answer is saved under.
pub(crate) struct Item {
    pub name: String,
    pub send: SendRequest,
}

/// `text` with each `{{name}}`, spaces inside the braces allowed, replaced by `value`.
fn replaced(text: &str, name: &str, value: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        match after.find("}}") {
            Some(close) if after[..close].trim() == name => {
                out.push_str(value);
                rest = &after[close + 2..];
            }
            // Not ours: keep the braces and look on from inside them.
            _ => {
                out.push_str("{{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `text` as a file name: a letter, digit, dot, dash or underscore each, anything else `_`.
fn as_file_name(text: &str) -> String {
    text.chars().take(48).map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect()
}

/// Every request of `batch`: one for each of `files` (a stem and its text) and each value.
pub(crate) fn expanded(batch: &SendRequests, files: &[(String, String)]) -> Result<Vec<Item>, String> {
    let values: Vec<Option<&str>> = match &batch.vary {
        Some(vary) if !vary.values.is_empty() => vary.values.iter().map(|value| Some(value.as_str())).collect(),
        Some(_) => return Err("`vary` names no values".to_string()),
        None => vec![None],
    };
    let bodies: Vec<Option<&(String, String)>> = match files.is_empty() {
        true => vec![None],
        false => files.iter().map(Some).collect(),
    };
    if batch.vary.is_none() && files.is_empty() {
        return Err("name `body_files` or `vary`: with neither there is one request, and `send_request` sends it".to_string());
    }
    let count = bodies.len() * values.len();
    if count > MOST {
        return Err(format!("{count} requests is more than the {MOST} a call sends: send them in more than one call"));
    }
    let mut taken = BTreeSet::new();
    let mut items = Vec::with_capacity(count);
    for body in &bodies {
        for value in &values {
            let fill = |text: &str| match (&batch.vary, value) {
                (Some(vary), Some(value)) => replaced(text, &vary.name, value),
                _ => text.to_string(),
            };
            let stem = body.map(|(stem, _)| as_file_name(stem));
            let named = match (stem, value) {
                (Some(stem), Some(value)) => format!("{stem}-{}", as_file_name(value)),
                (Some(stem), None) => stem,
                (None, Some(value)) => as_file_name(value),
                (None, None) => unreachable!("one of the two was named above"),
            };
            let mut name = named.clone();
            for again in 2.. {
                if taken.insert(name.clone()) {
                    break;
                }
                name = format!("{named}~{again}");
            }
            let template = &batch.template;
            items.push(Item {
                name,
                send: SendRequest {
                    session_id: None,
                    repository: None,
                    request: RequestToSend::Own {
                        method: template.method.clone(),
                        url: fill(&template.url),
                        collection: template.collection.clone(),
                    },
                    environment: batch.environment.clone(),
                    headers: template
                        .headers
                        .iter()
                        .map(|one| RequestHeader { name: one.name.clone(), value: fill(&one.value) })
                        .collect(),
                    body: body.map(|(_, text)| fill(text)),
                    body_file: None,
                    // A line never holds a body: the file does.
                    body_inline: false,
                    pick: None,
                },
            });
        }
    }
    Ok(items)
}

/// Where the run has got to, shared by the workers.
#[derive(Default)]
struct Progress {
    next: usize,
    failed_in_a_row: u32,
    stopped: bool,
    unkept: Option<String>,
}

/// Send `items` with at most `concurrency` in flight, saving each answer under `folder`, and
/// stop starting new ones after `stop_after` failures in a row. **A secret's value is in no line.**
pub(crate) fn run(
    items: &[Item],
    sender: &(dyn RequestSender + Send + Sync),
    secrets: &(dyn RequestSecrets + Send + Sync),
    filling: &Filling,
    bounds: &Bounds,
    batch: &SendRequests,
    folder: &Path,
) -> Result<RequestsSent, Unsent> {
    let concurrency = batch.concurrency.unwrap_or(CONCURRENCY).clamp(1, MOST_CONCURRENCY) as usize;
    let stop_after = batch.stop_after.unwrap_or(STOP_AFTER).max(1);
    let progress = Mutex::new(Progress::default());
    let lines: Vec<Mutex<Option<RequestLine>>> = items.iter().map(|_| Mutex::new(None)).collect();
    let notes = Mutex::new(BTreeSet::new());
    std::thread::scope(|scope| {
        for _ in 0..concurrency.min(items.len()) {
            scope.spawn(|| loop {
                // The stop and the next index are read under one lock, so nothing starts after it.
                let at = {
                    let mut now = progress.lock().expect("no worker panics holding it");
                    if now.stopped || now.next >= items.len() {
                        return;
                    }
                    now.next += 1;
                    now.next - 1
                };
                let item = &items[at];
                let line = match sent(sender, secrets, RequestBy::Agent, &item.send, filling, bounds) {
                    Ok(mut done) => {
                        done.save_body(folder, &item.name);
                        if let Some(note) = &done.answer.note {
                            notes.lock().expect("no worker panics holding it").insert(note.clone());
                        }
                        let answer = &done.answer;
                        RequestLine {
                            name: item.name.clone(),
                            status: answer.status,
                            ms: answer.ms,
                            saved: answer
                                .saved
                                .as_ref()
                                .and_then(|saved| Path::new(&saved.path).file_name())
                                .map(|file| file.to_string_lossy().into_owned()),
                            error: answer.error.as_deref().map(|why| super::cut(why, ERROR_CAP).0),
                            skipped: false,
                        }
                    }
                    Err(Unsent::Refused(why)) => RequestLine {
                        name: item.name.clone(),
                        status: None,
                        ms: 0,
                        saved: None,
                        error: Some(super::cut(&why, ERROR_CAP).0),
                        skipped: false,
                    },
                    Err(Unsent::Unkept(why)) => {
                        let mut now = progress.lock().expect("no worker panics holding it");
                        now.stopped = true;
                        now.unkept.get_or_insert(why);
                        return;
                    }
                };
                let failed = line.status.is_none_or(|status| status >= 400);
                *lines[at].lock().expect("no worker panics holding it") = Some(line);
                let mut now = progress.lock().expect("no worker panics holding it");
                now.failed_in_a_row = if failed { now.failed_in_a_row + 1 } else { 0 };
                if now.failed_in_a_row >= stop_after {
                    now.stopped = true;
                }
            });
        }
    });
    let progress = progress.into_inner().expect("no worker panics holding it");
    if let Some(why) = progress.unkept {
        return Err(Unsent::Unkept(why));
    }
    let lines: Vec<RequestLine> = lines
        .into_iter()
        .zip(items)
        .map(|(line, item)| {
            line.into_inner().expect("no worker panics holding it").unwrap_or_else(|| RequestLine {
                name: item.name.clone(),
                status: None,
                ms: 0,
                saved: None,
                error: None,
                skipped: true,
            })
        })
        .collect();
    let skipped = lines.iter().filter(|line| line.skipped).count();
    let mut said: Vec<String> = Vec::new();
    if skipped > 0 {
        said.push(format!(
            "Stopped after {stop_after} answers in a row that were a 4xx, a 5xx or an error: {skipped} of {} were not sent.",
            lines.len()
        ));
    }
    said.extend(notes.into_inner().expect("no worker panics holding it").into_iter().take(5));
    Ok(RequestsSent {
        folder: folder.join("requests").to_string_lossy().into_owned(),
        lines,
        note: (!said.is_empty()).then(|| said.join(" ")),
    })
}
