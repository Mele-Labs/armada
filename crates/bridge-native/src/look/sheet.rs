//! Tokens laid over each other the way the CSS cascade lays them, and resolved to values.
//!
//! A theme is the defaults with its own block over them. `var()` is resolved after the layering,
//! on the merged set, which is what a browser does on the element that carries `data-theme`: an
//! alias a theme does not mention follows the colour the theme did change. That is why
//! `--surface-card` is light under the light theme with no rule of its own.

use std::collections::BTreeMap;
use std::time::Duration;

use super::value::{self, Color, Shadow};

/// The deepest chain of `var()` followed before a value is called circular.
const MOST_DEPTH: usize = 24;

/// Declarations from several sources, later over earlier. Holds every property written, custom or not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sheet {
    declared: BTreeMap<String, String>,
}

impl Sheet {
    pub fn new() -> Sheet {
        Sheet::default()
    }

    /// Lay `layer` over what is held. A name already held takes the layer's value.
    pub fn lay(mut self, layer: BTreeMap<String, String>) -> Sheet {
        self.declared.extend(layer);
        self
    }

    pub fn declared(&self, name: &str) -> Option<&str> {
        self.declared.get(name).map(String::as_str)
    }

    /// The custom properties held, `--name`, in name order.
    pub fn tokens(&self) -> impl Iterator<Item = &str> {
        self.declared.keys().map(String::as_str).filter(|name| name.starts_with("--"))
    }

    /// The text of `name` with every `var()` replaced by what it names, or why it cannot be.
    pub fn expand(&self, name: &str) -> Result<String, Unresolved> {
        self.expand_at(name, &mut Vec::new())
    }

    fn expand_at(&self, name: &str, chain: &mut Vec<String>) -> Result<String, Unresolved> {
        if chain.iter().any(|seen| seen == name) || chain.len() >= MOST_DEPTH {
            return Err(Unresolved::Circular(name.to_string()));
        }
        let Some(text) = self.declared.get(name) else {
            return Err(Unresolved::Missing(name.to_string()));
        };
        chain.push(name.to_string());
        let out = self.substitute(text, chain);
        chain.pop();
        out
    }

    fn substitute(&self, text: &str, chain: &mut Vec<String>) -> Result<String, Unresolved> {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(at) = rest.find("var(") {
            out.push_str(&rest[..at]);
            let after = &rest[at + 4..];
            let Some(close) = closing_paren(after) else {
                return Err(Unresolved::Unreadable(text.to_string()));
            };
            let inner = &after[..close];
            let (name, fallback) = match inner.split_once(',') {
                Some((name, fallback)) => (name.trim(), Some(fallback.trim())),
                None => (inner.trim(), None),
            };
            let value = match (self.expand_at(name, chain), fallback) {
                (Ok(found), _) => found,
                (Err(Unresolved::Missing(_)), Some(fallback)) => self.substitute(fallback, chain)?,
                (Err(why), _) => return Err(why),
            };
            out.push_str(&value);
            rest = &after[close + 1..];
        }
        out.push_str(rest);
        Ok(out)
    }
}

/// The offset of the parenthesis closing a call whose opening one precedes `text`.
fn closing_paren(text: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (at, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// Why a token has no value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unresolved {
    /// A `var()` names a property nothing declares and gives no fallback.
    Missing(String),
    Circular(String),
    /// A `var(` with no closing parenthesis.
    Unreadable(String),
}

impl std::fmt::Display for Unresolved {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unresolved::Missing(name) => write!(out, "names {name}, which nothing declares"),
            Unresolved::Circular(name) => write!(out, "{name} refers back to itself"),
            Unresolved::Unreadable(text) => write!(out, "has an unclosed var() in {text:?}"),
        }
    }
}

/// Every token of a sheet, each in the one type its text reads as.
///
/// A token that reads as no type below (`1px solid var(--accent)`, a font stack, a shadow, an
/// easing) is kept in `text` with its `var()` replaced. A token that cannot be expanded at all is
/// in `unresolved` and nowhere else.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resolved {
    pub colors: BTreeMap<String, Color>,
    pub lengths: BTreeMap<String, f32>,
    pub durations: BTreeMap<String, Duration>,
    pub numbers: BTreeMap<String, f32>,
    pub percents: BTreeMap<String, f32>,
    pub text: BTreeMap<String, String>,
    pub unresolved: BTreeMap<String, Unresolved>,
}

impl Resolved {
    pub fn of(sheet: &Sheet) -> Resolved {
        let mut resolved = Resolved::default();
        for name in sheet.tokens() {
            let name = name.to_string();
            match sheet.expand(&name) {
                Err(why) => {
                    resolved.unresolved.insert(name, why);
                }
                Ok(text) => resolved.classify(name, text),
            }
        }
        resolved
    }

    fn classify(&mut self, name: String, text: String) {
        if let Some(color) = value::parse_color(&text) {
            self.colors.insert(name, color);
        } else if let Some(length) = value::parse_px(&text) {
            self.lengths.insert(name, length);
        } else if let Some(duration) = value::parse_duration(&text) {
            self.durations.insert(name, duration);
        } else if let Some(percent) = value::parse_percent(&text) {
            self.percents.insert(name, percent);
        } else if let Some(number) = value::parse_number(&text) {
            self.numbers.insert(name, number);
        } else {
            self.text.insert(name, text);
        }
    }

    /// For each token that could not be resolved, take the value `base` has for it, if it has one.
    /// The token stays listed in `unresolved`, so the failure is still reported; it is only never missing.
    pub fn fill_from(&mut self, base: &Resolved) {
        for name in self.unresolved.keys() {
            if let Some(color) = base.colors.get(name) {
                self.colors.insert(name.clone(), *color);
            } else if let Some(length) = base.lengths.get(name) {
                self.lengths.insert(name.clone(), *length);
            } else if let Some(duration) = base.durations.get(name) {
                self.durations.insert(name.clone(), *duration);
            } else if let Some(percent) = base.percents.get(name) {
                self.percents.insert(name.clone(), *percent);
            } else if let Some(number) = base.numbers.get(name) {
                self.numbers.insert(name.clone(), *number);
            } else if let Some(text) = base.text.get(name) {
                self.text.insert(name.clone(), text.clone());
            }
        }
    }

    /// Whether a token is held under some type.
    pub fn knows(&self, name: &str) -> bool {
        self.colors.contains_key(name)
            || self.lengths.contains_key(name)
            || self.durations.contains_key(name)
            || self.percents.contains_key(name)
            || self.numbers.contains_key(name)
            || self.text.contains_key(name)
    }

    pub fn easing(&self, name: &str) -> Option<[f32; 4]> {
        value::parse_bezier(self.text.get(name)?)
    }

    pub fn font_stack(&self, name: &str) -> Option<Vec<String>> {
        Some(value::parse_font_stack(self.text.get(name)?))
    }

    pub fn shadows(&self, name: &str) -> Option<Vec<Shadow>> {
        value::parse_shadows(self.text.get(name)?)
    }

    /// Tokens held as text that no typed accessor reads, such as `0.04em`. For a report of what a
    /// screen cannot yet ask for.
    pub fn compound(&self) -> Vec<&str> {
        self.text
            .keys()
            .map(String::as_str)
            .filter(|name| self.easing(name).is_none() && self.shadows(name).is_none() && value::parse_edge(&self.text[*name]).is_none() && !name.starts_with("--font-"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(pairs: &[(&str, &str)]) -> Sheet {
        Sheet::new().lay(pairs.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect())
    }

    #[test]
    fn a_var_chain_is_followed_to_the_end() {
        let sheet = sheet(&[
            ("--status-awaiting-review", "#E5A93D"),
            ("--status-piloted", "var(--status-awaiting-review)"),
            ("--step-waiting", "var(--status-piloted)"),
        ]);
        assert_eq!(sheet.expand("--step-waiting").as_deref(), Ok("#E5A93D"));
    }

    #[test]
    fn a_var_inside_a_color_mix_is_replaced_in_place() {
        let sheet = sheet(&[
            ("--accent", "#4A9EDB"),
            ("--accent-faint", "color-mix(in srgb, var(--accent) 10%, transparent)"),
        ]);
        assert_eq!(sheet.expand("--accent-faint").as_deref(), Ok("color-mix(in srgb, #4A9EDB 10%, transparent)"));
        let resolved = Resolved::of(&sheet);
        assert!((resolved.colors["--accent-faint"].a - 0.10).abs() < 0.001);
    }

    #[test]
    fn a_theme_over_the_defaults_moves_every_alias_that_follows_the_token_it_changed() {
        let defaults = [
            ("--bg-raised", "#161C23"),
            ("--surface-card", "var(--bg-raised)"),
            ("--accent", "#4A9EDB"),
            ("--accent-faint", "color-mix(in srgb, var(--accent) 10%, transparent)"),
        ];
        let over: BTreeMap<String, String> =
            [("--bg-raised".to_string(), "#FFFFFF".to_string()), ("--accent".to_string(), "#C1B4EC".to_string())].into();
        let themed = sheet(&defaults).lay(over);
        let resolved = Resolved::of(&themed);
        assert_eq!(resolved.colors["--surface-card"], resolved.colors["--bg-raised"]);
        assert_eq!(resolved.colors["--surface-card"].r, 1.0);
        // The faint accent is the new accent at ten percent, not the old one.
        assert!((resolved.colors["--accent-faint"].r - 0xC1 as f32 / 255.0).abs() < 0.002);
    }

    #[test]
    fn a_fallback_is_used_only_where_the_property_is_missing() {
        let sheet = sheet(&[("--a", "var(--nope, #000000)"), ("--b", "#FFFFFF"), ("--c", "var(--b, #000000)")]);
        assert_eq!(sheet.expand("--a").as_deref(), Ok("#000000"));
        assert_eq!(sheet.expand("--c").as_deref(), Ok("#FFFFFF"));
    }

    #[test]
    fn a_missing_property_and_a_cycle_are_reported_by_token() {
        let sheet = sheet(&[("--a", "var(--nope)"), ("--x", "var(--y)"), ("--y", "var(--x)")]);
        let resolved = Resolved::of(&sheet);
        assert_eq!(resolved.unresolved["--a"], Unresolved::Missing("--nope".into()));
        assert!(matches!(resolved.unresolved["--x"], Unresolved::Circular(_)));
        assert!(!resolved.knows("--a"));
    }

    #[test]
    fn a_token_that_cannot_resolve_takes_the_base_value_and_is_still_reported() {
        let base = Resolved::of(&sheet(&[("--accent", "#4A9EDB")]));
        let mut themed = Resolved::of(&sheet(&[("--accent", "var(--nope)")]));
        assert!(!themed.knows("--accent"));
        themed.fill_from(&base);
        assert_eq!(themed.colors["--accent"], base.colors["--accent"]);
        assert!(themed.unresolved.contains_key("--accent"));
    }

    #[test]
    fn each_token_lands_in_the_type_its_text_reads_as() {
        let sheet = sheet(&[
            ("--c", "#112233"),
            ("--len", "28px"),
            ("--dur", "160ms"),
            ("--pct", "40%"),
            ("--n", "600"),
            ("--font-sans", "\"IBM Plex Sans\", system-ui"),
            ("--ease", "cubic-bezier(0.2, 0, 0, 1)"),
            ("--shadow", "0 1px 2px rgb(0 0 0 / 0.3)"),
            ("--edge", "2px solid var(--c)"),
            ("--tracking", "0.04em"),
        ]);
        let resolved = Resolved::of(&sheet);
        assert!(resolved.colors.contains_key("--c"));
        assert_eq!(resolved.lengths["--len"], 28.0);
        assert_eq!(resolved.durations["--dur"], Duration::from_millis(160));
        assert_eq!(resolved.percents["--pct"], 0.4);
        assert_eq!(resolved.numbers["--n"], 600.0);
        assert_eq!(resolved.font_stack("--font-sans").expect("a stack"), vec!["IBM Plex Sans", "system-ui"]);
        assert_eq!(resolved.easing("--ease"), Some([0.2, 0.0, 0.0, 1.0]));
        assert_eq!(resolved.shadows("--shadow").expect("one layer").len(), 1);
        assert_eq!(resolved.text["--edge"], "2px solid #112233");
        assert_eq!(resolved.compound(), vec!["--tracking"]);
    }
}
