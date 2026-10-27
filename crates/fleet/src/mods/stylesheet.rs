//! What a theme's `theme.css` may say, checked before Bridge is given it.
//!
//! **A whitelist read by hand, not a CSS parser.** The accepted language is
//! `:root { --token: value; }` and `[data-theme] { ... }` and nothing else, which
//! is small enough that anything this reader does not understand is a problem
//! rather than a guess. A real parser would accept far more than a theme may say
//! and then need a second pass to take it away.

use std::collections::HashSet;
use std::sync::OnceLock;

/// The most a stylesheet may weigh. A full set of the design tokens is under 10 KiB.
pub(crate) const MOST_BYTES: u64 = 32 * 1024;

/// The most a value may run to. The longest token shipped is a shadow stack.
const MOST_VALUE: usize = 512;

/// How many problems are reported before the rest are counted.
const MOST_PROBLEMS: usize = 20;

/// Words that load or run something. A custom property holds them inertly, but
/// the value is read by whatever the theme is substituted into.
const FORBIDDEN: &[&str] = &[
    "url(",
    "image-set(",
    "image(",
    "src(",
    "element(",
    "expression(",
    "javascript:",
    "data:",
    "script",
];

/// Every design token this build ships, from the file `verify-tokens` writes.
fn tokens() -> &'static HashSet<&'static str> {
    static TOKENS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    TOKENS.get_or_init(|| {
        include_str!("../../../../packages/tokens/tokens.json")
            .lines()
            .filter_map(|line| line.trim().strip_prefix("\"name\": \""))
            .filter_map(|rest| rest.split('"').next())
            .collect()
    })
}

/// The least OKLab distance an `--accent` may keep from a status colour. A selected row is drawn
/// in the accent and a row in a status in that status's colour, so an accent nearer than this
/// cannot be told from the status at a glance. The old Dusk amber sat 0.037 from awaiting_review
/// and 0.068 from escalated; the lilac that replaced it is 0.110 from the nearest, rejected. The
/// default scaffold blue is 0.075 from running, which this still passes.
const LEAST_ACCENT_DISTANCE: f64 = 0.07;

/// Every status colour this build ships as a hex value, with its OKLab position, from the same
/// `tokens.json` (the generated copy of `packages/tokens/src/status.css`). Aliases and tints are
/// not hex, so they are not here.
fn status_colours() -> &'static Vec<(&'static str, &'static str, [f64; 3])> {
    static COLOURS: OnceLock<Vec<(&'static str, &'static str, [f64; 3])>> = OnceLock::new();
    COLOURS.get_or_init(|| {
        let mut name = "";
        let mut found = Vec::new();
        for line in include_str!("../../../../packages/tokens/tokens.json").lines().map(str::trim) {
            if let Some(rest) = line.strip_prefix("\"name\": \"") {
                name = rest.split('"').next().unwrap_or_default();
            } else if let Some(value) = line.strip_prefix("\"value\": \"").and_then(|rest| rest.split('"').next()) {
                if name.starts_with("--status-") && !name.ends_with("-bg") {
                    if let Some(lab) = oklab(value) {
                        found.push((name, value, lab));
                    }
                }
            }
        }
        found
    })
}

/// `#rgb` or `#rrggbb` as a point in OKLab. `None` for any other form of colour.
fn oklab(value: &str) -> Option<[f64; 3]> {
    let digits = value.strip_prefix('#')?;
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize| -> f64 {
        let byte = match digits.len() {
            3 => u8::from_str_radix(&digits[at..=at], 16).map(|n| n * 17),
            _ => u8::from_str_radix(&digits[at * 2..at * 2 + 2], 16),
        };
        let c = f64::from(byte.unwrap_or(0)) / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    if digits.len() != 3 && digits.len() != 6 {
        return None;
    }
    let (r, g, b) = (channel(0), channel(1), channel(2));
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    Some([
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ])
}

/// What is wrong with `value` as an `--accent`: the status colour it sits too near, where it is a
/// hex colour. Another form of colour is not measured.
fn accent_problem(value: &str) -> Option<String> {
    let accent = oklab(value)?;
    let (name, shipped, distance) = status_colours()
        .iter()
        .map(|(name, shipped, lab)| (*name, *shipped, accent.iter().zip(lab).map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt()))
        .min_by(|a, b| a.2.total_cmp(&b.2))?;
    (distance < LEAST_ACCENT_DISTANCE).then(|| {
        format!(
            "{value} is too close to `{name}` {shipped} (OKLab distance {distance:.3}, the least allowed is {LEAST_ACCENT_DISTANCE}). A selected row and a row in that status would look alike. Choose an accent at least that far from every status colour"
        )
    })
}

/// Every problem in `css`, in the order they appear. Empty means Bridge may inject it.
pub(crate) fn problems(css: &str) -> Vec<String> {
    problems_in(css, "theme.css")
}

/// The same checks for a stylesheet called `file`, which its problems name: `tweak.css` holds the
/// same token declarations a theme does.
pub(crate) fn problems_in(css: &str, file: &'static str) -> Vec<String> {
    let mut found = Found { file, ..Found::default() };
    let Some(plain) = without_comments(css, &mut found) else {
        return found.said;
    };
    if let Some(at) = plain.find('\\') {
        found.add(&plain, at, "a backslash cannot be read safely, so it is not accepted");
    }
    let mut at = 0;
    while at < plain.len() && !found.full() {
        at = statement(&plain, at, &mut found);
    }
    found.finish()
}

#[derive(Default)]
struct Found {
    file: &'static str,
    said: Vec<String>,
    more: usize,
}

impl Found {
    fn add(&mut self, text: &str, at: usize, said: &str) {
        if self.said.len() < MOST_PROBLEMS {
            let line = text[..at.min(text.len())].matches('\n').count() + 1;
            self.said.push(format!("{} line {line}: {said}", self.file));
        } else {
            self.more += 1;
        }
    }

    fn full(&self) -> bool {
        self.more > 0
    }

    fn finish(mut self) -> Vec<String> {
        if self.more > 0 {
            self.said.push(format!("and {} more", self.more));
        }
        self.said
    }
}

/// `css` with every comment blanked and its newlines kept, so a line number
/// still names the line the author wrote. `None` where a comment never ends.
fn without_comments(css: &str, found: &mut Found) -> Option<String> {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(open) = rest.find("/*") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let Some(close) = after.find("*/") else {
            found.add(css, css.len() - rest.len() + open, "a comment is never closed");
            return None;
        };
        let blanked: String = after[..close]
            .chars()
            .map(|c| if c == '\n' { '\n' } else { ' ' })
            .collect();
        out.push_str("  ");
        out.push_str(&blanked);
        out.push_str("  ");
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    Some(out)
}

/// One selector and its block, or one thing that is neither. Answers where the
/// next begins.
fn statement(text: &str, from: usize, found: &mut Found) -> usize {
    let start = from + (text[from..].len() - text[from..].trim_start().len());
    if start >= text.len() {
        return text.len();
    }
    let head_end = text[start..]
        .find(['{', ';', '}'])
        .map_or(text.len(), |at| start + at);
    let head = text[start..head_end].trim();
    if text[start..].starts_with('@') {
        let name = head.split_whitespace().next().unwrap_or("@");
        let said = match name {
            "@import" => "@import is not accepted: a theme cannot load anything".to_string(),
            _ => format!("{name} is not accepted: a theme holds token values and nothing else"),
        };
        found.add(text, start, &said);
        return after_statement(text, head_end);
    }
    if text[head_end..].chars().next() != Some('{') {
        found.add(text, start, &format!("`{head}` is outside a block"));
        return (head_end + 1).min(text.len());
    }
    if !selector_is_allowed(head) {
        found.add(
            text,
            start,
            &format!("`{head}` is not a selector a theme may use; only :root and [data-theme] blocks are read"),
        );
    }
    let body_start = head_end + 1;
    let Some(close) = text[body_start..].find('}').map(|at| body_start + at) else {
        found.add(text, start, "a block is never closed");
        return text.len();
    };
    if let Some(inner) = text[body_start..close].find('{') {
        found.add(text, body_start + inner, "a block inside a block is not accepted");
        return after_statement(text, head_end);
    }
    declarations(text, body_start, close, found);
    close + 1
}

/// Past the `;` or the balanced `{ }` that ends a statement being skipped.
fn after_statement(text: &str, from: usize) -> usize {
    let mut depth = 0usize;
    for (offset, c) in text[from..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' if depth <= 1 => return from + offset + 1,
            '}' => depth -= 1,
            ';' if depth == 0 => return from + offset + 1,
            _ => {}
        }
    }
    text.len()
}

fn selector_is_allowed(head: &str) -> bool {
    !head.is_empty()
        && head.split(',').all(|one| {
            let one = one.trim();
            one == ":root" || one == "[data-theme]" || theme_attribute(one.strip_prefix(":root").unwrap_or(one))
        })
}

/// `[data-theme="name"]`, in either quote or none, with `name` a slug.
fn theme_attribute(selector: &str) -> bool {
    let Some(inner) = selector
        .strip_prefix("[data-theme=")
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return false;
    };
    let name = inner.trim_matches(|c| c == '"' || c == '\'');
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn declarations(text: &str, from: usize, to: usize, found: &mut Found) {
    let mut at = from;
    for piece in text[from..to].split(';') {
        let begins = at;
        at += piece.len() + 1;
        let trimmed = piece.trim();
        if trimmed.is_empty() {
            continue;
        }
        let begins = begins + (piece.len() - piece.trim_start().len());
        let Some((property, value)) = trimmed.split_once(':') else {
            found.add(text, begins, &format!("`{trimmed}` is not a `--token: value` declaration"));
            continue;
        };
        let property = property.trim();
        if !tokens().contains(property) {
            found.add(text, begins, &format!("`{property}` is not a design token"));
            continue;
        }
        let value = value.trim();
        let why = value_problem(value).or_else(|| if property == "--accent" { accent_problem(value) } else { None });
        if let Some(why) = why {
            found.add(text, begins, &format!("`{property}`: {why}"));
        }
    }
}

fn value_problem(value: &str) -> Option<String> {
    if value.is_empty() {
        return Some("the value is empty".to_string());
    }
    if value.len() > MOST_VALUE {
        return Some(format!("the value is longer than {MOST_VALUE} characters"));
    }
    let lower = value.to_ascii_lowercase();
    if let Some(word) = FORBIDDEN.iter().find(|word| lower.contains(*word)) {
        return Some(format!("`{word}` is not accepted in a value"));
    }
    let outside = |c: char| {
        !(c.is_ascii_alphanumeric() || " #.,%+-*/()_'\":".contains(c))
    };
    value
        .chars()
        .find(|c| outside(*c))
        .map(|c| format!("`{c}` is not accepted in a value"))
}
