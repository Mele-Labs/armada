//! Fleet's wire-change notice, read for the table the thread draws in place of its lines. A pull request that
//! changes types the branch reads is told in the merge queue, in a head and one line a type, and once more
//! at the merge, as one line. Nothing on the wire says more than the words (`notice: wire_changed`), so the
//! words are read here, and a notice or a line that does not read stays plain text.

/// One changed type the branch reads, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WireRow {
    pub name: String,
    /// What changed, as Fleet says it: `adds notice`, `removes a, b`, `added`.
    pub change: String,
    /// Each place as `path:line`.
    pub places: Vec<String>,
}

/// A line under the head: a type's row, or words the table has no place for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum WireLine {
    Row(WireRow),
    Plain(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum WireNotice {
    /// The pull request's wire change and the types the branch reads, in the queue or merged.
    Types { merged: bool, pull: u64, lines: Vec<WireLine> },
    /// `#N merged: its wire change is on main.`: one line.
    Landed { pull: u64 },
}

/// The notice's head and lines as the table draws them, or `None` where its first line is not one Fleet writes.
pub(super) fn parse(text: &str) -> Option<WireNotice> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let head = lines.next()?;
    if let Some(rest) = head.strip_prefix('#') {
        let (number, tail) = rest.split_once(' ')?;
        let pull = number.parse().ok()?;
        tail.strip_prefix("merged: its wire change is on ")?.strip_suffix('.')?;
        return lines.next().is_none().then_some(WireNotice::Landed { pull });
    }
    let (merged, after) = match head.strip_prefix("Wire change in the merge queue: ") {
        Some(after) => (false, after),
        None => {
            let after = head.strip_prefix("Wire change merged into ")?;
            (true, after.split_once(": ")?.1)
        }
    };
    let (number, _) = after.strip_prefix('#')?.split_once('.')?;
    let pull = number.parse().ok()?;
    Some(WireNotice::Types { merged, pull, lines: lines.map(line).collect() })
}

fn line(text: &str) -> WireLine {
    match row(text) {
        Some(row) => WireLine::Row(row),
        None => WireLine::Plain(text.to_string()),
    }
}

/// `SessionRow: adds notice. Read at crates/bridge-native/src/view.rs:2, crates/bridge-native/src/jobs.rs:9`.
fn row(text: &str) -> Option<WireRow> {
    let (name, rest) = text.split_once(": ")?;
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let (change, places) = rest.rsplit_once(". Read at ")?;
    let places: Vec<String> = places.split(", ").map(str::trim).filter(|place| !place.is_empty()).map(str::to_string).collect();
    if change.trim().is_empty() || places.is_empty() {
        return None;
    }
    Some(WireRow { name: name.to_string(), change: change.trim().to_string(), places })
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUEUED: &str = "Wire change in the merge queue: #3662. It changes 2 types your branch reads.\nSessionRow: adds notice. Read at crates/bridge-native/src/view.rs:2, crates/bridge-native/src/jobs.rs:9\nOperatorTurnItem: adds conflict; removes held. Read at apps/pocket/src/turn.ts:41";

    #[test]
    fn a_queued_notice_reads_as_one_row_a_type_with_each_place() {
        let Some(WireNotice::Types { merged, pull, lines }) = parse(QUEUED) else { panic!("a queued notice reads") };
        assert!(!merged);
        assert_eq!(pull, 3662);
        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines[0],
            WireLine::Row(WireRow { name: "SessionRow".into(), change: "adds notice".into(), places: vec!["crates/bridge-native/src/view.rs:2".into(), "crates/bridge-native/src/jobs.rs:9".into()] })
        );
        assert_eq!(lines[1], WireLine::Row(WireRow { name: "OperatorTurnItem".into(), change: "adds conflict; removes held".into(), places: vec!["apps/pocket/src/turn.ts:41".into()] }));
    }

    #[test]
    fn a_notice_of_a_merge_the_queue_did_not_tell_reads_as_types_too() {
        let text = "Wire change merged into main: #3663. It changes 1 type your branch reads.\nNotice: added. Read at crates/bridge-native/src/job_view/menus.rs:209";
        let Some(WireNotice::Types { merged, pull, lines }) = parse(text) else { panic!("a merged notice reads") };
        assert!(merged);
        assert_eq!(pull, 3663);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn the_merge_of_a_pull_request_told_in_the_queue_is_one_line() {
        assert_eq!(parse("#3662 merged: its wire change is on main."), Some(WireNotice::Landed { pull: 3662 }));
        assert_eq!(parse("#3662 merged: its wire change is on main.\nand more"), None);
    }

    #[test]
    fn a_line_the_table_cannot_read_stays_plain_text_among_the_rows() {
        let text = "Wire change in the merge queue: #7. It changes 2 types your branch reads.\nSessionRow: adds notice. Read at src/view.rs:2\nSomething Fleet added later, with no type";
        let Some(WireNotice::Types { lines, .. }) = parse(text) else { panic!("it reads") };
        assert!(matches!(lines[0], WireLine::Row(_)));
        assert_eq!(lines[1], WireLine::Plain("Something Fleet added later, with no type".into()));
    }

    #[test]
    fn a_notice_that_does_not_open_as_fleet_writes_it_is_not_read() {
        assert_eq!(parse("The branch is 3 commits behind main."), None);
        assert_eq!(parse("Wire change in the merge queue: the number is missing."), None);
        assert_eq!(parse("#x merged: its wire change is on main."), None);
        assert_eq!(parse(""), None);
    }
}
