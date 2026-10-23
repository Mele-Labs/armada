//! The design tokens a teaching on Bridge's interface can name: their shipped values, which tokens
//! are aliases of which, and the one-line description the tokens file gives some of them.
//!
//! Read from `packages/tokens/tokens.json`, compiled in as the theme checks are, so a token added
//! there is known here by the next build. **No component is named**: the file lists none, so the
//! places a token is used are the aliases that move with it.

use std::sync::OnceLock;

/// One token as the tokens file has it.
pub(crate) struct Token {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) alias_of: Option<String>,
    pub(crate) note: Option<String>,
}

/// The text of a JSON string value after `"key": `, unescaped, or `None` for `null` or another key.
fn field(line: &str, key: &str) -> Option<Option<String>> {
    let rest = line.trim().strip_prefix(&format!("\"{key}\": "))?.trim_end_matches(',');
    if rest == "null" {
        return Some(None);
    }
    let inner = rest.strip_prefix('"')?.strip_suffix('"')?;
    Some(Some(inner.replace("\\\"", "\"").replace("\\\\", "\\")))
}

pub(crate) fn all() -> &'static [Token] {
    static TOKENS: OnceLock<Vec<Token>> = OnceLock::new();
    TOKENS.get_or_init(|| {
        let mut read: Vec<Token> = Vec::new();
        for line in include_str!("../../../../packages/tokens/tokens.json").lines() {
            if let Some(Some(name)) = field(line, "name") {
                read.push(Token { name, value: String::new(), alias_of: None, note: None });
            } else if let Some(last) = read.last_mut() {
                if let Some(Some(value)) = field(line, "value") {
                    last.value = value;
                } else if let Some(alias) = field(line, "aliasOf") {
                    last.alias_of = alias;
                } else if let Some(note) = field(line, "note") {
                    last.note = note;
                }
            }
        }
        read
    })
}

pub(crate) fn named(name: &str) -> Option<&'static Token> {
    all().iter().find(|token| token.name == name)
}

/// The tokens that are aliases of `name`, which change with it.
pub(crate) fn aliases_of(name: &str) -> Vec<String> {
    all().iter().filter(|token| token.alias_of.as_deref() == Some(name)).map(|token| token.name.clone()).collect()
}

/// Every token that is not an alias, one a line with its shipped value and description, for the
/// model to choose from.
pub(crate) fn listed() -> String {
    all()
        .iter()
        .filter(|token| token.alias_of.is_none())
        .map(|token| match &token.note {
            Some(note) => format!("{} {} ({note})", token.name, token.value),
            None => format!("{} {}", token.name, token.value),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The layout registry on one line a region, for the model to name ids from.
pub(crate) fn layout_registry() -> String {
    include_str!("../../../../packages/shell/layout-registry.json").split_whitespace().collect::<Vec<_>>().join(" ")
}
