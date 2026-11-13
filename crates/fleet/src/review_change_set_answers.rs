//! A change-set run's answer, held to what it was told: hunks it was given by id, sets it was
//! listed, docs in the checkout and practices it was numbered. Also the diff's hunks, which the
//! sets are told. `docs/concepts/pull-review.md`, *Change sets*.

use std::path::Path;

use ipc::{ChangeSet, ChangeSetKind, HunkRef, OverviewFinding, PracticeCheck, PracticeItem, PracticeVerdict, COMMENT_AT_MOST};
use serde::Deserialize;

use crate::review_runs::{file_in, nothing, place_holds, said, sentences_in};

/// Sets an answer may make. A change cut finer than this is a list of hunks, not a reading of it.
pub(crate) const SETS_AT_MOST: usize = 12;
const FINDINGS_AT_MOST: usize = 50;
pub(crate) const ITEMS_AT_MOST: usize = 30;

/// Every hunk of a unified diff, in the diff's order. A hunk's lines are counted off its header,
/// so a removed line that reads `--- ` is not taken for the next file.
pub(crate) fn hunks_of(patch: &str) -> Vec<HunkRef> {
    let (mut hunks, mut path) = (Vec::new(), String::new());
    let (mut old_left, mut new_left) = (0u64, 0u64);
    for line in patch.lines() {
        if old_left > 0 || new_left > 0 {
            match line.chars().next() {
                Some('-') => old_left = old_left.saturating_sub(1),
                Some('+') => new_left = new_left.saturating_sub(1),
                Some('\\') => {}
                _ => (old_left, new_left) = (old_left.saturating_sub(1), new_left.saturating_sub(1)),
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("diff --git ") {
            path = git_path(rest);
        } else if let Some(named) = line.strip_prefix("+++ b/") {
            path = named.to_string();
        } else if let Some(hunk) = line.strip_prefix("@@ -").and_then(|rest| hunk_of(rest, &path)) {
            (old_left, new_left) = (hunk.old_lines, hunk.new_lines);
            hunks.push(hunk);
        }
    }
    hunks
}

/// The new path of `a/<path> b/<path>`, which splits in the middle when both are one path.
fn git_path(rest: &str) -> String {
    let half = rest.len().saturating_sub(1) / 2;
    match (rest.get(..half), rest.get(half + 1..)) {
        (Some(old), Some(new)) if old.starts_with("a/") && new.starts_with("b/") && old.get(2..) == new.get(2..) => {
            new.get(2..).unwrap_or_default().to_string()
        }
        _ => rest.rsplit_once(" b/").map(|(_, new)| new.to_string()).unwrap_or_default(),
    }
}

/// `<old>[,<n>] +<new>[,<n>] @@`, a count left out being one.
fn hunk_of(rest: &str, path: &str) -> Option<HunkRef> {
    let (old, rest) = rest.split_once(" +")?;
    let (new, _) = rest.split_once(" @@")?;
    let range = |text: &str| -> Option<(u64, u64)> {
        match text.split_once(',') {
            Some((start, lines)) => Some((start.parse().ok()?, lines.parse().ok()?)),
            None => Some((text.parse().ok()?, 1)),
        }
    };
    let ((old_start, old_lines), (new_start, new_lines)) = (range(old)?, range(new)?);
    Some(HunkRef { path: path.to_string(), old_start, old_lines, new_start, new_lines })
}

/// The directory every path is under, `""` for the root.
fn area_of<'a>(paths: impl Iterator<Item = &'a str>) -> String {
    let mut common: Option<Vec<&str>> = None;
    for path in paths {
        let parts: Vec<&str> = path.split('/').collect();
        let dirs = &parts[..parts.len().saturating_sub(1)];
        common = Some(match common {
            None => dirs.to_vec(),
            Some(common) => common.iter().zip(dirs).take_while(|(a, b)| a == b).map(|(a, _)| *a).collect(),
        });
    }
    common.unwrap_or_default().join("/")
}

#[derive(Deserialize)]
struct SetsOut {
    sets: Vec<SetOut>,
}

#[derive(Deserialize)]
struct SetOut {
    title: String,
    kind: ChangeSetKind,
    purpose: String,
    hunks: Vec<String>,
}

#[derive(Deserialize)]
struct OverviewOut {
    findings: Vec<OverviewFinding>,
}

#[derive(Deserialize)]
struct FoundOut {
    items: Vec<PracticeCheck>,
}

#[derive(Deserialize)]
struct CheckedOut {
    checks: Vec<CheckOut>,
}

#[derive(Deserialize)]
struct CheckOut {
    item: usize,
    verdict: PracticeVerdict,
    note: String,
}

fn one_sentence(what: &str, text: &str) -> Result<(), String> {
    nothing(what, text)?;
    match sentences_in(text) {
        1 => Ok(()),
        n => Err(format!("{what} is {n} sentences, and it is one sentence")),
    }
}

/// The sets, with Fleet's ids and areas, and a last set for any hunk the answer put in none.
pub(crate) fn sets_of(json: &str, hunks: &[HunkRef]) -> Result<Vec<ChangeSet>, String> {
    let out: SetsOut = said(json)?;
    if out.sets.len() > SETS_AT_MOST {
        return Err(format!("it has {} sets, and at most {SETS_AT_MOST} are kept", out.sets.len()));
    }
    let mut placed = vec![false; hunks.len()];
    let mut sets = Vec::new();
    for (at, set) in out.sets.into_iter().enumerate() {
        let what = format!("set {}", at + 1);
        nothing(&format!("the title of {what}"), &set.title)?;
        one_sentence(&format!("the purpose of {what}"), set.purpose.trim())?;
        if set.hunks.is_empty() {
            return Err(format!("{what} has no hunks"));
        }
        let mut own = Vec::new();
        for id in &set.hunks {
            let index = id
                .strip_prefix('h')
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| (1..=hunks.len()).contains(n))
                .ok_or_else(|| format!("{what} names {id}, which is not a hunk of the diff"))?;
            if std::mem::replace(&mut placed[index - 1], true) {
                return Err(format!("{id} is in a set twice"));
            }
            own.push(hunks[index - 1].clone());
        }
        sets.push(ChangeSet {
            id: String::new(),
            title: set.title.trim().to_string(),
            kind: set.kind,
            purpose: set.purpose.trim().to_string(),
            area: area_of(own.iter().map(|hunk| hunk.path.as_str())),
            hunks: own,
        });
    }
    let rest: Vec<HunkRef> =
        hunks.iter().zip(&placed).filter(|(_, placed)| !**placed).map(|(hunk, _)| hunk.clone()).collect();
    if !rest.is_empty() {
        sets.push(ChangeSet {
            id: String::new(),
            title: "The rest of the change".to_string(),
            kind: ChangeSetKind::Other,
            purpose: "Hunks no set claimed, kept so nothing in the change goes unseen.".to_string(),
            area: area_of(rest.iter().map(|hunk| hunk.path.as_str())),
            hunks: rest,
        });
    }
    for (at, set) in sets.iter_mut().enumerate() {
        set.id = format!("set-{}", at + 1);
    }
    Ok(sets)
}

/// The findings, blockers first, each in a set it was told and placed on a changed file.
pub(crate) fn overview_of(json: &str, sets: &[String], changed: &[String]) -> Result<Vec<OverviewFinding>, String> {
    let mut out: OverviewOut = said(json)?;
    if out.findings.len() > FINDINGS_AT_MOST {
        return Err(format!("it has {} findings, and at most {FINDINGS_AT_MOST} are kept", out.findings.len()));
    }
    for (at, finding) in out.findings.iter_mut().enumerate() {
        let what = format!("finding {}", at + 1);
        nothing(&format!("the title of {what}"), &finding.title)?;
        finding.reason = finding.reason.trim().to_string();
        one_sentence(&format!("the reason of {what}"), &finding.reason)?;
        let length = finding.reason.chars().count();
        if length > COMMENT_AT_MOST {
            return Err(format!("the reason of {what} is {length} characters, and at most {COMMENT_AT_MOST} are posted"));
        }
        if !sets.contains(&finding.set) {
            return Err(format!("{what} names {}, which is not one of the sets", finding.set));
        }
        place_holds(&finding.at, changed, &what)?;
    }
    out.findings.sort_by_key(|finding| finding.severity);
    Ok(out.findings)
}

/// A first read: each practice found, its doc in the checkout, and the set checked against it.
pub(crate) fn found(json: &str, checkout: &Path) -> Result<Vec<PracticeCheck>, String> {
    let out: FoundOut = said(json)?;
    if out.items.len() > ITEMS_AT_MOST {
        return Err(format!("it has {} practices, and at most {ITEMS_AT_MOST} are kept", out.items.len()));
    }
    for (at, check) in out.items.iter().enumerate() {
        let what = format!("practice {}", at + 1);
        nothing(&what, &check.item)?;
        nothing(&format!("the note of {what}"), &check.note)?;
        if file_in(checkout, &check.source).is_none() {
            return Err(format!("{what} names `{}` as its doc, which is not a file in the checkout", check.source));
        }
    }
    Ok(out.items)
}

/// A check against kept practices: every item once, in the catalogue's order.
pub(crate) fn checked(json: &str, items: &[PracticeItem]) -> Result<Vec<PracticeCheck>, String> {
    let out: CheckedOut = said(json)?;
    let mut checks: Vec<Option<PracticeCheck>> = vec![None; items.len()];
    for check in out.checks {
        let number = check.item;
        let item = number
            .checked_sub(1)
            .and_then(|at| items.get(at))
            .ok_or_else(|| format!("item {number} is not in the list"))?;
        nothing(&format!("the note of item {number}"), &check.note)?;
        let made = PracticeCheck { item: item.item.clone(), source: item.source.clone(), verdict: check.verdict, note: check.note };
        if checks[number - 1].replace(made).is_some() {
            return Err(format!("item {number} is checked twice"));
        }
    }
    checks
        .into_iter()
        .enumerate()
        .map(|(at, check)| check.ok_or_else(|| format!("item {} has no check", at + 1)))
        .collect()
}

/// One plain paragraph: no blank line, and no heading or list to start it.
pub(crate) fn paragraph_holds(body: &str) -> Result<(), String> {
    let body = body.trim();
    nothing("the answer", body)?;
    if body.lines().any(|line| line.trim().is_empty()) {
        return Err("it is more than one paragraph".to_string());
    }
    let numbered = body.trim_start_matches(|c: char| c.is_ascii_digit());
    if body.starts_with(['#', '-', '*', '>']) || (numbered.len() < body.len() && numbered.starts_with(['.', ')'])) {
        return Err("it starts with a heading or a list, and it is a plain paragraph".to_string());
    }
    Ok(())
}
