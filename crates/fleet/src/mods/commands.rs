//! What a Commands mod's `commands.toml` may say: rows for Bridge's command palette, each doing
//! one of four things. Checked before Bridge reads it.
//!
//! **A row has no id of its own**: its name is what it is called, searched and chained by, so a
//! name is held to be unique. The blanks a row takes are read off its text here, once, so Bridge
//! never parses a prompt to find them.

use ipc::{CommandsFile, PaletteCommand};

/// The most `commands.toml` may weigh.
pub(crate) const MOST_BYTES: u64 = 16 * 1024;

/// Keys that are a word and not a character, spelled as the registry spells them.
const NAMED_KEYS: [&str; 14] = [
    "Esc", "Enter", "Tab", "Space", "Backspace", "Delete", "↑", "↓", "←", "→", "Home", "End", "PageUp", "PageDown",
];

/// What is wrong with a `{` in some text.
enum Brace {
    /// No `}` follows it.
    Unclosed,
    /// `{Pr}`: letters, digits and `_`, but not a lower case name that starts with a letter.
    Misspelled(String),
}

/// The blanks in `text`, once each in the order they first appear, and the first brace that is neither
/// a blank nor plain text. **A brace holding anything but letters, digits and `_` is text**, and so is one
/// right after a `$`, so a shell's `{a,b}`, `${HOME}` and `awk '{print $1}'` pass as they are.
fn blanks_of(text: &str) -> (Vec<String>, Option<Brace>) {
    let (mut blanks, mut fault) = (Vec::new(), None);
    for (at, _) in text.match_indices('{') {
        // `${HOME}` is a shell's own expansion.
        if text[..at].ends_with('$') {
            continue;
        }
        let after = &text[at + 1..];
        let Some(end) = after.find('}') else {
            fault.get_or_insert(Brace::Unclosed);
            continue;
        };
        let inner = &after[..end];
        if inner.is_empty() || !inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        if inner.starts_with(|c: char| c.is_ascii_lowercase()) && !inner.chars().any(|c| c.is_ascii_uppercase()) {
            if !blanks.iter().any(|one| one == inner) {
                blanks.push(inner.to_string());
            }
        } else {
            fault.get_or_insert(Brace::Misspelled(inner.to_string()));
        }
    }
    (blanks, fault)
}

/// The text a row's blanks are read from: its prompt, address or shell line.
fn text_of(one: &PaletteCommand) -> Option<&str> {
    one.prompt.as_deref().or(one.url.as_deref()).or(one.shell.as_deref())
}

/// `text` read, with each row's blanks filled in, or the problem with where it stopped.
pub(crate) fn read(text: &str) -> Result<CommandsFile, String> {
    let mut file: CommandsFile =
        toml::from_str(text).map_err(|why| crate::mods::toml_problem("commands.toml", text, &why))?;
    for one in &mut file.command {
        one.blanks = text_of(one).map(|said| blanks_of(said).0).unwrap_or_default();
    }
    Ok(file)
}

/// The line each `[[command]]` table opens on, in order.
fn lines_of(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with("[[command]]"))
        .map(|(at, _)| at + 1)
        .collect()
}

/// Why `hotkey` is not a chord in the registry's spelling, or `None`: modifiers in the order `⌃⌥⇧⌘`,
/// each at most once, then one key, which is a capital letter, a digit, a symbol, a named key or `F1` to `F99`.
fn hotkey_fault(hotkey: &str) -> Option<&'static str> {
    if hotkey.trim().is_empty() {
        return Some("an empty hotkey");
    }
    let mut key = hotkey;
    for modifier in ['⌃', '⌥', '⇧', '⌘'] {
        key = key.strip_prefix(modifier).unwrap_or(key);
    }
    let one_char = key.chars().count() == 1;
    let plain = one_char
        && key.chars().all(|c| !c.is_whitespace() && !c.is_control() && !"⌃⌥⇧⌘".contains(c) && !c.is_ascii_lowercase());
    let function = key.strip_prefix('F').is_some_and(|n| (1..=2).contains(&n.len()) && n.chars().all(|c| c.is_ascii_digit()));
    if plain || function || NAMED_KEYS.contains(&key) {
        return None;
    }
    Some("a hotkey that is not a chord: modifiers in the order ⌃⌥⇧⌘, then one key, a letter in capitals")
}

/// Why `address` cannot be opened once its blanks are taken out, or `None`: an http or https address with
/// no space, and a host of its own that holds no blank.
fn address_fault(address: &str) -> Option<&'static str> {
    let lower = address.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://")) else {
        return Some("an address that is not http or https");
    };
    if address.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Some("an address with a space or a control character");
    }
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() {
        return Some("an address with no host");
    }
    if host.contains(['{', '}']) {
        return Some("a blank in the address's host");
    }
    None
}

/// `name` as a palette id spells it: lower case, each run of anything but letters and digits one `-`, none at either end.
fn slug_of(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_string()
}

/// The line of a `blanks` key in the `[[command]]` table that opens on line `open` and ends before `close`.
fn blanks_line(text: &str, open: usize, close: usize) -> Option<usize> {
    text.lines()
        .enumerate()
        .skip(open)
        .take(close.saturating_sub(open + 1))
        .find(|(_, line)| line.trim_start().strip_prefix("blanks").is_some_and(|rest| rest.trim_start().starts_with('=')))
        .map(|(at, _)| at + 1)
}

/// Every problem in `text`, empty where Bridge may act on it. Each names its row and the line it opens on.
pub(crate) fn problems(text: &str, mod_name: &str) -> Vec<String> {
    let file = match read(text) {
        Ok(file) => file,
        Err(why) => return vec![why],
    };
    if file.command.is_empty() {
        return vec!["commands.toml: it needs at least one `[[command]]`".to_string()];
    }
    let opens = lines_of(text);
    let raw = toml::from_str::<toml::Table>(text)
        .ok()
        .and_then(|mut table| match table.remove("command") {
            Some(toml::Value::Array(rows)) => Some(rows.into_iter().filter_map(|row| row.try_into::<toml::Table>().ok()).collect::<Vec<_>>()),
            _ => None,
        })
        .unwrap_or_default();
    let mut found = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut aliases: Vec<String> = Vec::new();
    for (at, one) in file.command.iter().enumerate() {
        let on = opens.get(at).map(|line| format!(" on line {line}")).unwrap_or_default();
        if raw.get(at).is_some_and(|row| row.contains_key("blanks")) {
            let line = opens.get(at).and_then(|open| blanks_line(text, *open, opens.get(at + 1).copied().unwrap_or(usize::MAX)));
            let on = line.or(opens.get(at).copied()).map(|line| format!(" on line {line}")).unwrap_or_default();
            found.push(format!("commands.toml: `{}` sets `blanks`, which Fleet reads off the text,{on}", one.name.trim()));
        }
        let mut say = |said: String| found.push(format!("commands.toml: {said}{on}"));
        let name = one.name.trim();
        if name.is_empty() {
            say(format!("command {} has no name", at + 1));
            continue;
        }
        if names.iter().any(|seen| seen.eq_ignore_ascii_case(name)) {
            say(format!("`{name}` is named twice"));
        }
        names.push(name.to_string());

        let texts = [("prompt", one.prompt.as_deref()), ("url", one.url.as_deref()), ("shell", one.shell.as_deref())];
        let set = texts.iter().filter(|(_, said)| said.is_some()).count() + usize::from(one.chain.is_some());
        if set == 0 {
            say(format!("`{name}` does nothing: it needs one of prompt, url, shell or chain"));
        } else if set > 1 {
            say(format!("`{name}` does more than one of prompt, url, shell and chain"));
        }
        for (word, said) in texts {
            let Some(said) = said else { continue };
            if said.trim().is_empty() {
                say(format!("`{name}` has an empty {word}"));
                continue;
            }
            match blanks_of(said).1 {
                Some(Brace::Unclosed) => say(format!("`{name}` has a `{{` with no `}}`")),
                Some(Brace::Misspelled(inner)) => say(format!(
                    "`{name}` has `{{{inner}}}`, and a blank is lower case letters, digits and `_` starting with a letter"
                )),
                None => {}
            }
        }
        if let Some(why) = one.url.as_deref().filter(|said| !said.trim().is_empty()).and_then(address_fault) {
            say(format!("`{name}` has {why}"));
        }
        if let Some(chain) = &one.chain {
            if chain.is_empty() {
                say(format!("`{name}` has an empty chain"));
            }
            if chain.iter().any(|id| id.trim().is_empty()) {
                say(format!("`{name}` has an empty row id in its chain"));
            }
            if chain.iter().any(|id| id.contains(['{', '}'])) {
                say(format!("`{name}` takes no blanks in a chain"));
            }
            if chain.iter().any(|id| *id == format!("cmd:{mod_name}:{}", slug_of(name))) {
                say(format!("`{name}` chains itself"));
            }
        }
        for alias in &one.aliases {
            let alias = alias.trim();
            if alias.is_empty() {
                say(format!("`{name}` has an empty alias"));
            } else if aliases.iter().any(|seen| seen.eq_ignore_ascii_case(alias)) {
                say(format!("`{name}` has the alias `{alias}`, which another alias already has"));
            }
            aliases.push(alias.to_string());
        }
        if let Some(why) = one.hotkey.as_deref().and_then(hotkey_fault) {
            say(format!("`{name}` has {why}"));
        }
    }
    found
}
