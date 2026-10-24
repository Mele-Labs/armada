//! The beads claim: **an issue filed on one machine shows on the other machine's Issues within a
//! minute, with nobody pulling by hand; and when two machines claim one issue, both name the same
//! winner after a sync, and the loser's claim is refused naming who holds it.**
//!
//! Two machines on one remote, every copy held in memory. Nothing reaches `bd`, a disk or the
//! network: the remote is `testkit::FakeBeadsRemote`, each machine's copy a `FakeBeads` whose sync
//! merges as spike 38 measured beads' does. Each machine's Fleet is `fleet::backlog` driven on a
//! clock the test moves, at the intervals Fleet runs it at.
//!
//! | Proved | Not proved |
//! |---|---|
//! | A write on A is pushed at A's next look, before A's timer is due | That `bd`'s files move on a write and not on a read. `adapters::tests::beads` |
//! | B lists it after its own timed sync, inside a minute, and nothing on B pulled by hand | That Fleet runs the looks while it serves the repository. `fleet::tests::backlog` |
//! | A claim through Fleet on B, inside the minute before B heard of A's, is refused naming A, and both then name A | That the list Bridge shows is read again after a sync moved the copy. `fleet::tests::backlog` |
//! | Two claims made in the copies before either synced both answer yes, and after a sync each both name the later | That two claims through Fleet whose syncs overlap are told apart: both can answer yes until the next sync, as the spike found |

use std::time::Duration;

use adapter_traits::{Backlog, Tracker};
use fleet::backlog::{claim_settled, ClaimLost, Keeping, LOOK_EVERY};
use testkit::{FakeBeads, FakeBeadsRemote};

const ROOT: &str = "/repositories/armada";
const MINUTE: Duration = Duration::from_secs(60);

/// One machine: its copy of the backlog and what its Fleet keeps of it.
struct Machine {
    copy: FakeBeads,
    keeping: Keeping,
}

impl Machine {
    fn on(remote: &FakeBeadsRemote) -> Machine {
        Machine { copy: FakeBeads::on(remote), keeping: Keeping::default() }
    }

    /// One turn of this machine's Fleet over the repository.
    fn look(&mut self, now: Duration) {
        let _ = self.keeping.look(&self.copy, ROOT, now);
    }

    /// The open issues as this machine's Issues lists them.
    fn lists(&self, reference: &str) -> bool {
        self.copy.open_issues(ROOT).expect("listed").iter().any(|issue| issue.reference == reference)
    }

    /// Who this machine's copy says holds `reference`.
    fn holder(&self, reference: &str) -> Vec<String> {
        let read = self.copy.read_issue(ROOT, reference).expect("read").expect("there");
        read.assignees.iter().map(|one| one.as_written().to_string()).collect()
    }

    fn claim(&mut self, reference: &str, actor: &str, now: Duration) -> Result<(), ClaimLost> {
        claim_settled(&mut self.keeping, &self.copy, &self.copy, ROOT, reference, actor, now)
    }
}

/// Both machines' Fleets turning from `from`, every look, until `done` or a minute is up: when it
/// was done, counted from `from`.
fn turn_until(a: &mut Machine, b: &mut Machine, from: Duration, done: impl Fn(&Machine, &Machine) -> bool) -> Option<Duration> {
    let mut now = from;
    while now < from + MINUTE {
        now += LOOK_EVERY;
        a.look(now);
        b.look(now);
        if done(a, b) {
            return Some(now - from);
        }
    }
    None
}

#[test]
fn an_issue_filed_on_one_machine_shows_on_the_other_machines_issues_within_a_minute() {
    let remote = FakeBeadsRemote::new();
    let (mut a, mut b) = (Machine::on(&remote), Machine::on(&remote));
    a.look(Duration::ZERO);
    b.look(Duration::ZERO);

    // An agent on A files one with `bd create`; nobody tells Fleet.
    let filed_at = Duration::from_secs(1);
    let reference = a.copy.create("Sync the backlog every 30 seconds");
    assert!(!b.lists(&reference), "B has not synced");

    a.look(filed_at + LOOK_EVERY);
    assert_eq!(remote.holder(&reference).as_deref(), Some(""), "A's next look pushed the write, before its timer was due");

    let syncs_before = b.copy.syncs();
    let shown = turn_until(&mut a, &mut b, filed_at, |_, b| b.lists(&reference));
    assert!(shown.is_some_and(|after| after <= MINUTE), "B's Issues lists it inside a minute: {shown:?}");
    assert_eq!(b.copy.syncs() - syncs_before, 1, "one timed sync on B brought it, and nothing else pulled");
}

#[test]
fn two_machines_claim_one_issue_and_both_name_the_same_winner_after_a_sync() {
    let remote = FakeBeadsRemote::new();
    let (mut a, mut b) = (Machine::on(&remote), Machine::on(&remote));
    let reference = a.copy.create("Claim an issue as claim, sync, read back");
    a.look(Duration::ZERO);
    b.look(Duration::ZERO);
    assert!(b.lists(&reference), "both copies hold the issue");

    // A's Job claims it through Fleet, and B's a few seconds later, before B's timer pulled A's claim.
    let now = Duration::from_secs(5);
    assert_eq!(a.claim(&reference, "mac-a/job-a", now), Ok(()), "A holds it");
    match b.claim(&reference, "mac-b/job-b", now + LOOK_EVERY) {
        Err(ClaimLost::Held { holder }) => assert_eq!(holder, "mac-a/job-a", "the refusal names who holds it"),
        other => panic!("B's claim is refused naming A: {other:?}"),
    }
    let agreed = turn_until(&mut a, &mut b, now, |a, b| a.holder(&reference) == b.holder(&reference));
    assert!(agreed.is_some(), "both copies agree after a sync");
    assert_eq!(a.holder(&reference), ["mac-a/job-a"]);
    assert_eq!(b.holder(&reference), ["mac-a/job-a"]);

    // Two agents claim another in their own copies with `bd` before either synced: both are told
    // yes, as the spike found. After a sync each, both copies name the later.
    let other = a.copy.create("Two agents at once");
    let later = Duration::from_secs(100);
    a.look(later);
    b.look(later + Duration::from_secs(40));
    assert!(b.lists(&other));
    assert!(a.copy.claim(ROOT, &other, "mac-a/agent").is_ok());
    assert!(b.copy.claim(ROOT, &other, "mac-b/agent").is_ok());
    let settled = later + Duration::from_secs(41);
    let agreed = turn_until(&mut a, &mut b, settled, |a, b| a.holder(&other) == b.holder(&other));
    assert!(agreed.is_some(), "both copies agree inside a minute: {:?} and {:?}", a.holder(&other), b.holder(&other));
    assert_eq!(a.holder(&other), ["mac-b/agent"], "the later claim won on both");
}
