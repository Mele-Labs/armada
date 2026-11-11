//! What a thread's words name that the native app can open, as Electron's `refScanner` and
//! `linked` (packages/screens/src/prose-refs.ts, primitives/Prose/Prose.tsx): a Job by its id, its
//! `s-` prefix, its first eight characters or its exact title. **Found off the Jobs list, never off
//! the words' own claim**: a name that matches no Job stays text. Pull requests, issues, files,
//! documents and walk links open in panes and windows native does not draw yet, so they stay text
//! until those do. A web address that is a whole code span is a link, as Electron's `Code`.

use ipc::SessionRecord;

/// The scheme a Job reference travels through the markdown in, as Electron's `armada-ref:`.
const REF: &str = "armada-ref:session/";

/// Each Job and the words that name it.
pub(super) struct Refs {
    jobs: Vec<(String, Vec<String>)>,
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
        Refs { jobs }
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
        found.sort_by(|a, b| (b.1 - b.0).cmp(&(a.1 - a.0)).then(a.0.cmp(&b.0)));
        let mut kept: Vec<(usize, usize, &str)> = Vec::new();
        for one in found {
            if kept.iter().all(|had| one.1 <= had.0 || one.0 >= had.1) {
                kept.push(one);
            }
        }
        kept.sort_by_key(|one| one.0);
        kept
    }

    /// The Job a press on a link opens, where the link is one this file wrote.
    pub(super) fn job_of(address: &str) -> Option<String> {
        address.strip_prefix(REF).map(|id| id.to_string())
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
        for (start, end, id) in self.scan(part) {
            let said = part[start..end].replace('[', "\\[").replace(']', "\\]");
            out.push_str(&part[from..start]);
            out.push_str(&format!("[{said}]({REF}{id})"));
            from = end;
        }
        out.push_str(&part[from..]);
        out
    }
}

/// Whether `text[start..end]` stands as a whole word: no letter, digit, `_` or `-` either side.
fn whole(text: &str, start: usize, end: usize) -> bool {
    let joins = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    !text[..start].chars().next_back().is_some_and(joins) && !text[end..].chars().next().is_some_and(joins)
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
        Refs { jobs: vec![("ad34a1b8-683f".into(), vec!["ad34a1b8-683f".into(), "s-ad34a1b8".into(), "ad34a1b8".into(), "Audit the docs index".into()])] }
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
    fn a_name_inside_a_longer_word_or_a_written_link_is_not_one() {
        let out = refs().linked("xad34a1b8 [Audit the docs index](https://x.test/a)");
        assert_eq!(out, "xad34a1b8 [Audit the docs index](https://x.test/a)");
    }
}
