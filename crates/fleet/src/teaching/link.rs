//! Whether a taught link rule can work where it will run. Bridge reads the pattern as a JavaScript
//! `RegExp` and opens the address, so a rule is held to the part of regular expressions both
//! engines read the same, and to an address that is a web address with a host of its own.

use regex::RegexBuilder;

/// The longest pattern, in characters.
const MOST_PATTERN: usize = 200;

/// Why this rule cannot be kept, where it cannot. `words` are the selected words it was taught from.
pub(crate) fn fault(pattern: &str, address: &str, words: &str) -> Option<String> {
    pattern_fault(pattern, words).or_else(|| address_fault(address))
}

pub(crate) fn pattern_fault(pattern: &str, words: &str) -> Option<String> {
    if pattern.is_empty() || pattern.chars().count() > MOST_PATTERN {
        return Some(format!("the pattern is empty or over {MOST_PATTERN} characters"));
    }
    if let Some(what) = script_only(pattern) {
        return Some(format!("the pattern uses {what}, which Bridge cannot read"));
    }
    if repeats_a_repeat(pattern) {
        return Some("the pattern repeats a group that itself repeats or chooses, which can hang Bridge".to_string());
    }
    // The engine refuses lookaround and backreferences itself, which is the subset wanted.
    let compiled = match RegexBuilder::new(pattern).size_limit(100_000).build() {
        Ok(compiled) => compiled,
        Err(why) => return Some(format!("the pattern does not compile: {}", first_line(&why.to_string()))),
    };
    if compiled.is_match("") {
        return Some("the pattern matches the empty string".to_string());
    }
    match compiled.find(words) {
        Some(found) if found.start() == 0 && found.end() == words.len() => None,
        _ => Some(format!("the pattern does not match all of \"{words}\"")),
    }
}

fn first_line(text: &str) -> &str {
    text.lines().rev().find(|line| !line.trim().is_empty()).unwrap_or(text).trim()
}

/// What in `pattern` this engine reads and a JavaScript one does not, or reads differently.
fn script_only(pattern: &str) -> Option<&'static str> {
    let mut chars = pattern.chars().peekable();
    let mut in_class = false;
    let mut before = ' ';
    while let Some(one) = chars.next() {
        match one {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    if "AzZpPhHRXGKQEea<>".contains(escaped) {
                        return Some("an escape that is not in JavaScript");
                    }
                }
            }
            '[' if in_class => return Some("a class inside a class"),
            '[' => {
                in_class = true;
                if chars.peek() == Some(&':') {
                    return Some("a POSIX class");
                }
            }
            ']' => in_class = false,
            '(' if !in_class && chars.peek() == Some(&'?') => {
                chars.next();
                if chars.peek() != Some(&':') {
                    return Some("a flag, a name or a lookaround after (?");
                }
            }
            '&' | '~' if in_class && before == one => return Some("a class operation"),
            _ => {}
        }
        before = one;
    }
    None
}

/// Whether a group that is repeated (`+`, `*` or `{`) holds a repeat or a choice: `(a+)+`, `(a*)*`,
/// `(a|aa)+`. A JavaScript engine backtracks through those without end on a near miss, and Bridge
/// runs every taught pattern on every message it draws. The engine here does not backtrack, so it
/// would never show it.
fn repeats_a_repeat(pattern: &str) -> bool {
    let mut chars = pattern.chars().peekable();
    let mut groups: Vec<bool> = Vec::new();
    let mut in_class = false;
    let mut after_open = false;
    while let Some(one) = chars.next() {
        let was_open = std::mem::take(&mut after_open);
        match one {
            '\\' => {
                chars.next();
            }
            '[' => in_class = true,
            ']' => in_class = false,
            _ if in_class => {}
            '(' => {
                groups.push(false);
                after_open = true;
            }
            ')' => {
                let held = groups.pop().unwrap_or(false);
                if held && matches!(chars.peek(), Some('+' | '*' | '{')) {
                    return true;
                }
                if held {
                    if let Some(parent) = groups.last_mut() {
                        *parent = true;
                    }
                }
            }
            '+' | '*' | '{' | '|' => {
                if let Some(group) = groups.last_mut() {
                    *group = true;
                }
            }
            // `?` after `(` opens `(?:`; anywhere else it makes the one before it optional.
            '?' if !was_open => {
                if let Some(group) = groups.last_mut() {
                    *group = true;
                }
            }
            _ => {}
        }
    }
    false
}

pub(crate) fn address_fault(address: &str) -> Option<String> {
    let rest = ["https://", "http://"]
        .iter()
        .find(|scheme| address.len() >= scheme.len() && address[..scheme.len()].eq_ignore_ascii_case(scheme))
        .map(|scheme| &address[scheme.len()..]);
    let Some(rest) = rest else {
        return Some("the address is not an http or https address".to_string());
    };
    if address.chars().any(|one| one.is_whitespace() || one.is_control()) {
        return Some("the address has a space or a control character".to_string());
    }
    if address.replace("{match}", "").contains(['{', '}']) {
        return Some("the address has braces other than {match}".to_string());
    }
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() {
        return Some("the address has no host".to_string());
    }
    if host.contains("{match}") {
        return Some("the matched words cannot be in the address's host".to_string());
    }
    None
}
