//! Whose a batch's new `verify-foundations` lines are, read from the paths
//! they name and the paths each member changed. `None` sends the batch back
//! to the split. `docs/capabilities/merge-line.md`, *Batching*.

/// Each member's share of `new_lines`, in member order, where every line
/// names at least one path and every path it names was changed by exactly
/// one member, the same one. `own` is each member's paths against its
/// merge-base.
pub fn blame(new_lines: &[String], own: &[Vec<String>]) -> Option<Vec<Vec<String>>> {
    let known: Vec<&str> = own.iter().flatten().map(String::as_str).collect();
    let mut owners = Vec::with_capacity(new_lines.len());
    for line in new_lines {
        owners.push(match line.trim().strip_prefix("missing:") {
            Some(finding) => Some(owner(finding, own, &known)?),
            // A rule's own `FAIL` line: it is new only with its findings,
            // and the first of them follows it.
            None => None,
        });
    }
    let mut shares = vec![Vec::new(); own.len()];
    for (at, line) in new_lines.iter().enumerate() {
        let member = match owners[at] {
            Some(member) => member,
            None => owners.get(at + 1).copied().flatten()?,
        };
        shares[member].push(line.clone());
    }
    Some(shares)
}

/// The one member that changed every path `finding` names.
fn owner(finding: &str, own: &[Vec<String>], known: &[&str]) -> Option<usize> {
    let mut found = None;
    for path in named_paths(finding, known) {
        let touched: Vec<usize> = own
            .iter()
            .enumerate()
            .filter(|(_, paths)| paths.iter().any(|changed| touches(&path, changed)))
            .map(|(member, _)| member)
            .collect();
        let [member] = touched[..] else {
            return None;
        };
        if found.is_some_and(|before| before != member) {
            return None;
        }
        found = Some(member);
    }
    found
}

fn touches(named: &str, changed: &str) -> bool {
    named == changed || (named.ends_with('/') && changed.starts_with(named))
}

/// Every word of `finding` that is a repository path, with its `:N` line
/// number off: one with a `/` in it, or a root file some member changed.
fn named_paths(finding: &str, known: &[&str]) -> Vec<String> {
    finding
        .split_whitespace()
        .map(|word| {
            let mut word = word
                .trim_matches(|c| "`'\"()[]<>,;".contains(c))
                .trim_end_matches(['.', ':']);
            while let Some((head, tail)) = word.rsplit_once(':') {
                if tail.is_empty() || !tail.bytes().all(|b| b.is_ascii_digit()) {
                    break;
                }
                word = head;
            }
            word.strip_prefix("./").unwrap_or(word)
        })
        .filter(|word| {
            (word.contains('/') && !word.contains("://") && !word.starts_with('/'))
                || known.contains(word)
        })
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::blame;

    fn lines(text: &[&str]) -> Vec<String> {
        text.iter().map(|line| line.to_string()).collect()
    }

    fn paths(each: &[&[&str]]) -> Vec<Vec<String>> {
        each.iter().map(|paths| lines(paths)).collect()
    }

    #[test]
    fn a_file_one_member_grew_is_that_members() {
        let new = lines(&[
            "FAIL  no_file_too_long",
            "missing: apps/desktop/src/main/index.ts is 1203 lines, over 1200",
        ]);
        let own = paths(&[
            &["a.rs"],
            &["b.rs"],
            &["apps/desktop/src/main/index.ts", "c.rs"],
        ]);
        assert_eq!(blame(&new, &own), Some(vec![vec![], vec![], new.clone()]));
    }

    #[test]
    fn a_line_number_and_a_root_file_are_read_as_paths() {
        let new = lines(&[
            "missing: crates/a.rs:80 — a rule this line breaks",
            "missing: ROUTES.md is 51 lines, over 50 — move the explanation, keep the pointer",
        ]);
        let own = paths(&[&["ROUTES.md"], &["crates/a.rs"]]);
        assert_eq!(
            blame(&new, &own),
            Some(vec![vec![new[1].clone()], vec![new[0].clone()]])
        );
    }

    #[test]
    fn a_path_two_members_touched_is_nobodys_alone() {
        let new = lines(&["missing: long.ts is 12 lines, over 11"]);
        assert_eq!(blame(&new, &paths(&[&["long.ts"], &["long.ts"]])), None);
    }

    #[test]
    fn a_line_naming_no_path_or_an_untouched_one_falls_back() {
        let own = paths(&[&["a.rs"], &["b.rs"]]);
        assert_eq!(blame(&lines(&["missing: a new subject"]), &own), None);
        let untouched = lines(&["missing: docs/x.md is a document docs/INDEX.md does not mention"]);
        assert_eq!(blame(&untouched, &own), None);
    }

    #[test]
    fn a_line_naming_two_members_paths_falls_back() {
        let new = lines(&["missing: a/one.rs imports a/two.rs"]);
        assert_eq!(blame(&new, &paths(&[&["a/one.rs"], &["a/two.rs"]])), None);
    }

    #[test]
    fn a_directory_is_touched_by_a_file_under_it() {
        let new = lines(&["missing: docs/notes/ has its own index"]);
        let own = paths(&[&["x.rs"], &["docs/notes/INDEX.md"]]);
        assert_eq!(blame(&new, &own), Some(vec![vec![], new.clone()]));
    }

    #[test]
    fn a_rule_line_with_no_finding_after_it_falls_back() {
        let own = paths(&[&["a.rs"]]);
        assert_eq!(blame(&lines(&["FAIL  a new rule"]), &own), None);
    }
}
