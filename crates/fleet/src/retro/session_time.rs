//! The timing row of a Session's record: where the Job's time went, set beside the usual Job of its
//! workflow. `docs/concepts/retro.md`, *The record it is read from*.
//!
//! **Only a Job with a bucket far over its usual gets one**, so a retro of an ordinary Job is not
//! asked to explain time. The rule is the Usage page's: twice the usual and at least ten minutes
//! beyond it. Values in, a row out; nothing here reads the store.

use ipc::{RecordSplit, RecordTime};

use super::record::cite;

/// A bucket is far over its usual at this many times it, and this many milliseconds beyond it.
const FAR: u64 = 2;
const MARGIN_MS: u64 = 10 * 60 * 1000;

/// What the row is made from: the Job's split, and the usual of its workflow.
pub struct TimingSources {
    pub workflow: String,
    /// How many kept Jobs of the workflow the usual is the median of.
    pub jobs: u32,
    pub spent: RecordSplit,
    pub usual: RecordSplit,
}

fn far_over(spent: u64, usual: u64) -> bool {
    spent >= usual.saturating_mul(FAR) && spent.saturating_sub(usual) >= MARGIN_MS
}

/// The row, or none where no bucket is far over.
pub fn timed(from: &TimingSources) -> Option<RecordTime> {
    let over: Vec<String> = from
        .spent
        .each()
        .into_iter()
        .zip(from.usual.each())
        .filter(|((_, spent), (_, usual))| far_over(*spent, *usual))
        .map(|((name, _), _)| name.to_string())
        .collect();
    if over.is_empty() {
        return None;
    }
    Some(RecordTime {
        cite: cite("time", 0),
        workflow: from.workflow.clone(),
        jobs: from.jobs,
        spent: from.spent,
        usual: from.usual,
        over,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: u64 = 60_000;

    fn sources(spent: RecordSplit, usual: RecordSplit) -> TimingSources {
        TimingSources { workflow: "build".into(), jobs: 12, spent, usual }
    }

    #[test]
    fn a_bucket_twice_its_usual_and_ten_minutes_beyond_is_far_over() {
        let row = timed(&sources(
            RecordSplit { you: 40 * MINUTE, model: 5 * MINUTE, ..RecordSplit::default() },
            RecordSplit { you: 20 * MINUTE, model: 4 * MINUTE, ..RecordSplit::default() },
        ))
        .expect("you is far over");
        assert_eq!(row.over, vec!["you".to_string()]);
        assert_eq!((row.cite.as_str(), row.workflow.as_str(), row.jobs), ("time:1", "build", 12));
        assert_eq!(row.spent.you, 40 * MINUTE);
        assert_eq!(row.usual.you, 20 * MINUTE);
    }

    #[test]
    fn twice_with_less_than_ten_minutes_beyond_is_not_far_over() {
        let row = timed(&sources(
            RecordSplit { tools: 13 * MINUTE, ..RecordSplit::default() },
            RecordSplit { tools: 4 * MINUTE, ..RecordSplit::default() },
        ));
        assert_eq!(row, None, "13 against 4 is more than twice but only nine minutes beyond");
    }

    #[test]
    fn ten_minutes_beyond_but_under_twice_is_not_far_over() {
        let row = timed(&sources(
            RecordSplit { you: 59 * MINUTE, ..RecordSplit::default() },
            RecordSplit { you: 30 * MINUTE, ..RecordSplit::default() },
        ));
        assert_eq!(row, None);
    }

    #[test]
    fn the_edge_itself_is_far_over() {
        let row = timed(&sources(
            RecordSplit { checks: 20 * MINUTE, ..RecordSplit::default() },
            RecordSplit { checks: 10 * MINUTE, ..RecordSplit::default() },
        ));
        assert_eq!(row.map(|row| row.over), Some(vec!["checks".to_string()]));
    }
}
