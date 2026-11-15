//! A Job waiting for a worktree slot, in the words its row, its card and Now share. Fleet sends the
//! wait (`SessionRecord.slot_wait`) and its place; nothing here works one out. Waiting is the default,
//! so none of it asks him anything.

use ipc::{SlotHolder, SlotWait};

/// The heading every place says.
pub(crate) const WAITING: &str = "Waiting for a slot";

/// Its place in the line, counted from one: `1st in line`, `2nd in line`.
pub(crate) fn place(ahead: u32) -> String {
    let n = u64::from(ahead) + 1;
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix} in line")
}

/// The row's tooltip and Now's line: `Waiting for a slot, 2nd in line`.
pub(crate) fn said(wait: &SlotWait) -> String {
    format!("{WAITING}, {}", place(wait.ahead))
}

/// The slot Free gives back: the holder quiet longest that nothing keeps. Fleet lists them longest quiet first.
pub(crate) fn freeable(wait: &SlotWait) -> Option<&SlotHolder> {
    wait.holders.iter().find(|one| one.keeps.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait(ahead: u32, holders: Vec<SlotHolder>) -> SlotWait {
        SlotWait { since: ipc::Instant::carried("2026-11-14T16:12:00.000Z"), ahead, holders, creating_since: None, create_refused: None }
    }

    fn holder(slot: u32, keeps: Option<ipc::SlotKeeps>) -> SlotHolder {
        SlotHolder {
            slot,
            job: ipc::SlotHolderJob { id: format!("job-{slot}"), title: format!("Job {slot}") },
            quiet_since: ipc::Instant::carried("2026-11-14T15:00:00.000Z"),
            keeps,
        }
    }

    #[test]
    fn the_place_counts_from_one_with_its_ordinal() {
        let said: Vec<String> = [0, 1, 2, 3, 10, 11, 12, 20, 21, 111].into_iter().map(place).collect();
        assert_eq!(said, ["1st in line", "2nd in line", "3rd in line", "4th in line", "11th in line", "12th in line", "13th in line", "21st in line", "22nd in line", "112th in line"]);
    }

    #[test]
    fn the_row_and_now_say_the_wait_and_its_place() {
        assert_eq!(said(&wait(1, Vec::new())), "Waiting for a slot, 2nd in line");
    }

    #[test]
    fn free_offers_the_quietest_holder_nothing_keeps() {
        let held = wait(0, vec![holder(4, Some(ipc::SlotKeeps::Uncommitted)), holder(9, None), holder(2, None)]);
        assert_eq!(freeable(&held).map(|one| one.slot), Some(9));
        assert!(freeable(&wait(0, vec![holder(4, Some(ipc::SlotKeeps::Running))])).is_none());
    }
}
