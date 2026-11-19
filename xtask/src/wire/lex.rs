//! A Rust source file as tokens: enough to read the items `crates/ipc/src` declares, and nothing else.
//!
//! No dependencies, like the rest of `xtask`: the gate runs on a checkout with nothing built, so
//! this reads the source the way `rules_enums` does rather than through `syn`.

/// One token. Brackets are plain `Punct`s; [`super::parse`] matches them.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// An identifier or keyword, or a lifetime with its tick (`'a`).
    Ident(String),
    Punct(char),
    /// A string literal's text. Escapes other than `\"` and `\\` are kept as written.
    Str(String),
    /// A number or a character literal.
    Lit,
    /// One `///` line, without the marker and one leading space.
    Doc(String),
}

pub fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '/' && c.get(i + 1) == Some(&'/') {
            let start = i;
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
            let line: String = c[start..i].iter().collect();
            if line.starts_with("///") && !line.starts_with("////") {
                let text = &line[3..];
                out.push(Tok::Doc(text.strip_prefix(' ').unwrap_or(text).to_string()));
            }
        } else if ch == '/' && c.get(i + 1) == Some(&'*') {
            let mut depth = 0;
            while i < c.len() {
                if c[i] == '/' && c.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
        } else if ch == '"' {
            let (text, next) = string(&c, i + 1)?;
            out.push(Tok::Str(text));
            i = next;
        } else if ch == '\'' {
            if c.get(i + 1) == Some(&'\\') {
                i += 2;
                while i < c.len() && c[i] != '\'' {
                    i += 1;
                }
                i += 1;
                out.push(Tok::Lit);
            } else if c.get(i + 2) == Some(&'\'') {
                i += 3;
                out.push(Tok::Lit);
            } else {
                let start = i;
                i += 1;
                while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                    i += 1;
                }
                out.push(Tok::Ident(c[start..i].iter().collect()));
            }
        } else if (ch == 'r' || ch == 'b') && raw_or_byte_string(&c, i).is_some() {
            let (text, next) = raw_or_byte_string(&c, i).unwrap_or_default();
            out.push(Tok::Str(text));
            i = next;
        } else if ch == 'r' && c.get(i + 1) == Some(&'#') && c.get(i + 2).is_some_and(|n| n.is_alphabetic()) {
            // A raw identifier, `r#type`, which serde writes without the prefix.
            i += 2;
            let start = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(c[start..i].iter().collect()));
        } else if ch.is_alphabetic() || ch == '_' {
            let start = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(c[start..i].iter().collect()));
        } else if ch.is_ascii_digit() {
            while i < c.len()
                && (c[i].is_alphanumeric()
                    || c[i] == '_'
                    || (c[i] == '.' && c.get(i + 1).is_some_and(|n| n.is_ascii_digit())))
            {
                i += 1;
            }
            out.push(Tok::Lit);
        } else {
            out.push(Tok::Punct(ch));
            i += 1;
        }
    }
    Ok(out)
}

/// The text of a plain string whose opening quote is before `i`, and where it ends.
fn string(c: &[char], mut i: usize) -> Result<(String, usize), String> {
    let mut text = String::new();
    while i < c.len() {
        match c[i] {
            '"' => return Ok((text, i + 1)),
            '\\' => {
                match c.get(i + 1) {
                    Some('"') => text.push('"'),
                    Some('\\') => text.push('\\'),
                    Some(other) => {
                        text.push('\\');
                        text.push(*other);
                    }
                    None => break,
                }
                i += 2;
            }
            other => {
                text.push(other);
                i += 1;
            }
        }
    }
    Err("an unterminated string".to_string())
}

/// `r"…"`, `r#"…"#`, `b"…"`, `br#"…"#` and `b'x'`, where one starts at `i`.
fn raw_or_byte_string(c: &[char], i: usize) -> Option<(String, usize)> {
    let mut j = i;
    if c[j] == 'b' {
        j += 1;
        if c.get(j) == Some(&'\'') {
            let mut k = j + 1;
            while k < c.len() && c[k] != '\'' {
                k += if c[k] == '\\' { 2 } else { 1 };
            }
            return Some((String::new(), k + 1));
        }
        if c.get(j) == Some(&'"') {
            return string(c, j + 1).ok();
        }
    }
    if c.get(j) != Some(&'r') {
        return None;
    }
    j += 1;
    let mut hashes = 0;
    while c.get(j) == Some(&'#') {
        hashes += 1;
        j += 1;
    }
    if c.get(j) != Some(&'"') {
        return None;
    }
    j += 1;
    let start = j;
    while j < c.len() {
        if c[j] == '"' && (0..hashes).all(|h| c.get(j + 1 + h) == Some(&'#')) {
            return Some((c[start..j].iter().collect(), j + 1 + hashes));
        }
        j += 1;
    }
    None
}
