//! A pool slot as `armada worktree lease` leaves it on disk: the record beside the slot, and the
//! branch checked out in it. Fleet reads the pool through these files, so a test that wants a
//! Session to hold a slot writes them.

use adapters::leasing::Holder;

/// Who a lease is written as taken by.
pub(crate) enum By<'a> {
    /// This test's process, for no session: a person's own terminal.
    Nobody,
    /// This test's process, for the agent session `id`: what `armada worktree lease` writes now.
    Session(&'a str),
    /// A Job's lease, by its id: a hosted session's, which holds as a Job of its own id.
    Job(&'a str),
}

/// Lease slot `slot` of the repository at `root` on `branch`, and check the branch out in it.
/// Answers the slot's path.
pub(crate) fn leased(root: &str, slot: u32, branch: &str, by: By<'_>) -> String {
    let at = adapter_traits::slot_path(root, slot);
    std::fs::create_dir_all(format!("{at}/.git")).expect("a slot");
    std::fs::write(format!("{at}/.git/HEAD"), format!("ref: refs/heads/{branch}\n")).expect("a head");
    lease_record(root, slot, branch, by)
}

/// The record alone, for a slot that is already a worktree.
pub(crate) fn lease_record(root: &str, slot: u32, branch: &str, by: By<'_>) -> String {
    let at = adapter_traits::slot_path(root, slot);
    let pid = std::process::id();
    let process = || match Holder::of(pid).expect("this process is running") {
        Holder::Process { pid, started } => format!("{pid} {started}"),
        Holder::Job(id) => format!("job {id}"),
    };
    let record = match by {
        By::Nobody => format!("branch {branch}\nholder {}\nsince 1\n", process()),
        By::Session(id) => format!("branch {branch}\nholder {}\nsince 1\nsession {id}\n", process()),
        By::Job(id) => format!("branch {branch}\nholder job {id}\nsince 1\n"),
    };
    std::fs::write(format!("{at}.lease"), record).expect("a lease record");
    at
}

/// Give slot `slot` back, as `armada worktree release` does: the record is emptied, and the
/// slot is left detached.
pub(crate) fn released(root: &str, slot: u32) {
    let at = adapter_traits::slot_path(root, slot);
    std::fs::write(format!("{at}.lease"), "").expect("a record emptied");
    std::fs::write(format!("{at}/.git/HEAD"), "0123456789012345678901234567890123456789\n").expect("a head");
}
