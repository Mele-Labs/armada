//! The few things a `package.json` is asked: its top-level members, and the strings in a value. **A scanner, not a
//! parser**: bytes enter Fleet as JSON only through `store` and `ipc`, and nothing here builds a value.

/// The members of the object `text` opens with, each as its key and the text of its value.
pub(crate) fn members(text: &str) -> Vec<(String, &str)> {
    let bytes = text.as_bytes();
    let Some(mut at) = text.find('{').map(|open| open + 1) else { return Vec::new() };
    let mut out = Vec::new();
    loop {
        while at < bytes.len() && (bytes[at].is_ascii_whitespace() || bytes[at] == b',') {
            at += 1;
        }
        if at >= bytes.len() || bytes[at] != b'"' {
            return out;
        }
        let Some((key, after)) = string_at(text, at) else { return out };
        at = after;
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        if at >= bytes.len() || bytes[at] != b':' {
            return out;
        }
        at += 1;
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        let start = at;
        at = value_end(text, at);
        out.push((key, text[start..at].trim()));
    }
}

/// Every string literal in `text`, in order: the elements of an array of strings.
pub(crate) fn strings(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = text[at..].find('"') {
        match string_at(text, at + found) {
            Some((one, after)) => {
                out.push(one);
                at = after;
            }
            None => break,
        }
    }
    out
}

/// The value of `key` among the top-level members of `text`.
pub(crate) fn member<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    members(text).into_iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The string literal opening at byte `open`, unescaped, and the byte after its closing quote.
fn string_at(text: &str, open: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = open + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'"' => return Some((String::from_utf8_lossy(&out).into_owned(), at + 1)),
            b'\\' if at + 1 < bytes.len() => {
                at += 1;
                out.push(match bytes[at] {
                    b'n' => b'\n',
                    b't' => b'\t',
                    other => other,
                });
            }
            other => out.push(other),
        }
        at += 1;
    }
    None
}

/// The byte after the value that starts at `start`: a string, a balanced object or array, or a bare word.
fn value_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut at = start;
    let mut depth = 0usize;
    while at < bytes.len() {
        match bytes[at] {
            b'"' => match string_at(text, at) {
                Some((_, after)) => {
                    at = after;
                    if depth == 0 {
                        return at;
                    }
                    continue;
                }
                None => return bytes.len(),
            },
            b'{' | b'[' => depth += 1,
            b'}' | b']' if depth == 0 => return at,
            b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return at + 1;
                }
            }
            b',' if depth == 0 => return at,
            _ => {}
        }
        at += 1;
    }
    at
}
