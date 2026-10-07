//! The forge's merge queue for the base branch, one call. `gh api graphql`
//! reduced by `--jq` to `@tsv`, so the vendor's words stop in [`state_of`].

use adapter_traits::{MergeQueue, QueueEntry, QueueState};

use crate::main_ci::asked;

const QUERY: &str =
    "query($owner:String!,$name:String!,$branch:String!){repository(owner:$owner,name:$name){\
    mergeQueue(branch:$branch){entries(first:50){nodes{position state enqueuedAt \
    pullRequest{number} headCommit{oid}}}}}}";

const ENTRIES: &str = "\
    .data.repository.mergeQueue.entries.nodes[]? | \
    [(.pullRequest.number | tostring), (.position | tostring), (.state // \"\")] | @tsv";

/// The queue in position order. `None` where the forge would not answer; a
/// repository with no queue answers with none waiting.
pub(crate) fn read(in_repo: &str, base: &str) -> Option<MergeQueue> {
    if base.is_empty() || base.starts_with('-') {
        return None;
    }
    let query = format!("query={QUERY}");
    let branch = format!("branch={base}");
    let said = asked(
        in_repo,
        &[
            "api",
            "graphql",
            "-F",
            "owner={owner}",
            "-F",
            "name={repo}",
            "-f",
            &branch,
            "-f",
            &query,
            "--jq",
            ENTRIES,
        ],
    )?;
    let mut queue: MergeQueue = said.lines().filter_map(entry_of_line).collect();
    queue.sort_by_key(|entry| entry.position);
    Some(queue)
}

fn entry_of_line(line: &str) -> Option<QueueEntry> {
    let mut field = line.split('\t');
    let number = field.next()?.parse().ok()?;
    let position = field.next()?.parse().ok()?;
    Some(QueueEntry {
        number,
        position,
        state: state_of(field.next().unwrap_or_default()),
    })
}

/// **A word with no name here is only in the queue**: nothing is claimed of it.
fn state_of(word: &str) -> QueueState {
    match word {
        "QUEUED" => QueueState::Queued,
        "AWAITING_CHECKS" => QueueState::AwaitingChecks,
        "MERGEABLE" => QueueState::Mergeable,
        "UNMERGEABLE" => QueueState::Unmergeable,
        _ => QueueState::InQueue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_read_in_position_order_with_the_forges_states() {
        let lines = "21\t2\tQUEUED\n20\t1\tAWAITING_CHECKS\n22\t3\tMERGEABLE\n23\t4\tUNMERGEABLE\n24\t5\tLOCKED\nx\t5\tQUEUED\n";
        let mut queue: MergeQueue = lines.lines().filter_map(entry_of_line).collect();
        queue.sort_by_key(|entry| entry.position);
        let seen: Vec<(u64, u32, QueueState)> = queue
            .iter()
            .map(|it| (it.number, it.position, it.state))
            .collect();
        assert_eq!(
            seen,
            [
                (20, 1, QueueState::AwaitingChecks),
                (21, 2, QueueState::Queued),
                (22, 3, QueueState::Mergeable),
                (23, 4, QueueState::Unmergeable),
                (24, 5, QueueState::InQueue),
            ]
        );
    }

    #[test]
    fn a_base_that_reads_as_an_option_is_not_asked() {
        assert!(read("/nowhere", "--main").is_none());
    }
}
