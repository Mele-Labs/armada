//! The requests file, the same at either place: its rules, its text, what an environment Bridge
//! writes keeps in it, and how the person's own and the repository's are listed together.
//!
//! **The rules name the first broken field**, as a save's body spells it (`collection.requests[1].method`)
//! or as the file does (`collection[0].requests[1].method`), and never quote a value.

use std::collections::BTreeMap;

use adapter_traits::Secret;
use ipc::{
    Placed, RequestCollection, RequestEnvironment, RequestEnvironmentWrite, RequestPlace,
    RequestVariable, RequestsFile, SavedRequest, VariableWrite, REQUESTS_VERSION, SERVER_ENVIRONMENT_PREFIX,
};

use super::{header_named, METHODS};

/// What saving an environment asks of the Keychain. **No `Debug`**: `Keep` holds a value.
pub enum SecretChange {
    /// Keep `value` for the variable `name`, replacing what was kept.
    Keep { name: String, value: Secret<String> },
    /// The variable `name` is no longer secret, or no longer there: take its value out.
    Forget { name: String },
}

fn variable_named(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
}

fn headers_fit(headers: &[ipc::RequestHeader], at: &str) -> Result<(), String> {
    match headers.iter().position(|one| !header_named(one.name.trim())) {
        Some(index) => Err(format!("{at}.headers[{index}].name is not a header name")),
        None => Ok(()),
    }
}

/// A saved request's rules, `here` naming it.
pub(crate) fn check_request(request: &SavedRequest, here: &str) -> Result<(), String> {
    if request.id.trim().is_empty() {
        return Err(format!("{here}.id is blank"));
    }
    if request.name.trim().is_empty() {
        return Err(format!("{here}.name is blank"));
    }
    let method = request.method.trim().to_ascii_uppercase();
    if !METHODS.contains(&method.as_str()) {
        return Err(format!("{here}.method is not one of {}", METHODS.join(", ")));
    }
    headers_fit(&request.headers, here)
}

/// A collection's rules, `at` naming it.
pub fn check_collection(collection: &RequestCollection, at: &str) -> Result<(), String> {
    if collection.id.trim().is_empty() {
        return Err(format!("{at}.id is blank"));
    }
    if collection.name.trim().is_empty() {
        return Err(format!("{at}.name is blank"));
    }
    headers_fit(&collection.headers, at)?;
    let mut ids = BTreeMap::new();
    for (index, request) in collection.requests.iter().enumerate() {
        let here = format!("{at}.requests[{index}]");
        check_request(request, &here)?;
        let id = request.id.trim();
        if let Some(first) = ids.insert(id, index) {
            return Err(format!("{here}.id `{id}` is already {at}.requests[{first}].id"));
        }
    }
    Ok(())
}

fn environment_name_fits(name: &str, at: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err(format!("{at}.name is blank"));
    }
    if name.starts_with(SERVER_ENVIRONMENT_PREFIX) {
        return Err(format!("{at}.name starts with `{SERVER_ENVIRONMENT_PREFIX}`, which names a server a Job runs"));
    }
    if name.contains(['\n', '\r']) {
        return Err(format!("{at}.name is more than one line"));
    }
    Ok(())
}

fn variable_names_fit<'a>(names: impl Iterator<Item = &'a str>, at: &str) -> Result<(), String> {
    let mut seen = BTreeMap::new();
    for (index, name) in names.enumerate() {
        if !variable_named(name) {
            return Err(format!("{at}.variables[{index}].name is not letters, digits, `_`, `.` and `-`"));
        }
        if let Some(first) = seen.insert(name, index) {
            return Err(format!("{at}.variables[{index}].name `{name}` is already {at}.variables[{first}].name"));
        }
    }
    Ok(())
}

/// An environment's rules, as a file keeps it.
pub fn check_environment(environment: &RequestEnvironment, at: &str) -> Result<(), String> {
    environment_name_fits(&environment.name, at)?;
    variable_names_fit(environment.variables.iter().map(|one| variable_name(one)), at)
}

/// An environment's rules, as Bridge writes it: a secret's value is one line of printable ASCII,
/// which a header carries and the Keychain reads back as written.
pub(crate) fn check_environment_write(environment: &RequestEnvironmentWrite, at: &str) -> Result<(), String> {
    environment_name_fits(&environment.name, at)?;
    let names = environment.variables.iter().map(|one| match one {
        VariableWrite::Plain { name, .. } | VariableWrite::Secret { name, .. } => name.as_str(),
    });
    variable_names_fit(names, at)?;
    for (index, variable) in environment.variables.iter().enumerate() {
        if let VariableWrite::Secret { value: Some(value), .. } = variable {
            if value.is_empty() || !value.bytes().all(|byte| (0x20..0x7f).contains(&byte)) {
                return Err(format!("{at}.variables[{index}].value is not one line of printable ASCII"));
            }
        }
    }
    Ok(())
}

pub(crate) fn variable_name(variable: &RequestVariable) -> &str {
    match variable {
        RequestVariable::Plain { name, .. } | RequestVariable::Secret { name } => name,
    }
}

/// The file's rules: its version, collections unique by id and environments by name.
pub fn check(file: &RequestsFile) -> Result<(), String> {
    if file.version != REQUESTS_VERSION {
        return Err(format!("version is {}, and this Fleet reads {REQUESTS_VERSION}", file.version));
    }
    let mut ids = BTreeMap::new();
    for (index, collection) in file.collections.iter().enumerate() {
        let at = format!("collection[{index}]");
        check_collection(collection, &at)?;
        if let Some(first) = ids.insert(collection.id.as_str(), index) {
            return Err(format!("{at}.id `{}` is already collection[{first}].id", collection.id));
        }
    }
    let mut names = BTreeMap::new();
    for (index, environment) in file.environments.iter().enumerate() {
        let at = format!("environment[{index}]");
        check_environment(environment, &at)?;
        if let Some(first) = names.insert(environment.name.as_str(), index) {
            return Err(format!("{at}.name `{}` is already environment[{first}].name", environment.name));
        }
    }
    Ok(())
}

/// What a file's text comes to. **A refusal names the line and never quotes it**, since a line
/// spelling a secret's value would carry the value with it.
pub fn parse(text: &str) -> Result<RequestsFile, String> {
    let file: RequestsFile = toml::from_str(text).map_err(|why| match why.span() {
        Some(span) => format!("line {}: {}", text[..span.start.min(text.len())].matches('\n').count() + 1, why.message()),
        None => why.message().to_string(),
    })?;
    check(&file)?;
    Ok(file)
}

/// The file's text, as every place writes it: the person's own, a commit and a package.
pub fn written(file: &RequestsFile) -> Result<String, String> {
    toml::to_string(file).map_err(|why| why.to_string())
}

/// What an environment written by Bridge keeps in the file, and what it asks of the Keychain.
/// `before` is the environment of that name the person's own held. **A secret written with no
/// value keeps the one kept**; a secret `before` held that is now plain or gone is forgotten.
pub fn environment_written(
    write: RequestEnvironmentWrite,
    before: Option<&RequestEnvironment>,
) -> (RequestEnvironment, Vec<SecretChange>) {
    let mut changes = Vec::new();
    let mut variables = Vec::new();
    for variable in write.variables {
        match variable {
            VariableWrite::Plain { name, value } => variables.push(RequestVariable::Plain { name, value }),
            VariableWrite::Secret { name, value } => {
                if let Some(value) = value {
                    changes.push(SecretChange::Keep { name: name.clone(), value: Secret::new(value) });
                }
                variables.push(RequestVariable::Secret { name });
            }
        }
    }
    let secret_now = |name: &str| variables.iter().any(|one| matches!(one, RequestVariable::Secret { name: now } if now == name));
    for gone in before.map(|one| one.variables.as_slice()).unwrap_or_default() {
        if let RequestVariable::Secret { name } = gone {
            if !secret_now(name) {
                changes.push(SecretChange::Forget { name: name.clone() });
            }
        }
    }
    (RequestEnvironment { name: write.name, variables }, changes)
}

/// The person's own and the repository's listed together, keyed by `key`: the person's own first,
/// each marked `hides` where the repository holds one of its key, then the repository's it does
/// not hide.
pub(crate) fn merged<T: Clone>(own: &[T], repository: &[T], key: impl Fn(&T) -> &str) -> Vec<Placed<T>> {
    let hides = |one: &T| repository.iter().any(|theirs| key(theirs) == key(one));
    let yours = own.iter().map(|one| Placed { item: one.clone(), place: RequestPlace::Yours, hides: hides(one) });
    let theirs = repository
        .iter()
        .filter(|one| !own.iter().any(|mine| key(mine) == key(one)))
        .map(|one| Placed { item: one.clone(), place: RequestPlace::Repository, hides: false });
    yours.chain(theirs).collect()
}

/// A plain address slug for a request or collection saved under `name`, as Requests makes one.
pub(crate) fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.trim().to_lowercase().chars() {
        match c.is_ascii_lowercase() || c.is_ascii_digit() {
            true => slug.push(c),
            false if !slug.ends_with('-') => slug.push('-'),
            false => {}
        }
    }
    let slug = slug.trim_matches('-').to_string();
    match slug.is_empty() {
        true => String::from("request"),
        false => slug,
    }
}

/// `slug(name)`, with `-2`, `-3` and on added until no id in `taken` is it.
pub(crate) fn unique_id<'a>(name: &str, taken: impl Iterator<Item = &'a str> + Clone) -> String {
    let base = slug(name);
    let free = |id: &str| !taken.clone().any(|one| one == id);
    if free(&base) {
        return base;
    }
    (2..).map(|n| format!("{base}-{n}")).find(|id| free(id)).unwrap_or(base)
}
