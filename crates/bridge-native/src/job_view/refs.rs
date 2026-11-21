//! What a thread's words name that the native app can open, as Electron's `refScanner` and
//! `linked` (packages/screens/src/prose-refs.ts, primitives/Prose/Prose.tsx): a Job by its id, its
//! `s-` prefix, its first eight characters or its exact title. **Found off the Jobs list, never off
//! the words' own claim**: a name that matches no Job stays text. A pull request is a link by `#N`
//! only where it is known in the Manifest, so an issue's number never opens a PR tab; `owner/name#N`
//! and a forge address are links as Electron's are. Issues, files, documents and walk links open in
//! panes and windows native does not draw yet, so they stay text until those do. A web address that
//! is a whole code span is a link, as Electron's `Code`.

use ipc::SessionRecord;

/// The scheme a Job reference travels through the markdown in, as Electron's `armada-ref:`.
const REF: &str = "armada-ref:session/";

/// The scheme a file path travels through the markdown in: a code span that is a whole path opens in a file tab.
const FILE: &str = "armada-ref:file/";

/// Each Job and the words that name it, and each pull request known in the Manifest by its number and address.
pub(super) struct Refs {
    jobs: Vec<(String, Vec<String>)>,
    pulls: Vec<(u64, String)>,
}

impl Refs {
    pub(super) fn of(records: &[SessionRecord]) -> Self {
        let jobs = records
            .iter()
            .map(|record| {
                let id = record.id.as_str().to_string();
                let prefix: String = id.chars().take(8).collect();
                let mut needles = vec![id.clone(), format!("s-{prefix}"), prefix];
                if let Some(title) = record.title.as_ref().filter(|title| title.chars().count() >= 4) {
                    needles.push(title.clone());
                }
                needles.retain(|needle| !needle.is_empty());
                (id, needles)
            })
            .collect();
        Refs { jobs, pulls: Vec::new() }
    }

    /// The pull requests known in the Manifest the words are read in: a `#N` names one of these, or nothing.
    pub(super) fn with_pulls(mut self, pulls: Vec<(u64, String)>) -> Self {
        self.pulls = pulls;
        self
    }

    /// Spans of `text` that name a Job, longest first where two overlap, then in order.
    fn scan(&self, text: &str) -> Vec<(usize, usize, &str)> {
        let mut found = Vec::new();
        for (id, needles) in &self.jobs {
            for needle in needles {
                let mut from = 0;
                while let Some(at) = text[from..].find(needle.as_str()) {
                    let start = from + at;
                    let end = start + needle.len();
                    if whole(text, start, end) {
                        found.push((start, end, id.as_str()));
                    }
                    from = start + needle.len().max(1);
                }
            }
        }
        kept_in_order(found)
    }

    /// Spans of `text` that name a Job or a pull request, each with the address its link goes to.
    fn targets(&self, text: &str) -> Vec<(usize, usize, String)> {
        let mut found: Vec<(usize, usize, String)> = self.scan(text).into_iter().map(|(start, end, id)| (start, end, format!("{REF}{id}"))).collect();
        found.extend(self.pull_spans(text));
        kept_in_order(found)
    }

    /// The address of a pull request known here by its number, and by its repository's `owner/name` where one is given.
    fn known(&self, number: u64, slug: Option<&str>) -> Option<&str> {
        self.pulls
            .iter()
            .find(|(known, address)| *known == number && slug.is_none_or(|slug| super::pulls::pull_address(address).is_some_and(|(held, _)| held.eq_ignore_ascii_case(slug))))
            .map(|(_, address)| address.as_str())
    }

    /// As Electron's `prose-refs.ts` finds them: `#N` that is a pull request known here, `owner/name#N` (its
    /// own where it is known, else the forge's page), and a forge address of a pull request or an issue.
    fn pull_spans(&self, text: &str) -> Vec<(usize, usize, String)> {
        let bytes = text.as_bytes();
        let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        let named = |b: u8| word(b) || b == b'.' || b == b'-';
        let mut found = Vec::new();
        for (hash, _) in text.match_indices('#') {
            let digits = bytes[hash + 1..].iter().take_while(|b| b.is_ascii_digit()).count();
            let end = hash + 1 + digits;
            if digits == 0 || bytes.get(end).is_some_and(|b| word(*b)) {
                continue;
            }
            let Ok(number) = text[hash + 1..end].parse::<u64>() else { continue };
            let before = hash.checked_sub(1).map(|at| bytes[at]);
            if before.is_none_or(|b| !word(b) && !b"&/#".contains(&b)) {
                if let Some(address) = self.known(number, None) {
                    found.push((hash, end, address.to_string()));
                }
                continue;
            }
            // `owner/name#N`: back over the name, a slash and the owner.
            let name = bytes[..hash].iter().rev().take_while(|b| named(**b)).count();
            let slash = hash - name;
            if name == 0 || slash == 0 || bytes[slash - 1] != b'/' {
                continue;
            }
            let owner = bytes[..slash - 1].iter().rev().take_while(|b| named(**b)).count();
            let start = slash - 1 - owner;
            if owner == 0 || start.checked_sub(1).is_some_and(|at| named(bytes[at]) || bytes[at] == b'/') {
                continue;
            }
            let slug = &text[start..hash];
            let address = self.known(number, Some(slug)).map_or_else(|| format!("https://github.com/{slug}/issues/{number}"), str::to_string);
            found.push((start, end, address));
        }
        for scheme in ["https://", "http://"] {
            for (start, _) in text.match_indices(scheme) {
                if start.checked_sub(1).is_some_and(|at| word(bytes[at]) || b"([<".contains(&bytes[at])) {
                    continue;
                }
                let span = text[start..].find(|c: char| c.is_whitespace() || "<>`)]".contains(c)).map_or(text.len(), |at| start + at);
                let Some(end) = forge_end(&text[start..span]).map(|at| start + at) else { continue };
                let address = &text[start..end];
                if self.pulls.iter().any(|(_, known)| known == address) || github_page(address) {
                    found.push((start, end, address.to_string()));
                }
            }
        }
        found
    }

    /// The Job a press on a link opens, where the link is one this file wrote.
    pub(super) fn job_of(address: &str) -> Option<String> {
        address.strip_prefix(REF).map(|id| id.to_string())
    }

    /// The file a press on a link opens, where the link is one this file wrote.
    /// A `:12` or `:12-20` line suffix is where to look, not part of the name, so it is dropped.
    pub(super) fn file_of(address: &str) -> Option<String> {
        let path = address.strip_prefix(FILE)?;
        let (name_at, name) = path.rsplit_once('/').map_or((0, path), |(dir, name)| (dir.len() + 1, name));
        let end = name.find(':').map_or(path.len(), |colon| name_at + colon);
        Some(path[..end].to_string())
    }

    /// The words with each Job they name written as a link, and code left alone: a code span that is exactly
    /// a Job's id, or a web address, is the link; a number or a name inside a fence or a span is not.
    pub(super) fn linked(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for (code, part) in code_parts(text) {
            if !code {
                out.push_str(&self.words(part));
                continue;
            }
            if part.starts_with("```") || part.starts_with("~~~") {
                out.push_str(part);
                continue;
            }
            let inner = &part[1..part.len() - 1];
            let job = self.scan(inner).into_iter().find(|one| one.0 == 0 && one.1 == inner.len()).map(|one| one.2);
            match job {
                Some(id) => out.push_str(&format!("[{part}]({REF}{id})")),
                None if is_web(inner) => out.push_str(&format!("[{part}](<{inner}>)")),
                None if is_path(inner) => out.push_str(&format!("[{part}](<{FILE}{inner}>)")),
                None => out.push_str(part),
            }
        }
        out
    }

    /// A stretch of words with no code in it: a link a model wrote stays one link, and the rest is scanned.
    fn words(&self, part: &str) -> String {
        let mut out = String::with_capacity(part.len());
        let mut rest = part;
        while let Some((before, link, after)) = written_link(rest) {
            out.push_str(&self.named(before));
            out.push_str(link);
            rest = after;
        }
        out.push_str(&self.named(rest));
        out
    }

    fn named(&self, part: &str) -> String {
        let mut out = String::with_capacity(part.len());
        let mut from = 0;
        for (start, end, target) in self.targets(part) {
            let said = part[start..end].replace('[', "\\[").replace(']', "\\]");
            out.push_str(&part[from..start]);
            // An address the words wrote whole goes in angle brackets, as a web address in code does.
            if part[start..end] == target {
                out.push_str(&format!("[{said}](<{target}>)"));
            } else {
                out.push_str(&format!("[{said}]({target})"));
            }
            from = end;
        }
        out.push_str(&part[from..]);
        out
    }
}

/// Spans that do not overlap, longest first where two do, then in order.
fn kept_in_order<T>(mut found: Vec<(usize, usize, T)>) -> Vec<(usize, usize, T)> {
    found.sort_by(|a, b| (b.1 - b.0).cmp(&(a.1 - a.0)).then(a.0.cmp(&b.0)));
    let mut kept: Vec<(usize, usize, T)> = Vec::new();
    for one in found {
        if kept.iter().all(|had| one.1 <= had.0 || one.0 >= had.1) {
            kept.push(one);
        }
    }
    kept.sort_by_key(|one| one.0);
    kept
}

/// Where the first `/pull/N` or `/issues/N` in `token` ends, where nothing that carries the address on follows it.
fn forge_end(token: &str) -> Option<usize> {
    let bytes = token.as_bytes();
    for (at, _) in token.match_indices('/') {
        let rest = &token[at..];
        let Some(path) = ["/pull/", "/issues/"].into_iter().find(|path| rest.starts_with(path)) else { continue };
        let from = at + path.len();
        let digits = bytes[from..].iter().take_while(|b| b.is_ascii_digit()).count();
        let end = from + digits;
        if digits > 0 && !bytes.get(end).is_some_and(|b| b.is_ascii_alphanumeric() || b"_/#?".contains(b)) {
            return Some(end);
        }
    }
    None
}

/// `https://github.com/owner/name/pull/N` or `/issues/N`, whole.
fn github_page(address: &str) -> bool {
    let Some(path) = address.strip_prefix("https://github.com/") else { return false };
    let parts: Vec<&str> = path.split('/').collect();
    let named = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c));
    parts.len() == 4 && named(parts[0]) && named(parts[1]) && ["pull", "issues"].contains(&parts[2]) && parts[3].chars().all(|c| c.is_ascii_digit())
}

/// Whether `text[start..end]` stands as a whole word: no letter, digit, `_` or `-` either side.
fn whole(text: &str, start: usize, end: usize) -> bool {
    let joins = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    !text[..start].chars().next_back().is_some_and(joins) && !text[end..].chars().next().is_some_and(joins)
}

/// A code span that is a whole file path: no spaces, a slash, and a file name with an extension. Fleet refuses one
/// that is not a file of the Job's repository, and the tab says so.
fn is_path(text: &str) -> bool {
    if text.contains(char::is_whitespace) || text.contains("://") || !text.contains('/') {
        return false;
    }
    let name = text.rsplit('/').next().unwrap_or("");
    let name = name.split(':').next().unwrap_or("");
    name.rsplit_once('.').is_some_and(|(stem, ext)| !stem.is_empty() && !ext.is_empty() && ext.len() <= 8 && ext.chars().all(|c| c.is_ascii_alphanumeric()))
}

fn is_web(text: &str) -> bool {
    (text.starts_with("https://") || text.starts_with("http://")) && text.len() > 8 && !text.chars().any(char::is_whitespace)
}

/// The text split into stretches of words and of code (a fenced block, or a span on one line), in order.
fn code_parts(text: &str) -> Vec<(bool, &str)> {
    let mut parts = Vec::new();
    let mut from = 0;
    let mut at = 0;
    let bytes = text.as_bytes();
    while at < text.len() {
        let rest = &text[at..];
        let fence = ["```", "~~~"].into_iter().find(|fence| rest.starts_with(fence));
        let end = if let Some(fence) = fence {
            rest[3..].find(fence).map(|close| at + 3 + close + 3)
        } else if bytes[at] == b'`' {
            rest[1..].find(['`', '\n']).filter(|close| rest.as_bytes()[1 + close] == b'`').map(|close| at + 1 + close + 1)
        } else {
            None
        };
        match end {
            Some(end) => {
                if from < at {
                    parts.push((false, &text[from..at]));
                }
                parts.push((true, &text[at..end]));
                at = end;
                from = end;
            }
            None => at += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    if from < text.len() {
        parts.push((false, &text[from..]));
    }
    parts
}

/// The first `[words](address)` a model wrote in `text`: what is before it, the link, and what is after.
fn written_link(text: &str) -> Option<(&str, &str, &str)> {
    let mut search = 0;
    while let Some(open) = text[search..].find('[').map(|at| search + at) {
        let start = if open > 0 && text.as_bytes()[open - 1] == b'!' { open - 1 } else { open };
        if let Some(close) = text[open..].find("](").map(|at| open + at) {
            if !text[open + 1..close].contains(['[', '\n']) {
                if let Some(end) = text[close + 2..].find(')').map(|at| close + 2 + at) {
                    if !text[close + 2..end].contains(char::is_whitespace) {
                        return Some((&text[..start], &text[start..=end], &text[end + 1..]));
                    }
                }
            }
        }
        search = open + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs() -> Refs {
        Refs { jobs: vec![("ad34a1b8-683f".into(), vec!["ad34a1b8-683f".into(), "s-ad34a1b8".into(), "ad34a1b8".into(), "Audit the docs index".into()])], pulls: Vec::new() }
    }

    #[test]
    fn a_pull_request_it_knows_is_a_link_by_number_name_or_address_and_a_number_it_does_not_stays_text() {
        let refs = refs().with_pulls(vec![(54001, "https://github.com/acme/kite/pull/54001".into())]);
        let out = refs.linked("CI is running on #54001. I replied on #53970; `#54001` and x#54001 are not. See acme/kite#54001, other/repo#7 and https://github.com/acme/kite/pull/54001.");
        assert_eq!(
            out,
            "CI is running on [#54001](https://github.com/acme/kite/pull/54001). I replied on #53970; `#54001` and x#54001 are not. See [acme/kite#54001](https://github.com/acme/kite/pull/54001), [other/repo#7](https://github.com/other/repo/issues/7) and [https://github.com/acme/kite/pull/54001](<https://github.com/acme/kite/pull/54001>)."
        );
    }

    #[test]
    fn a_job_named_by_title_or_id_is_a_link() {
        let out = refs().linked("See Audit the docs index and s-ad34a1b8.");
        assert_eq!(out, "See [Audit the docs index](armada-ref:session/ad34a1b8-683f) and [s-ad34a1b8](armada-ref:session/ad34a1b8-683f).");
    }

    #[test]
    fn code_is_left_alone_unless_it_is_exactly_a_job_or_an_address() {
        let out = refs().linked("`s-ad34a1b8` `fix s-ad34a1b8` `https://example.com/a` ```\ns-ad34a1b8\n```");
        assert_eq!(out, "[`s-ad34a1b8`](armada-ref:session/ad34a1b8-683f) `fix s-ad34a1b8` [`https://example.com/a`](<https://example.com/a>) ```\ns-ad34a1b8\n```");
    }

    #[test]
    fn a_code_span_that_is_a_whole_file_path_is_a_file_link() {
        let out = refs().linked("`/tmp/armada-red/operator-handoff.md` `crates/a/b.rs` `a/b` `cargo test` `x.rs`");
        assert_eq!(out, "[`/tmp/armada-red/operator-handoff.md`](<armada-ref:file//tmp/armada-red/operator-handoff.md>) [`crates/a/b.rs`](<armada-ref:file/crates/a/b.rs>) `a/b` `cargo test` `x.rs`");
        assert_eq!(Refs::file_of("armada-ref:file//tmp/x.md").as_deref(), Some("/tmp/x.md"));
        assert_eq!(Refs::file_of("armada-ref:file/ops/config/routes.rb:878-880").as_deref(), Some("ops/config/routes.rb"));
    }

    #[test]
    fn a_name_inside_a_longer_word_or_a_written_link_is_not_one() {
        let out = refs().linked("xad34a1b8 [Audit the docs index](https://x.test/a)");
        assert_eq!(out, "xad34a1b8 [Audit the docs index](https://x.test/a)");
    }
}
