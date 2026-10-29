//! Checks on what a report asks a window or a walk to do, before Fleet acts on it.

use super::*;


/// Whether an address is one Bridge's window will load: any `http` or `https` address with a host
/// and no blank in it (`docs/practices/capture-window.md`).
pub(super) fn shows_a_page(url: &str) -> bool {
    let Some(rest) = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit('@').next().unwrap_or_default();
    let name = match host.strip_prefix('[') {
        Some(inner) => inner.split(']').next().unwrap_or_default(),
        None => host.split(':').next().unwrap_or_default(),
    };
    !name.is_empty() && !url.contains(char::is_whitespace)
}

/// An address a Job opened outside Bridge, as its reminder row shows it: **no query, fragment or
/// sign-in**, which is where a token rides, and at most 300 characters. `None` where it is blank.
pub(crate) fn address_shown(address: &str) -> Option<String> {
    let address = address.trim().split(['?', '#']).next().unwrap_or_default();
    let address = match address.split_once("://") {
        Some((scheme, rest)) => {
            let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
            format!("{scheme}://{}{path}", authority.rsplit('@').next().unwrap_or_default())
        }
        None => address.to_string(),
    };
    let shown: String = address.chars().take(300).collect();
    (!shown.is_empty()).then_some(shown)
}

/// The takes a call named, trimmed, or why they cannot be shown. **Each address is held to the one
/// `url` is**, and a call that names takes shows the first: `url` must be that same page.
pub(super) fn takes_shown(url: &str, named: Vec<ipc::WindowTake>) -> Result<Vec<ipc::WindowTake>, &'static str> {
    let takes: Vec<_> = named
        .into_iter()
        .map(|one| ipc::WindowTake { name: one.name.trim().to_string(), url: one.url.trim().to_string() })
        .collect();
    if takes.iter().any(|one| one.name.is_empty()) {
        return Err("a name on every variation");
    }
    if takes.iter().any(|one| !shows_a_page(&one.url)) {
        return Err("an `http` or `https` address on every variation");
    }
    if takes.first().is_some_and(|first| first.url != url) {
        return Err("`url` to be the first variation's address");
    }
    Ok(takes)
}

/// Most steps one walk carries, and most the walk weighs once written. **It rides on the session's
/// record** as the window's artifact, which is published whole on every change.
const MOST_STEPS: usize = 200;
const MOST_STEP_BYTES: usize = 32 * 1024;
/// Longest a caption, a name, a typed text and a key may be, in characters.
const MOST_SAID: usize = 240;
const MOST_NAME: usize = 200;
const MOST_TYPED: usize = 1000;
const MOST_KEY: usize = 40;
/// How deep `within` may nest, and how far a drag may go, in pixels.
const MOST_NESTING: usize = 4;
const MOST_DRAG: i32 = 5000;

/// Why a target cannot be found on a page, or `None` where it can be looked for.
fn target_problem(target: &ipc::WalkTarget, depth: usize) -> Option<&'static str> {
    if target.role.trim().is_empty() || target.role.chars().any(char::is_whitespace) || target.role.chars().count() > MOST_KEY {
        return Some("to have a `role` of one word on every target, such as `button`, or `text`");
    }
    match target.name.as_deref() {
        Some(name) if name.chars().count() > MOST_NAME => return Some("to have a target `name` of no more than 200 characters"),
        Some(name) if name.trim().is_empty() => return Some("to have a target `name` with words in it, or none"),
        None if target.role == "text" => return Some("to have a `name` on a `text` target: the words it draws"),
        _ => {}
    }
    match target.within.as_deref() {
        Some(_) if depth >= MOST_NESTING => Some("to nest its targets no more than four deep"),
        Some(inner) => target_problem(inner, depth + 1),
        None => None,
    }
}

/// What is wrong with one step, said as what the report needs of it, or `None`.
fn step_problem(step: &ipc::WalkStep) -> Option<&'static str> {
    if step.say.trim().is_empty() || step.say.chars().count() > MOST_SAID {
        return Some("to say, in a line of no more than 240 characters, what it shows");
    }
    let verbs = [
        step.press.is_some(),
        step.look.is_some(),
        step.hover.is_some(),
        step.later.is_some(),
        step.typed.is_some(),
        step.key.is_some(),
        step.select.is_some(),
        step.paste.is_some(),
        step.drag.is_some(),
    ];
    if verbs.into_iter().filter(|one| *one).count() != 1 {
        return Some("to name exactly one verb of press, look, hover, later, type, key, select, paste or drag");
    }
    // The target is the verb's own, and the keys that belong to another verb are stray.
    let (target, stray) = if step.typed.is_some() {
        (step.into.as_ref(), [step.on.is_some(), step.within.is_some(), step.by.is_some()])
    } else if step.key.is_some() {
        (step.on.as_ref(), [step.into.is_some(), step.within.is_some(), step.by.is_some()])
    } else if step.select.is_some() {
        (step.within.as_ref(), [step.into.is_some(), step.on.is_some(), step.by.is_some()])
    } else if step.drag.is_some() {
        (step.drag.as_ref(), [step.into.is_some(), step.on.is_some(), step.within.is_some()])
    } else {
        let own = step.press.as_ref().or(step.look.as_ref()).or(step.hover.as_ref()).or(step.later.as_ref()).or(step.paste.as_ref());
        (own, [step.into.is_some() || step.on.is_some(), step.within.is_some(), step.by.is_some()])
    };
    if stray.into_iter().any(|one| one) || (step.drag.is_some() && step.by.is_none()) {
        return Some("to pair each verb with its own target: `type` with `into`, `key` with `on`, `select` with `in`, `drag` with `by`, and no other");
    }
    let Some(target) = target else {
        return Some("to have a target: `into` for `type`, `on` for `key`, `in` for `select`");
    };
    if let Some(problem) = target_problem(target, 0) {
        return Some(problem);
    }
    if step.typed.as_deref().is_some_and(|words| words.chars().count() > MOST_TYPED) {
        return Some("to type no more than 1000 characters");
    }
    if step.key.as_deref().is_some_and(|key| key.is_empty() || key.chars().count() > MOST_KEY) {
        return Some("to have a `key` such as `n`, `Tab` or `Meta+n`");
    }
    if step.select.as_deref().is_some_and(|words| words.is_empty() || words.chars().count() > MOST_NAME) {
        return Some("to select words of no more than 200 characters");
    }
    if step.by.is_some_and(|by| by.x.abs() > MOST_DRAG || by.y.abs() > MOST_DRAG) {
        return Some("to drag `by` no more than 5000 pixels each way");
    }
    None
}

/// The steps a call named, trimmed, or what the report needs of them. **The first step that is
/// wrong is named by its place**, so the session fixes that one and not the whole list.
pub(crate) fn steps_shown(named: Vec<ipc::WalkStep>) -> Result<Vec<ipc::WalkStep>, String> {
    if named.len() > MOST_STEPS {
        return Err(String::from("no more than 200 steps"));
    }
    let steps: Vec<_> = named
        .into_iter()
        .map(|mut one| {
            one.say = one.say.trim().to_string();
            one
        })
        .collect();
    if let Some((at, problem)) = steps.iter().enumerate().find_map(|(at, one)| step_problem(one).map(|why| (at, why))) {
        return Err(format!("step {} {problem}", at + 1));
    }
    if ipc::WalkStep::as_detail(&steps).len() > MOST_STEP_BYTES {
        return Err(String::from("steps that weigh no more than 32 KiB written out"));
    }
    Ok(steps)
}

/// The origins a call named, trimmed, or why they cannot be passed through. **An origin only**:
/// `http` or `https`, a host and an optional port, and no path, query, fragment or sign-in of its own.
pub(super) fn origins_named(named: Vec<String>) -> Result<Vec<String>, &'static str> {
    let origins: Vec<String> = named.into_iter().map(|one| one.trim().to_string()).collect();
    let bare = |one: &String| {
        shows_a_page(one) && one.split_once("://").is_some_and(|(_, rest)| !rest.contains(['/', '?', '#', '@']))
    };
    if origins.iter().all(bare) {
        Ok(origins)
    } else {
        Err("every `origins` entry to be an `http` or `https` origin with no path, such as https://login.example.com")
    }
}

/// The sign-in addresses a call named, trimmed, or why they cannot be loaded. **Each is a page
/// on the shown origin or on one of `origins`**, so a sign-in reaches nowhere the window may not.
pub(super) fn sign_in_named(url: &str, origins: &[String], named: Vec<String>) -> Result<Vec<String>, &'static str> {
    let addresses: Vec<String> = named.into_iter().map(|one| one.trim().to_string()).collect();
    let allowed: Vec<String> = std::iter::once(url).chain(origins.iter().map(String::as_str)).filter_map(origin_of).collect();
    let on_one = |one: &String| shows_a_page(one) && origin_of(one).is_some_and(|origin| allowed.contains(&origin));
    if addresses.iter().all(on_one) {
        Ok(addresses)
    } else {
        Err("every `sign_in` entry to be an `http` or `https` address on the shown origin or one of `origins`")
    }
}

/// The two kinds a session holds one of at a time. Every other kind is kept as
/// the open word it arrived as.
pub(super) const ONE_AT_A_TIME: [&str; 2] = ["branch", "slot"];
