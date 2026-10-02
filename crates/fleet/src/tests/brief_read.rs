//! One kept brief, read back through `get_brief`: `crate::asked::read_back`.
//!
//! **The Job's briefs directory is the allowlist**, so the cases that matter
//! are the names that would leave it — a parent, another Job's brief, a link
//! planted inside the directory that points out of it.

use core_model::Job;
use testkit::FakeWorkProduct;

use crate::asked::briefs_dir;
use crate::daemon::Fleet;
use crate::tests::daemon::{a_proposal, fittings};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

const JUDGE: &str = "implement.1.tests_pass.txt";
const GAMING: &str = "implement.1.gaming.skipped_test.txt";

fn a_fleet(home: &TempDir) -> Fixture {
    Fleet::assembled(fittings(home, FakeWorkProduct::changed(&["src/brief.rs"])))
}

fn records_root(fleet: &Fixture, job: &Job) -> String {
    fleet
        .served_by(job)
        .expect("the Job's repository")
        .records_root()
        .to_string()
}

async fn read(fleet: &Fixture, job: &Job, name: &str) -> Result<ipc::BriefContents, api::Refusal> {
    api::Queries::get_brief(fleet, ipc::JobId::from(job.id()), name.to_string()).await
}

/// The claim: a Judge's brief and a gaming check's come back whole, verbatim,
/// under the path `brief_path` spells.
#[tokio::test]
async fn a_kept_brief_comes_back_whole_under_its_own_path() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("read a brief"))
        .await
        .expect("a Job");
    let briefs = briefs_dir(&records_root(&fleet, &job), &job.handle());
    std::fs::create_dir_all(&briefs).expect("the briefs directory");
    std::fs::write(briefs.join(JUDGE), "Criterion `tests_pass`\n\n  indented\n").expect("a brief");
    std::fs::write(briefs.join(GAMING), "Pattern `skipped_test`\n").expect("a brief");

    let judged = read(&fleet, &job, JUDGE).await.expect("the Judge's brief");
    assert_eq!(
        judged.path,
        format!(".armada/briefs/{}/{JUDGE}", job.handle())
    );
    assert_eq!(judged.lines, ["Criterion `tests_pass`", "", "  indented"]);
    assert_eq!((judged.total_lines, judged.bytes), (3, 35));
    assert!(judged.whole);

    let gaming = read(&fleet, &job, GAMING).await.expect("the gaming brief");
    assert_eq!(gaming.lines, ["Pattern `skipped_test`"]);
}

/// The claim: a brief past the bound comes back as its head, and says so —
/// every line still counted.
#[tokio::test]
async fn a_long_brief_comes_back_as_its_head_and_says_it_is_cut() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("a long brief"))
        .await
        .expect("a Job");
    let briefs = briefs_dir(&records_root(&fleet, &job), &job.handle());
    std::fs::create_dir_all(&briefs).expect("the briefs directory");
    let over = crate::check_output::A_READING + 5;
    let text: String = (1..=over).map(|n| format!("line {n}\n")).collect();
    std::fs::write(briefs.join(JUDGE), text).expect("a brief");

    let read = read(&fleet, &job, JUDGE).await.expect("the head");
    assert_eq!(read.lines.len(), crate::check_output::A_READING);
    assert_eq!(read.lines[0], "line 1", "the head, not the tail");
    assert_eq!(read.total_lines as usize, over);
    assert!(!read.whole);
}

/// The claim: no name reaches a file outside this Job's briefs directory. Each
/// is a 422 naming the Job as there, never a 404.
#[tokio::test]
async fn a_name_that_leaves_the_jobs_briefs_reaches_no_file() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet.propose(a_proposal("this Job")).await.expect("a Job");
    let other = fleet
        .propose(a_proposal("another Job"))
        .await
        .expect("a Job");
    let root = records_root(&fleet, &job);
    let briefs = briefs_dir(&root, &job.handle());
    let theirs = briefs_dir(&root, &other.handle());
    std::fs::create_dir_all(&briefs).expect("the briefs directory");
    std::fs::create_dir_all(&theirs).expect("the other Job's");
    std::fs::write(theirs.join(JUDGE), "not this Job's").expect("their brief");
    let outside = home.path().join("secret.txt");
    std::fs::write(&outside, "outside").expect("a file outside");
    std::os::unix::fs::symlink(&outside, briefs.join("planted.txt")).expect("a link out");

    let parent_of_theirs = format!("../{}/{JUDGE}", other.handle());
    for named in [
        "..",
        ".",
        "",
        "../../../../etc/passwd",
        parent_of_theirs.as_str(),
        "/etc/passwd",
        "planted.txt",
        "never-kept.txt",
    ] {
        let refused = read(&fleet, &job, named).await;
        assert!(
            matches!(refused, Err(api::Refusal::Unacceptable(_))),
            "`{named}` must reach no file, and the Job is there"
        );
    }

    let gone = ipc::JobId::carried("01NOSUCHJOB");
    let missing = api::Queries::get_brief(&fleet, gone, JUDGE.to_string()).await;
    assert!(matches!(missing, Err(api::Refusal::NoSuchJob(_))));
}
