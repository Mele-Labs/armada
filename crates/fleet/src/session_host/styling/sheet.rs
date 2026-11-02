//! Making a stylesheet usable by a sealed frame, as it is kept. A constructed stylesheet ignores
//! `@import`, reads no address, and knows no Tailwind, so each of those is dealt with here, in the
//! order the steps are numbered. `.armada/documents/hi-fi-sketches.html`, *How Bridge draws it*.

use std::path::{Path, PathBuf};

/// Most one stylesheet weighs once its imports are in.
pub(crate) const SHEET_AT_MOST: usize = 256 * 1024;
/// Most one file is read at all, so a stray bundle costs a stat and not a read.
const FILE_AT_MOST: u64 = 4 * 1024 * 1024;
/// Most imports deep one sheet goes.
const DEPTH_AT_MOST: usize = 16;

/// A `@font-face` of a sheet whose file is on disk inside the checkout.
pub(crate) struct Face {
    pub family: String,
    /// As written: `400`, or `100 900` for a variable font.
    pub weight: String,
    pub style: Option<String>,
    pub file: PathBuf,
}

pub(crate) struct Made {
    pub css: String,
    pub faces: Vec<Face>,
    /// Every file that went into `css`, canonical.
    pub read: Vec<PathBuf>,
}

/// `file` (canonical, inside `root`) as a sheet a frame can adopt, or why it cannot be.
pub(crate) fn make(root: &Path, file: &Path) -> Result<Made, String> {
    let mut loader = Loader { root, seen: Vec::new(), faces: Vec::new(), read: Vec::new(), weight: 0 };
    let css = loader.load(file, 0)?;
    let css = theme_as_root(&css);
    let css = drop_at_rules(&css, &TAILWIND_ONLY);
    let css = css.replace("--theme(", "var(");
    let css = drop_declarations_with(&css, &["url(", "image-set(", "expression("]);
    let css = squeezed(&css);
    if css.len() > SHEET_AT_MOST {
        return Err(format!(
            "it is {} bytes with its imports in, and a stylesheet is kept to at most {SHEET_AT_MOST}",
            css.len()
        ));
    }
    Ok(Made { css, faces: loader.faces, read: loader.read })
}

struct Loader<'a> {
    root: &'a Path,
    seen: Vec<PathBuf>,
    faces: Vec<Face>,
    read: Vec<PathBuf>,
    /// Bytes loaded so far across every import, so a tree of imports stops early.
    weight: usize,
}

impl Loader<'_> {
    fn load(&mut self, file: &Path, depth: usize) -> Result<String, String> {
        // A sheet imported twice, or in a loop, applies once.
        if depth > DEPTH_AT_MOST || self.seen.iter().any(|one| one == file) {
            return Ok(String::new());
        }
        self.seen.push(file.to_path_buf());
        let named = file.strip_prefix(self.root).unwrap_or(file).display().to_string();
        match std::fs::metadata(file) {
            Ok(meta) if meta.len() <= FILE_AT_MOST => {}
            _ => return Err(format!("{named} is not a file of at most {FILE_AT_MOST} bytes")),
        }
        let raw = std::fs::read_to_string(file).map_err(|why| format!("{named} cannot be read: {why}"))?;
        self.weight += raw.len();
        if self.weight > SHEET_AT_MOST * 4 {
            return Err(format!("{named} pulls in more than {} bytes of imports", SHEET_AT_MOST * 4));
        }
        self.read.push(file.to_path_buf());
        let dir = file.parent().unwrap_or(self.root);
        let bare = strip_comments(&raw);
        let bare = self.faces_out(&bare, dir);
        self.imports_in(&bare, dir, depth)
    }

    /// Every `@font-face` taken out, its local file noted. A face with no local file is dropped.
    fn faces_out(&mut self, css: &str, dir: &Path) -> String {
        let mut out = String::with_capacity(css.len());
        let mut rest = css;
        while let Some(at) = find_ci(rest, "@font-face") {
            out.push_str(&rest[..at]);
            let after = &rest[at..];
            let Some(open) = after.find('{') else { rest = ""; break };
            let close = block_end(after, open);
            let body = &after[open + 1..close.saturating_sub(1).max(open + 1)];
            if let Some(face) = self.face_of(body, dir) {
                if !self.faces.iter().any(|one| (&one.family, &one.weight, &one.style) == (&face.family, &face.weight, &face.style)) {
                    self.faces.push(face);
                }
            }
            rest = &after[close..];
        }
        out.push_str(rest);
        out
    }

    fn face_of(&self, body: &str, dir: &Path) -> Option<Face> {
        let mut family = None;
        let mut weight = None;
        let mut style = None;
        let mut src = None;
        for (name, value) in declarations(body) {
            match name.as_str() {
                "font-family" => family = Some(unquoted(&value)),
                "font-weight" => weight = Some(value.trim().to_string()),
                "font-style" => style = Some(value.trim().to_string()),
                "src" => src = Some(value),
                _ => {}
            }
        }
        let file = best_font_url(&src?).and_then(|address| self.local(dir, &address))?;
        Some(Face { family: family?, weight: weight.unwrap_or_else(|| String::from("400")), style, file })
    }

    /// `address` against `dir` as a file inside the checkout, or `None` for a remote or missing one.
    fn local(&self, dir: &Path, address: &str) -> Option<PathBuf> {
        let plain = address.split(['?', '#']).next()?;
        if plain.is_empty() || plain.starts_with("data:") || plain.starts_with("//") || plain.contains("://") {
            return None;
        }
        let found = std::fs::canonicalize(dir.join(plain)).ok()?;
        (found.starts_with(self.root) && found.is_file()).then_some(found)
    }

    /// Each `@import` replaced by the sheet it names, wrapped in the layer or media it asked for.
    fn imports_in(&mut self, css: &str, dir: &Path, depth: usize) -> Result<String, String> {
        let mut out = String::with_capacity(css.len());
        let mut rest = css;
        while let Some(at) = find_ci(rest, "@import") {
            out.push_str(&rest[..at]);
            let after = &rest[at + "@import".len()..];
            let end = statement_end(after);
            let statement = after[..end].trim();
            rest = after.get(end + 1..).unwrap_or("");
            let (address, qualifiers) = import_parts(statement);
            let Some(path) = self.resolved(dir, &address) else { continue };
            let inner = self.load(&path, depth + 1)?;
            out.push_str(&wrapped(&inner, qualifiers));
        }
        out.push_str(rest);
        Ok(out)
    }

    /// The stylesheet an `@import` names: beside the importing file, or in a `node_modules` between
    /// it and the checkout's root. Anything else, a remote address and `tailwindcss` itself, is `None`.
    fn resolved(&self, dir: &Path, address: &str) -> Option<PathBuf> {
        if address.contains("://") || address.starts_with("//") || address.starts_with("data:") {
            return None;
        }
        let mut candidates = vec![dir.join(address)];
        let mut up = Some(dir);
        while let Some(here) = up.filter(|here| here.starts_with(self.root)) {
            candidates.push(here.join("node_modules").join(address));
            up = here.parent();
        }
        candidates
            .into_iter()
            .filter_map(|one| std::fs::canonicalize(one).ok())
            .find(|one| one.starts_with(self.root) && one.is_file() && one.extension().is_some_and(|ext| ext == "css"))
    }
}

/// What `@import "x" layer(a) screen` asks of the sheet: `(address, "layer(a) screen")`.
fn import_parts(statement: &str) -> (String, &str) {
    let statement = statement.trim();
    if let Some(rest) = statement.strip_prefix("url(") {
        let close = rest.find(')').unwrap_or(rest.len());
        return (unquoted(&rest[..close]), rest.get(close + 1..).unwrap_or("").trim());
    }
    match statement.chars().next() {
        Some(quote @ ('"' | '\'')) => {
            let inner = &statement[1..];
            let close = inner.find(quote).unwrap_or(inner.len());
            (inner[..close].to_string(), inner.get(close + 1..).unwrap_or("").trim())
        }
        _ => (String::new(), ""),
    }
}

fn wrapped(inner: &str, qualifiers: &str) -> String {
    let mut layer = None;
    let mut media = String::new();
    let mut rest = qualifiers.trim();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("layer(") {
            let close = after.find(')').unwrap_or(after.len());
            layer = Some(after[..close].trim().to_string());
            rest = after.get(close + 1..).unwrap_or("").trim_start();
        } else if rest == "layer" || rest.starts_with("layer ") {
            layer = Some(String::new());
            rest = rest["layer".len()..].trim_start();
        } else if let Some(after) = rest.strip_prefix("supports(") {
            // Applied whether or not the frame supports it: a frame is a current browser.
            let close = block_end(&format!("({after}"), 0);
            rest = after.get(close.saturating_sub(1)..).unwrap_or("").trim_start();
        } else {
            media = rest.to_string();
            break;
        }
    }
    let mut css = inner.to_string();
    if !media.is_empty() {
        css = format!("@media {media} {{ {css} }}");
    }
    match layer {
        Some(name) => format!("@layer {name} {{ {css} }}"),
        None => css,
    }
}

/// Where the statement after `@import` ends: its `;`, outside quotes and parentheses.
fn statement_end(text: &str) -> usize {
    let (mut quote, mut depth) = (None, 0usize);
    for (at, ch) in text.char_indices() {
        match (quote, ch) {
            (Some(open), _) if open == ch => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, '(') => depth += 1,
            (None, ')') => depth = depth.saturating_sub(1),
            (None, ';') if depth == 0 => return at,
            _ => {}
        }
    }
    text.len()
}

/// The offset just past the `}` that closes the `{` at `open`, or the end of the text.
fn block_end(text: &str, open: usize) -> usize {
    let (mut quote, mut depth) = (None, 0usize);
    for (at, ch) in text.char_indices().skip_while(|(at, _)| *at < open) {
        match (quote, ch) {
            (Some(q), _) if q == ch => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, '{' | '(') => depth += 1,
            (None, '}' | ')') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return at + 1;
                }
            }
            _ => {}
        }
    }
    text.len()
}

/// `name: value` pairs of a block body, split at the `;` outside quotes and parentheses.
fn declarations(body: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = body;
    while !rest.trim().is_empty() {
        let end = statement_end(rest);
        if let Some((name, value)) = rest[..end].split_once(':') {
            found.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
        rest = rest.get(end + 1..).unwrap_or("");
    }
    found
}

/// The address of the best font file a `src:` lists: `woff2`, then `woff`, then the rest.
fn best_font_url(src: &str) -> Option<String> {
    let mut urls = Vec::new();
    let mut rest = src;
    while let Some(at) = find_ci(rest, "url(") {
        let after = &rest[at + 4..];
        let close = after.find(')').unwrap_or(after.len());
        urls.push(unquoted(&after[..close]));
        rest = after.get(close + 1..).unwrap_or("");
    }
    ["woff2", "woff", "ttf", "otf"]
        .iter()
        .find_map(|ext| urls.iter().find(|one| one.split(['?', '#']).next().is_some_and(|plain| plain.to_ascii_lowercase().ends_with(ext))))
        .or_else(|| urls.first())
        .cloned()
}

fn unquoted(value: &str) -> String {
    value.trim().trim_matches(|c| c == '"' || c == '\'').trim().to_string()
}

/// A case-insensitive `find` for an ASCII needle. Lower-casing ASCII keeps every byte offset.
pub(crate) fn find_ci(text: &str, needle: &str) -> Option<usize> {
    text.to_ascii_lowercase().find(&needle.to_ascii_lowercase())
}

/// The text without `/* ... */` comments, leaving strings alone.
pub(crate) fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(ch) = chars.next() {
        match quote {
            Some(open) => {
                out.push(ch);
                if ch == '\\' {
                    out.extend(chars.next());
                } else if ch == open {
                    quote = None;
                }
            }
            None if ch == '/' && chars.peek() == Some(&'*') => {
                chars.next();
                while let Some(skipped) = chars.next() {
                    if skipped == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    quote = Some(ch);
                }
                out.push(ch);
            }
        }
    }
    out
}

/// `@theme { ... }` is Tailwind's; as `:root { ... }` the same variables reach a plain frame.
fn theme_as_root(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(at) = find_ci(rest, "@theme") {
        let after = &rest[at + "@theme".len()..];
        let named = after.chars().next().is_some_and(|c| c.is_whitespace() || c == '{');
        let brace = after.find('{').filter(|_| named);
        out.push_str(&rest[..at]);
        match brace {
            Some(brace) => {
                out.push_str(":root ");
                rest = &after[brace..];
            }
            None => {
                out.push_str("@theme");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The at-rules only Tailwind reads. Without it they are rules a frame cannot make sense of.
const TAILWIND_ONLY: [&str; 10] =
    ["source", "plugin", "config", "reference", "tailwind", "apply", "custom-variant", "utility", "variant", "theme"];

/// Each `@name ...;` or `@name ... { ... }` of `names` removed. An `@apply` ends at the `}` of its
/// rule where it has no `;`.
fn drop_at_rules(css: &str, names: &[&str]) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    'scan: while let Some(at) = rest.find('@') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let word: String = after.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        if !names.contains(&word.to_ascii_lowercase().as_str()) {
            out.push('@');
            rest = after;
            continue;
        }
        let (mut quote, mut depth) = (None, 0usize);
        for (offset, ch) in after.char_indices() {
            match (quote, ch) {
                (Some(q), _) if q == ch => quote = None,
                (Some(_), _) => {}
                (None, '"' | '\'') => quote = Some(ch),
                (None, '(') => depth += 1,
                (None, ')') => depth = depth.saturating_sub(1),
                (None, ';') if depth == 0 => {
                    rest = &after[offset + 1..];
                    continue 'scan;
                }
                (None, '{') if depth == 0 => {
                    rest = &after[block_end(after, offset)..];
                    continue 'scan;
                }
                (None, '}') if depth == 0 => {
                    rest = &after[offset..];
                    continue 'scan;
                }
                _ => {}
            }
        }
        rest = "";
    }
    out.push_str(rest);
    out
}

/// Each declaration that holds one of `needles` removed whole, as the document page drops a
/// declaration that reaches for an address.
fn drop_declarations_with(css: &str, needles: &[&str]) -> String {
    let mut text = css.to_string();
    loop {
        let lower = text.to_ascii_lowercase();
        let Some(at) = needles.iter().filter_map(|needle| lower.find(needle)).min() else { return text };
        let start = text[..at].rfind([';', '{', '}']).map_or(0, |found| found + 1);
        let (mut quote, mut depth, mut end) = (None, 0usize, text.len());
        for (offset, ch) in text[at..].char_indices() {
            match (quote, ch) {
                (Some(q), _) if q == ch => quote = None,
                (Some(_), _) => {}
                (None, '"' | '\'') => quote = Some(ch),
                (None, '(') => depth += 1,
                (None, ')') => depth = depth.saturating_sub(1),
                (None, ';') if depth == 0 => {
                    end = at + offset + 1;
                    break;
                }
                (None, '}') if depth == 0 => {
                    end = at + offset;
                    break;
                }
                _ => {}
            }
        }
        text.replace_range(start..end, "");
    }
}

/// Runs of blank lines folded away, which comments leave behind.
fn squeezed(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut blank = false;
    for line in css.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            blank = true;
            continue;
        }
        if blank && !out.is_empty() {
            out.push('\n');
        }
        blank = false;
        out.push_str(line);
        out.push('\n');
    }
    out
}
