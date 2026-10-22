//! What a send is filled from: the request it names, saved or its own, with its collection's
//! headers under its own, and the values of the environment it names.
//!
//! **One pass**: a `{{name}}` is replaced where it is written, and a value holding `{{other}}` is
//! sent as it is, never filled again.

use std::collections::BTreeMap;

use adapter_traits::{RequestSecrets, SecretError, SecretVariable};
use ipc::{RequestCollection, RequestEnvironment, RequestHeader, RequestToSend, RequestVariable, SendRequest};

use super::{is_named, Unsent};

/// The shortest secret value taken out of what is answered.
const SHORTEST_HIDDEN: usize = 6;

/// What a send is filled from, read before it goes.
#[derive(Clone, Debug, Default)]
pub struct Filling {
    /// The repository's key, which every secret's Keychain account starts with. `None` where the
    /// send places no repository, so no secret is read.
    pub repository: Option<String>,
    /// As Requests lists them: the person's own over the repository's.
    pub collections: Vec<RequestCollection>,
    /// The saved ones, the person's own over the repository's, then a server's for each server
    /// the Job runs.
    pub environments: Vec<RequestEnvironment>,
}

/// A request before it is filled.
pub(super) struct Asked {
    pub(super) method: String,
    pub(super) url: String,
    pub(super) headers: Vec<RequestHeader>,
    pub(super) body: Option<String>,
}

impl Asked {
    /// Every text a `{{name}}` is filled in.
    pub(super) fn texts(&self) -> Vec<&str> {
        let mut texts = vec![self.url.as_str()];
        texts.extend(self.headers.iter().map(|one| one.value.as_str()));
        texts.extend(self.body.as_deref());
        texts
    }
}

/// `under` with each header `over` names, in any case, taken out, then `over`.
pub(crate) fn layered(under: &[RequestHeader], over: &[RequestHeader]) -> Vec<RequestHeader> {
    under
        .iter()
        .filter(|one| !over.iter().any(|top| is_named(&top.name, &one.name)))
        .chain(over)
        .cloned()
        .collect()
}

/// The one of `items` whose id is `named`, or failing that whose name is.
pub(crate) fn by_id_or_name<'a, T>(
    items: &'a [T],
    named: &str,
    id: impl Fn(&T) -> &str,
    name: impl Fn(&T) -> &str,
) -> Option<&'a T> {
    let named = named.trim();
    items.iter().find(|one| id(one) == named).or_else(|| items.iter().find(|one| name(one) == named))
}

impl Filling {
    fn collection(&self, named: &str) -> Result<&RequestCollection, String> {
        by_id_or_name(&self.collections, named, |one| &one.id, |one| &one.name)
            .ok_or_else(|| format!("there is no collection {named:?}"))
    }

    /// The request `send` names, its collection's headers under its own and the send's over both.
    pub(super) fn asked(&self, send: &SendRequest) -> Result<Asked, String> {
        match &send.request {
            RequestToSend::Saved(saved) => {
                let collection = self.collection(&saved.collection)?;
                let request = by_id_or_name(&collection.requests, &saved.request, |one| &one.id, |one| &one.name)
                    .ok_or_else(|| format!("{:?} holds no request {:?}", collection.name, saved.request))?;
                Ok(Asked {
                    method: request.method.clone(),
                    url: request.url.clone(),
                    headers: layered(&layered(&collection.headers, &request.headers), &send.headers),
                    body: send.body.clone().or_else(|| request.body.clone()),
                })
            }
            RequestToSend::Own { method, url, collection } => {
                let under = match collection.as_deref().map(str::trim).filter(|named| !named.is_empty()) {
                    Some(named) => self.collection(named)?.headers.clone(),
                    None => Vec::new(),
                };
                Ok(Asked {
                    method: method.clone(),
                    url: url.clone(),
                    headers: layered(&under, &send.headers),
                    body: send.body.clone(),
                })
            }
        }
    }

    /// The values `environment` fills `texts` with: its plain ones, and the secrets they name read
    /// from `secrets`.
    pub(super) fn values(
        &self,
        environment: Option<&str>,
        texts: &[&str],
        secrets: &dyn RequestSecrets,
    ) -> Result<Values, Unsent> {
        let Some(named) = environment else { return Ok(Values::default()) };
        let environment = self
            .environments
            .iter()
            .find(|one| one.name == named)
            .ok_or_else(|| Unsent::Refused(format!("there is no environment {named:?}")))?;
        let wanted: Vec<String> = texts.iter().flat_map(|text| names_in(text)).collect();
        let mut values = Values::default();
        for variable in &environment.variables {
            match variable {
                RequestVariable::Plain { name, value } => {
                    values.known.insert(name.clone(), value.clone());
                }
                RequestVariable::Secret { name } if wanted.contains(name) => {
                    let Some(repository) = &self.repository else { continue };
                    let asked = SecretVariable {
                        repository: repository.clone(),
                        environment: environment.name.clone(),
                        name: name.clone(),
                    };
                    let read = secrets.read(&asked).map_err(|why| match why {
                        SecretError::Refused(why) | SecretError::Unkept(why) => {
                            Unsent::Unkept(format!("the secret {name} could not be read: {why}"))
                        }
                    })?;
                    if let Some(value) = read {
                        values.known.insert(name.clone(), value.expose().clone());
                        values.secret.push((name.clone(), value.expose().clone()));
                    }
                }
                RequestVariable::Secret { .. } => {}
            }
        }
        Ok(values)
    }
}

/// Every `{{name}}` in `text`, in order: a name is letters, digits, `_`, `.` and `-`, and spaces
/// inside the braces are not part of it.
fn names_in(text: &str) -> Vec<String> {
    placeholders(text).into_iter().map(|(_, _, name)| name).collect()
}

/// Each `{{name}}`'s start, end and name.
fn placeholders(text: &str) -> Vec<(usize, usize, String)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find("{{").map(|at| at + from) {
        let Some(close) = text[open + 2..].find("}}").map(|at| at + open + 2) else { break };
        let name = text[open + 2..close].trim();
        let fits = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c));
        if fits {
            found.push((open, close + 2, name.to_string()));
            from = close + 2;
        } else {
            from = open + 2;
        }
    }
    found
}

/// The values a send is filled with. **Holds secrets' values**, so it never leaves this module's
/// caller and has no `Debug`.
#[derive(Default)]
pub(super) struct Values {
    known: BTreeMap<String, String>,
    /// Each secret filled in, by name: what [`Values::hidden`] takes back out.
    secret: Vec<(String, String)>,
}

impl Values {
    /// `text` with each `{{name}}` a value is known for replaced by it.
    pub(super) fn filled(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        for (open, close, name) in placeholders(text) {
            if let Some(value) = self.known.get(&name) {
                out.push_str(&text[at..open]);
                out.push_str(value);
                at = close;
            }
        }
        out.push_str(&text[at..]);
        out
    }

    /// Each name written in `texts` that no value fills, once, in the order written.
    pub(super) fn unfilled(&self, texts: &[&str]) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for name in texts.iter().flat_map(|text| names_in(text)) {
            if !self.known.contains_key(&name) && !names.contains(&name) {
                names.push(name);
            }
        }
        names
    }

    /// `text` with each secret's value put back as `{{name}}`.
    pub(super) fn hidden(&self, text: &str) -> String {
        hidden(text, &self.secret)
    }

    /// The longest secret's length.
    pub(super) fn longest(&self) -> usize {
        self.secret.iter().map(|(_, value)| value.len()).max().unwrap_or(0)
    }
}

/// `text` with each of `secrets`' values, six characters or more, replaced by its `{{name}}`,
/// the longest first so a value inside another is not left half hidden.
pub fn hidden(text: &str, secrets: &[(String, String)]) -> String {
    let mut by_length: Vec<&(String, String)> =
        secrets.iter().filter(|(_, value)| value.len() >= SHORTEST_HIDDEN).collect();
    by_length.sort_by_key(|(_, value)| std::cmp::Reverse(value.len()));
    by_length
        .into_iter()
        .fold(text.to_string(), |text, (name, value)| text.replace(value.as_str(), &format!("{{{{{name}}}}}")))
}
