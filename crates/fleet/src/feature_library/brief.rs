//! A feature's page condensed for an agent: about a thousand tokens of plain text, which is four thousand characters at
//! four to a token. **Text for the agent that reads it, first lines first**: the name, area, what the page is current
//! as of and which sections are stale; who is on it; the five code entry points; the test command; the newest three
//! open bugs; how it runs in a line; designs as titles; debt in a line; and how to fetch a full section. Over the cap,
//! the last of these go first.

/// The cap in characters, a thousand tokens at four to a token.
pub(crate) const CAP_CHARS: usize = 4000;
const LINE_CHARS: usize = 240;
const ENTRY_POINTS: usize = 5;
const BUGS: usize = 3;
const DESIGNS: usize = 6;
const JOBS: usize = 5;

/// What a page is made of, already read.
#[derive(Default)]
pub(crate) struct BriefFacts {
    pub(crate) slug: String,
    pub(crate) title: String,
    pub(crate) area: String,
    /// The newest merge kept, and its date `YYYY-MM-DD`.
    pub(crate) current_as_of: Option<(u64, String)>,
    /// Each stale section's name and the merge it is stale since.
    pub(crate) stale: Vec<(String, u64)>,
    /// The titles of the Jobs on it now.
    pub(crate) jobs_now: Vec<String>,
    /// `path` or `path:line`, most useful first.
    pub(crate) entry_points: Vec<String>,
    pub(crate) test: Option<String>,
    /// Open bugs, newest first.
    pub(crate) bugs: Vec<String>,
    pub(crate) runs: Option<String>,
    pub(crate) designs: Vec<String>,
    pub(crate) debt: Option<String>,
}

/// The first non-blank line of a written section, cut to a line's length.
pub(crate) fn first_line(text: &str) -> Option<String> {
    text.lines().map(str::trim).find(|line| !line.is_empty()).map(cut)
}

fn cut(line: &str) -> String {
    match line.char_indices().nth(LINE_CHARS) {
        Some((at, _)) => format!("{}...", line[..at].trim_end()),
        None => line.to_string(),
    }
}

/// `chars / 4`, rounded up: the estimate a page is held to.
pub(crate) fn tokens_of(text: &str) -> u32 {
    text.chars().count().div_ceil(4) as u32
}

/// The page as text, in priority order and cut from the end to fit.
pub(crate) fn condensed(facts: &BriefFacts) -> String {
    let mut blocks: Vec<String> = Vec::new();
    let mut head = format!("{} ({})", facts.title, facts.area);
    match &facts.current_as_of {
        Some((pull, date)) => head.push_str(&format!("\nCurrent as of #{pull}, {date}.")),
        None => head.push_str("\nNo merge kept yet."),
    }
    if !facts.stale.is_empty() {
        let marks: Vec<String> = facts.stale.iter().map(|(section, pull)| format!("{section} since #{pull}")).collect();
        head.push_str(&format!("\nStale: {}.", marks.join("; ")));
    }
    blocks.push(head);
    if !facts.jobs_now.is_empty() {
        let titles: Vec<String> = facts.jobs_now.iter().take(JOBS).map(|title| cut(title)).collect();
        blocks.push(format!("On it now: {}.", titles.join("; ")));
    }
    if !facts.entry_points.is_empty() {
        let points: Vec<String> = facts.entry_points.iter().take(ENTRY_POINTS).map(|point| format!("- {}", cut(point))).collect();
        blocks.push(format!("Code:\n{}", points.join("\n")));
    }
    if let Some(test) = &facts.test {
        blocks.push(format!("Test: {}", cut(test)));
    }
    if !facts.bugs.is_empty() {
        let bugs: Vec<String> = facts.bugs.iter().take(BUGS).map(|bug| format!("- {}", cut(bug))).collect();
        blocks.push(format!("Open bugs:\n{}", bugs.join("\n")));
    }
    if let Some(runs) = &facts.runs {
        blocks.push(format!("Runs: {}", cut(runs)));
    }
    if !facts.designs.is_empty() {
        let titles: Vec<String> = facts.designs.iter().take(DESIGNS).map(|title| cut(title)).collect();
        blocks.push(format!("Designs: {}.", titles.join("; ")));
    }
    if let Some(debt) = &facts.debt {
        blocks.push(format!("Debt: {}", cut(debt)));
    }
    blocks.push(format!("Full sections: get_feature_page feature={}.", facts.slug));
    fit(blocks)
}

/// The blocks joined, the last dropped while the text is over the cap. The first block always stays.
fn fit(mut blocks: Vec<String>) -> String {
    while blocks.len() > 1 && blocks.join("\n").chars().count() > CAP_CHARS {
        blocks.pop();
    }
    blocks.join("\n")
}
