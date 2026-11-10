//! A draft's markdown as the page of a Document: the skeleton `writing-a-document` asks for, one `<title>`
//! and one `<h1>` of the draft's title, then its text as `<h2>` sections, paragraphs, lists, quotes, code,
//! emphasis, links and simple tables.
//!
//! **Written here and not taken from a crate**, because the page allows a small subset (no script, style or
//! image, and a link only to an address that cannot run) and a general converter would have to be held to
//! it by filtering what it emits. Nothing here passes a raw tag through: every character of the draft is
//! escaped on the way out. A list is one level, an indented item joins it flat. Text that is not markdown
//! falls out as paragraphs.

/// The page for a draft titled `title`, whose text is `text`.
pub(crate) fn page_of(title: &str, text: &str) -> String {
    let plain = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let body = body_of(without_title(text, &plain));
    let title = escaped(&plain);
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>{title}</title>\n\
         <link rel=\"stylesheet\" href=\"armada-kit.css\">\n</head>\n<body>\n<h1>{title}</h1>\n{body}</body>\n</html>\n"
    )
}

/// The file name a title makes: lower case, each run of anything but letters and digits one `-`, at most 60
/// characters. A title with no letter or digit in it is `draft`.
pub(crate) fn slug_of(title: &str) -> String {
    let mut slug = String::new();
    for one in title.chars() {
        if one.is_ascii_alphanumeric() {
            slug.push(one.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.truncate(60);
    let slug = slug.trim_matches('-');
    if slug.is_empty() { "draft".to_string() } else { slug.to_string() }
}

/// The text without a first `# heading` that only says the title again, which the page's `<h1>` already does.
fn without_title<'a>(text: &'a str, title: &str) -> &'a str {
    let text = text.trim_start();
    let (first, rest) = text.split_once('\n').unwrap_or((text, ""));
    match heading_of(first.trim()) {
        Some((1, words)) if words.eq_ignore_ascii_case(title) => rest,
        _ => text,
    }
}

fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    text.chars().for_each(|one| push_escaped(&mut out, one));
    out
}

fn push_escaped(out: &mut String, one: char) {
    match one {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '"' => out.push_str("&quot;"),
        '\'' => out.push_str("&#39;"),
        other => out.push(other),
    }
}

/// `# words` to `###### words`: the level and the words. A `#` with no space after it is text.
fn heading_of(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|one| *one == '#').count();
    let words = line.get(level..)?;
    if level == 0 || level > 6 || !(words.is_empty() || words.starts_with(' ')) {
        return None;
    }
    // A closing run of `#` is cut only where a space sets it off, so `# C#` keeps its name.
    let words = words.trim();
    let cut = words.trim_end_matches('#');
    Some((level, if cut.is_empty() || cut.ends_with(' ') { cut.trim_end() } else { words }))
}

fn fence_of(line: &str) -> Option<&'static str> {
    ["```", "~~~"].into_iter().find(|fence| line.starts_with(fence))
}

fn is_rule(line: &str) -> bool {
    let chars: Vec<char> = line.chars().filter(|one| !one.is_whitespace()).collect();
    chars.len() >= 3 && ['-', '*', '_'].iter().any(|mark| chars.iter().all(|one| one == mark))
}

/// A list line's kind and what follows its marker: `(ordered, words)`.
fn item_of(line: &str) -> Option<(bool, &str)> {
    let line = line.trim_start();
    if let Some(words) = ["- ", "* ", "+ "].iter().find_map(|marker| line.strip_prefix(marker)) {
        return Some((false, words));
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let rest = line.get(digits..)?;
    let words = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "))?;
    (digits > 0 && digits <= 9).then_some((true, words))
}

fn starts_a_block(line: &str) -> bool {
    let line = line.trim();
    fence_of(line).is_some()
        || heading_of(line).is_some()
        || line.starts_with('>')
        || is_rule(line)
        || item_of(line).is_some()
}

fn cells_of(row: &str) -> Vec<&str> {
    let row = row.trim();
    let row = row.strip_prefix('|').unwrap_or(row);
    row.strip_suffix('|').unwrap_or(row).split('|').map(str::trim).collect()
}

fn is_separator(row: &str) -> bool {
    row.contains('-')
        && cells_of(row).iter().all(|cell| {
            let bar = cell.trim_matches(':');
            !bar.is_empty() && bar.chars().all(|one| one == '-')
        })
}

fn body_of(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    let mut at = 0;
    while at < lines.len() {
        let line = lines[at].trim();
        if line.is_empty() {
            at += 1;
        } else if let Some(fence) = fence_of(line) {
            let mut code = Vec::new();
            at += 1;
            while at < lines.len() && !lines[at].trim().starts_with(fence) {
                code.push(lines[at]);
                at += 1;
            }
            at += 1;
            out.push_str(&format!("<pre><code>{}</code></pre>\n", escaped(&code.join("\n"))));
        } else if let Some((level, words)) = heading_of(line) {
            let tag = if level <= 2 { "h2" } else { "h3" };
            out.push_str(&format!("<{tag}>{}</{tag}>\n", inline(words)));
            at += 1;
        } else if is_rule(line) {
            out.push_str("<hr>\n");
            at += 1;
        } else if line.contains('|') && lines.get(at + 1).is_some_and(|next| is_separator(next)) {
            at = table(&lines, at, &mut out);
        } else if line.starts_with('>') {
            let start = at;
            while at < lines.len() && lines[at].trim().starts_with('>') {
                at += 1;
            }
            let inner: Vec<&str> = lines[start..at].iter().map(|quoted| unquoted(quoted)).collect();
            out.push_str(&format!("<blockquote>\n{}</blockquote>\n", body_of(&inner.join("\n"))));
        } else if let Some((ordered, _)) = item_of(line) {
            at = list(&lines, at, ordered, &mut out);
        } else {
            let start = at;
            at += 1;
            while at < lines.len() && !lines[at].trim().is_empty() && !starts_a_block(lines[at]) {
                at += 1;
            }
            let words: Vec<&str> = lines[start..at].iter().map(|one| one.trim()).collect();
            out.push_str(&format!("<p>{}</p>\n", inline(&words.join("\n"))));
        }
    }
    out
}

/// A quoted line without its `>` and the space after it.
fn unquoted(line: &str) -> &str {
    let line = line.trim();
    let line = line.strip_prefix('>').unwrap_or(line);
    line.strip_prefix(' ').unwrap_or(line)
}

/// The table whose header is at `from`, as HTML, and the line after it.
fn table(lines: &[&str], from: usize, out: &mut String) -> usize {
    out.push_str("<table>\n<thead><tr>");
    for cell in cells_of(lines[from]) {
        out.push_str(&format!("<th>{}</th>", inline(cell)));
    }
    out.push_str("</tr></thead>\n<tbody>\n");
    let mut at = from + 2;
    while at < lines.len() && !lines[at].trim().is_empty() && lines[at].contains('|') {
        out.push_str("<tr>");
        for cell in cells_of(lines[at]) {
            out.push_str(&format!("<td>{}</td>", inline(cell)));
        }
        out.push_str("</tr>\n");
        at += 1;
    }
    out.push_str("</tbody>\n</table>\n");
    at
}

/// The list of one kind that starts at `from`, as HTML, and the line after it. An indented line that is
/// not an item goes on the item before it, and a blank line between items does not end the list.
fn list(lines: &[&str], from: usize, ordered: bool, out: &mut String) -> usize {
    let mut items: Vec<String> = Vec::new();
    let mut at = from;
    while at < lines.len() {
        let line = lines[at];
        match item_of(line) {
            Some((kind, words)) if kind == ordered => items.push(words.trim().to_string()),
            Some(_) => break,
            None if line.trim().is_empty() => {
                let next = lines.get(at + 1).and_then(|next| item_of(next));
                if !next.is_some_and(|(kind, _)| kind == ordered) {
                    break;
                }
            }
            None if line.starts_with([' ', '\t']) && !starts_a_block(line) => match items.last_mut() {
                Some(last) => {
                    last.push('\n');
                    last.push_str(line.trim());
                }
                None => break,
            },
            None => break,
        }
        at += 1;
    }
    let tag = if ordered { "ol" } else { "ul" };
    out.push_str(&format!("<{tag}>\n"));
    for item in items {
        out.push_str(&format!("<li>{}</li>\n", inline(&item)));
    }
    out.push_str(&format!("</{tag}>\n"));
    at
}

/// A line's words with `code`, `**strong**`, `*emphasis*`, `_emphasis_` and `[links](address)` as HTML, and
/// everything else escaped.
fn inline(text: &str) -> String {
    spans(&text.chars().collect::<Vec<_>>())
}

fn spans(text: &[char]) -> String {
    let mut out = String::new();
    let mut at = 0;
    while at < text.len() {
        let here = text[at];
        match here {
            '\\' if text.get(at + 1).is_some_and(char::is_ascii_punctuation) => {
                push_escaped(&mut out, text[at + 1]);
                at += 2;
            }
            '`' => {
                let run = text[at..].iter().take_while(|one| **one == '`').count();
                match code_end(text, at + run, run) {
                    Some(end) => {
                        let code: String = text[at + run..end].iter().collect();
                        out.push_str(&format!("<code>{}</code>", escaped(code.trim())));
                        at = end + run;
                    }
                    None => {
                        out.push_str(&"`".repeat(run));
                        at += run;
                    }
                }
            }
            '!' if text.get(at + 1) == Some(&'[') => match link_at(text, at + 1) {
                // An image is not allowed on the page, so it is its alt text.
                Some((label, _, end)) => {
                    out.push_str(&spans(label));
                    at = end;
                }
                None => {
                    out.push('!');
                    at += 1;
                }
            },
            '[' => match link_at(text, at) {
                Some((label, address, end)) => {
                    let words = spans(label);
                    match safe_address(&address) {
                        Some(href) => out.push_str(&format!("<a href=\"{}\">{words}</a>", escaped(href))),
                        None => out.push_str(&words),
                    }
                    at = end;
                }
                None => {
                    out.push('[');
                    at += 1;
                }
            },
            '*' | '_' => {
                let run = if text.get(at + 1) == Some(&here) { 2 } else { 1 };
                match emphasis_end(text, at, here, run) {
                    Some(end) => {
                        let tag = if run == 2 { "strong" } else { "em" };
                        out.push_str(&format!("<{tag}>{}</{tag}>", spans(&text[at + run..end])));
                        at = end + run;
                    }
                    None => {
                        out.push(here);
                        at += 1;
                    }
                }
            }
            other => {
                push_escaped(&mut out, other);
                at += 1;
            }
        }
    }
    out
}

/// Where a run of `run` backticks that closes the code span begins, searching from `from`.
fn code_end(text: &[char], from: usize, run: usize) -> Option<usize> {
    let mut at = from;
    while at < text.len() {
        if text[at] == '`' {
            let here = text[at..].iter().take_while(|one| **one == '`').count();
            if here == run && at > from {
                return Some(at);
            }
            at += here;
        } else {
            at += 1;
        }
    }
    None
}

/// Where the delimiter that closes the emphasis opened at `open` begins. An opener is followed by a
/// non-space and a closer follows one, a `_` is never inside a word, and the span is not empty.
fn emphasis_end(text: &[char], open: usize, mark: char, run: usize) -> Option<usize> {
    let inside = open + run;
    if text.get(inside).is_none_or(|one| one.is_whitespace()) {
        return None;
    }
    let word = |one: Option<&char>| one.is_some_and(|one| one.is_alphanumeric());
    if mark == '_' && open > 0 && word(text.get(open - 1)) {
        return None;
    }
    let mut at = inside;
    while at < text.len() {
        if text[at] == mark {
            let here = text[at..].iter().take_while(|one| **one == mark).count();
            let closes = at > inside && !text[at - 1].is_whitespace() && !(mark == '_' && word(text.get(at + here)));
            if closes && here == run {
                return Some(at);
            }
            // A longer run inside a single emphasis is a strong span's, not its end.
            at += here;
        } else {
            at += 1;
        }
    }
    None
}

/// `[label](address)` opening at `open`: the label, the address and where the link ends.
fn link_at(text: &[char], open: usize) -> Option<(&[char], String, usize)> {
    let mut depth = 0usize;
    let mut close = None;
    for (at, one) in text.iter().enumerate().skip(open) {
        match one {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(at);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    if text.get(close + 1) != Some(&'(') {
        return None;
    }
    let mut depth = 0usize;
    for (at, one) in text.iter().enumerate().skip(close + 1) {
        match one {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let inside: String = text[close + 2..at].iter().collect();
                    let address = inside.split_whitespace().next().unwrap_or_default().to_string();
                    return Some((&text[open + 1..close], address, at + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// The address where a link may keep it: `http`, `https`, `mailto`, a `#` anchor or a relative one. A
/// `javascript:` or `data:` address, or any other scheme, is dropped and the link is its words.
fn safe_address(address: &str) -> Option<&str> {
    let address = address.trim();
    if address.is_empty() {
        return None;
    }
    let scheme_ends = address.find(':').filter(|colon| !address[..*colon].contains(['/', '?', '#']));
    match scheme_ends {
        None => Some(address),
        Some(colon) => ["http", "https", "mailto"]
            .iter()
            .any(|scheme| address[..colon].eq_ignore_ascii_case(scheme))
            .then_some(address),
    }
}
