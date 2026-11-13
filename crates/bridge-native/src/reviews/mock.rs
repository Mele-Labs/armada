//! The Reviews screen on invented data: every inbox group, a refresh under way, a changed pull request
//! with replies, a force-pushed one, a viewed file that changed, a critique and drafts. No name here is
//! a real company's, repository's or ticket's.

use ipc::{
    DiffSide, PullRequestCheckRow, PullRequestCheckState, PullRequestCommit, PullRequestEntry, PullRequestFileStatus, PullRequestVerdict,
    PullReviewChecks, PullReviewList, PullReviewThread, PullReviewThreadComment,
};

use super::model::{Critique, DraftOf, Finding, Group, Inbox, InboxRow, Mine, Page, PageFile, Reply, Seen, Span, Summary, ToolRun, Verdict, Viewed};

/// The pull request his last visit had open, which opens again at once from what was kept.
pub const LAST_OPEN: u64 = 2190;

#[allow(clippy::too_many_arguments)]
fn row(number: u64, repository: &str, title: &str, author: &str, age: &str, group: Group, new_commits: Option<u32>, lists: &[PullReviewList], checks: PullReviewChecks, mine: Verdict) -> InboxRow {
    InboxRow {
        number,
        repository: repository.into(),
        title: title.into(),
        author: author.into(),
        age: age.into(),
        group,
        new_commits,
        lists: lists.to_vec(),
        checks,
        mine,
    }
}

/// The inbox as the last read left it, drawn while the refresh runs.
pub fn inbox() -> Inbox {
    use PullReviewList::*;
    let rows = vec![
        row(2214, "harbor/ledger-api", "Add rate limits to the export endpoint", "mira", "2h", Group::FirstLook, None, &[Requested], PullReviewChecks::Passed, Verdict::NotReviewed),
        row(871, "harbor/web-shell", "Move the settings page to the new form kit", "jun", "5h", Group::FirstLook, None, &[Team], PullReviewChecks::Running, Verdict::NotReviewed),
        row(2198, "harbor/ledger-api", "Retry failed webhooks with backoff", "tobias", "40m", Group::Changed, Some(3), &[Requested, Assigned], PullReviewChecks::Running, Verdict::Commented),
        row(865, "harbor/web-shell", "Cache the avatar images", "priya", "1h", Group::Changed, Some(2), &[Assigned], PullReviewChecks::Failed, Verdict::ChangesRequested),
        row(2190, "harbor/ledger-api", "Split the invoice job into stages", "mira", "1d", Group::Waiting, None, &[Requested], PullReviewChecks::Passed, Verdict::Commented),
        row(143, "harbor/infra-kit", "Pin the runner image", "you", "3d", Group::Waiting, None, &[Mine], PullReviewChecks::Passed, Verdict::NotReviewed),
        row(2177, "harbor/ledger-api", "Drop the old CSV export", "jun", "4d", Group::Done, None, &[Requested], PullReviewChecks::Passed, Verdict::Approved),
        row(859, "harbor/web-shell", "Fix the focus ring on menus", "priya", "6d", Group::Done, None, &[Team], PullReviewChecks::Passed, Verdict::Approved),
    ];
    Inbox { rows, refreshing: true }
}

/// What the refresh reads: Split the invoice job got a commit, so it moves to Changed.
pub fn refreshed(mut inbox: Inbox) -> Inbox {
    if let Some(row) = inbox.rows.iter_mut().find(|row| row.number == 2190) {
        row.group = Group::Changed;
        row.new_commits = Some(1);
        row.age = "just now".into();
        // Its test check started on the new commit.
        row.checks = PullReviewChecks::Running;
    }
    inbox.refreshing = false;
    inbox
}

fn commit(sha: &str, headline: &str, author: &str, at: &str) -> PullRequestCommit {
    PullRequestCommit { sha: sha.into(), headline: headline.into(), author: Some(author.into()), at: Some(at.into()) }
}

/// `seconds` is how long it took, or for a running one how long it has run.
fn check(name: &str, state: PullRequestCheckState, seconds: Option<u64>) -> PullRequestCheckRow {
    PullRequestCheckRow { name: name.into(), state, seconds, address: None }
}

fn file(path: &str, status: PullRequestFileStatus, add: u64, del: u64, viewed: Viewed, since: bool, patch: &str, patch_since: Option<&str>) -> PageFile {
    PageFile {
        path: path.into(),
        status,
        additions: add,
        deletions: del,
        viewed,
        since,
        patch: patch.into(),
        patch_since: patch_since.map(str::to_string),
    }
}

fn said(id: &str, author: &str, at: &str, body: &str) -> PullRequestEntry {
    PullRequestEntry::Comment { id: id.into(), author: author.into(), bot: false, at: at.into(), edited_at: None, body: body.into() }
}

fn thread(id: &str, path: &str, line: u64, said: &[(&str, &str)]) -> PullReviewThread {
    PullReviewThread {
        id: id.into(),
        path: path.into(),
        line: Some(line),
        side: DiffSide::Right,
        resolved: false,
        comments: said
            .iter()
            .enumerate()
            .map(|(at, (author, body))| PullReviewThreadComment {
                id: format!("{id}-{at}"),
                author: (*author).into(),
                bot: false,
                at: "2026-11-12T16:40:00Z".into(),
                edited_at: None,
                body: (*body).into(),
            })
            .collect(),
    }
}

const RETRY_WHOLE: &str = "@@ -1,14 +1,38 @@
 use std::time::Duration;
+use rand::Rng;

-const RETRIES: u32 = 3;
+/// The most attempts a webhook gets.
+const RETRIES: u32 = 5;
+/// The longest wait between two attempts.
+const CAP: Duration = Duration::from_secs(30);

 pub fn delay(attempt: u32) -> Duration {
-    Duration::from_secs(2)
+    let base = Duration::from_millis(500 * 2u64.pow(attempt));
+    let capped = base.min(CAP);
+    capped + jitter()
 }
+
+fn jitter() -> Duration {
+    Duration::from_millis(rand::thread_rng().gen_range(0..250))
+}
";

const RETRY_SINCE: &str = "@@ -5,9 +5,12 @@
 pub fn delay(attempt: u32) -> Duration {
     let base = Duration::from_millis(500 * 2u64.pow(attempt));
-    base.min(CAP)
+    let capped = base.min(CAP);
+    capped + jitter()
 }
+
+fn jitter() -> Duration {
+    Duration::from_millis(rand::thread_rng().gen_range(0..250))
+}
";

const QUEUE_WHOLE: &str = "@@ -12,8 +12,14 @@ impl Queue {
     pub fn push(&mut self, hook: Hook) {
-        self.items.push_back(hook);
+        if self.items.len() >= self.limit {
+            self.items.pop_front();
+        }
+        self.items.push_back(hook);
+        tracing::info!(attempt = hook.attempt, \"queued a retry\");
     }
";

const QUEUE_SINCE: &str = "@@ -15,6 +15,7 @@ impl Queue {
         self.items.push_back(hook);
+        tracing::info!(attempt = hook.attempt, \"queued a retry\");
     }
";

const TEST_WHOLE: &str = "@@ -0,0 +1,14 @@
+use ledger_api::webhooks::{delay, Queue};
+
+#[test]
+fn the_first_retry_waits_half_a_second() {
+    let first = delay(0);
+    assert!(first >= std::time::Duration::from_millis(500));
+}
+
+#[test]
+fn a_full_queue_drops_its_oldest() {
+    let mut queue = Queue::with_limit(1);
+    queue.push(hook(1));
+    queue.push(hook(2));
+}
";

const MOD_WHOLE: &str = "@@ -1,3 +1,4 @@
 mod queue;
 mod retry;
+pub use retry::delay;
";

const DOCS_WHOLE: &str = "@@ -8,4 +8,8 @@ ## Delivery
-A failed delivery is tried again three times.
+A failed delivery is tried again up to five times.
+The wait doubles each time, from half a second up to thirty seconds,
+with a little jitter so clients do not all retry at once.
";

fn retry_page() -> Page {
    use PullRequestFileStatus::*;
    Page {
        number: 2198,
        repository: "harbor/ledger-api".into(),
        title: "Retry failed webhooks with backoff".into(),
        author: "tobias".into(),
        branch: "tobias/webhook-backoff".into(),
        base: "main".into(),
        head: "e71b0d4".into(),
        mine: Mine { verdict: Verdict::Commented, on: Some("a3f9c21".into()), day: Some("yesterday".into()), requested: None, commits: 3, replies: 2 },
        body: "A failed webhook was tried three times, two seconds apart.\n\nNow it backs off: half a second, doubling, capped at thirty seconds, with jitter.".into(),
        seen: Some(Seen { day: "yesterday".into(), time: "16:40".into(), old: "a3f9c21".into(), new: "e71b0d4".into(), gone: false }),
        new_commits: vec![
            commit("e71b0d4", "Back off with jitter between retries", "tobias", "Fri 08:58"),
            commit("c0d18ea", "Cap retries at five", "tobias", "Fri 08:41"),
            commit("9b2e7f1", "Log each retry with its attempt", "tobias", "Fri 08:30"),
        ],
        files: vec![
            file("src/webhooks/retry.rs", Modified, 24, 3, Viewed::ChangedAfter, true, RETRY_WHOLE, Some(RETRY_SINCE)),
            file("src/webhooks/queue.rs", Modified, 6, 1, Viewed::No, true, QUEUE_WHOLE, Some(QUEUE_SINCE)),
            file("tests/webhooks_retry.rs", Added, 14, 0, Viewed::No, true, TEST_WHOLE, Some(TEST_WHOLE)),
            file("src/webhooks/mod.rs", Modified, 1, 0, Viewed::Yes, false, MOD_WHOLE, None),
            file("docs/webhooks.md", Modified, 3, 1, Viewed::Yes, false, DOCS_WHOLE, None),
        ],
        replies: vec![
            Reply { path: "src/webhooks/retry.rs".into(), line: 9, author: "tobias".into(), body: "Done. The wait doubles and adds up to 250 ms of jitter.".into() },
            Reply { path: "src/webhooks/queue.rs".into(), line: 13, author: "tobias".into(), body: "Kept it. The queue drops its oldest once it is full.".into() },
        ],
        checks: vec![
            check("build", PullRequestCheckState::Passed, Some(184)),
            check("test", PullRequestCheckState::Passed, Some(371)),
            check("lint", PullRequestCheckState::Running, Some(95)),
        ],
        conversation: vec![
            said("c1", "tobias", "2026-11-11T09:12:00Z", "Ready for a look. The retry wait is the main change."),
            PullRequestEntry::Review {
                id: "r1".into(),
                author: "you".into(),
                bot: false,
                at: "2026-11-12T16:40:00Z".into(),
                edited_at: None,
                verdict: PullRequestVerdict::Commented,
                body: "Two line comments: jitter, and what the queue does when it is full.".into(),
            },
            said("c2", "tobias", "2026-11-13T09:02:00Z", "Pushed three commits for both."),
        ],
        threads: vec![
            thread("t1", "src/webhooks/retry.rs", 9, &[("you", "Add jitter, or every client retries at once."), ("tobias", "Done. The wait doubles and adds up to 250 ms of jitter.")]),
            thread("t2", "src/webhooks/queue.rs", 13, &[("you", "What happens when the queue is full?"), ("tobias", "Kept it. The queue drops its oldest once it is full.")]),
        ],
        critique: Critique::None,
        summary: Summary::None,
        tools: Default::default(),
        holder: Some("Retry failed webhooks".into()),
        notes: vec!["Ask for a test that waits for the second attempt.".into()],
        kept: false,
        ended: None,
    }
}

/// What the critique of Retry failed webhooks finds, once it has run.
pub fn retry_findings() -> Vec<Finding> {
    vec![
        Finding {
            id: "f1".into(),
            title: "Jitter can push the wait past the cap".into(),
            path: "src/webhooks/retry.rs".into(),
            line: 9,
            comment: "The jitter is added after the cap, so the last wait can run to 30.25 s. Add the jitter first, then apply the cap.".into(),
        },
        Finding {
            id: "f2".into(),
            title: "The new test never reaches a second attempt".into(),
            path: "tests/webhooks_retry.rs".into(),
            line: 5,
            comment: "It checks the first wait only. Call delay(1) as well and assert it is longer than delay(0).".into(),
        },
        Finding {
            id: "f3".into(),
            title: "Each retry logs at info".into(),
            path: "src/webhooks/queue.rs".into(),
            line: 16,
            comment: "An endpoint that is down fills the log with one line a retry. Log at debug, and at info only on the last attempt.".into(),
        },
    ]
}

const AVATAR_WHOLE: &str = "@@ -3,6 +3,15 @@
 pub async fn avatar(id: UserId) -> Image {
-    fetch(id).await
+    if let Some(hit) = CACHE.get(&id) {
+        return hit;
+    }
+    let image = fetch(id).await;
+    CACHE.insert(id, image.clone());
+    image
 }
";

fn avatar_page() -> Page {
    use PullRequestFileStatus::*;
    Page {
        number: 865,
        repository: "harbor/web-shell".into(),
        title: "Cache the avatar images".into(),
        author: "priya".into(),
        branch: "priya/avatar-cache".into(),
        base: "main".into(),
        head: "f3c7a12".into(),
        mine: Mine { verdict: Verdict::ChangesRequested, on: Some("4d2a9e0".into()), day: Some("Monday".into()), requested: None, commits: 2, replies: 0 },
        body: "Avatars are fetched on every page. This keeps them for the session.".into(),
        seen: Some(Seen { day: "Monday".into(), time: "11:05".into(), old: "4d2a9e0".into(), new: "f3c7a12".into(), gone: true }),
        new_commits: vec![commit("f3c7a12", "Cache avatars for the session", "priya", "Thu 15:20"), commit("08be4d1", "Clear the cache on sign out", "priya", "Thu 14:02")],
        files: vec![
            file("src/avatar.rs", Modified, 6, 1, Viewed::ChangedAfter, true, AVATAR_WHOLE, None),
            file("src/session.rs", Modified, 2, 0, Viewed::No, true, "@@ -20,3 +20,5 @@\n pub fn sign_out() {\n+    CACHE.clear();\n+    tracing::debug!(\"cleared the avatar cache\");\n }\n", None),
        ],
        replies: Vec::new(),
        checks: vec![check("build", PullRequestCheckState::Passed, Some(95)), check("e2e", PullRequestCheckState::Failed, Some(402))],
        conversation: vec![said("c1", "priya", "2026-11-10T10:00:00Z", "Rebased on main and squashed.")],
        threads: Vec::new(),
        critique: Critique::None,
        summary: Summary::None,
        tools: Default::default(),
        holder: None,
        notes: Vec::new(),
        kept: false,
        ended: None,
    }
}

fn limits_page() -> Page {
    use PullRequestFileStatus::*;
    Page {
        number: 2214,
        repository: "harbor/ledger-api".into(),
        title: "Add rate limits to the export endpoint".into(),
        author: "mira".into(),
        branch: "mira/export-limits".into(),
        base: "main".into(),
        head: "5e0c9a4".into(),
        mine: Mine { verdict: Verdict::NotReviewed, on: None, day: None, requested: Some("2h ago".into()), commits: 0, replies: 0 },
        body: "Exports are limited to ten a minute for each account.".into(),
        seen: None,
        new_commits: Vec::new(),
        files: vec![file(
            "src/export/limits.rs",
            Added,
            5,
            0,
            Viewed::No,
            false,
            "@@ -0,0 +1,5 @@\n+pub const PER_MINUTE: u32 = 10;\n+\n+pub fn allowed(count: u32) -> bool {\n+    count < PER_MINUTE\n+}\n",
            None,
        )],
        replies: Vec::new(),
        checks: vec![check("build", PullRequestCheckState::Passed, Some(160))],
        conversation: Vec::new(),
        threads: Vec::new(),
        critique: Critique::None,
        summary: Summary::None,
        tools: Default::default(),
        holder: None,
        notes: Vec::new(),
        kept: false,
        ended: None,
    }
}

fn invoice_page() -> Page {
    use PullRequestFileStatus::*;
    Page {
        number: 2190,
        repository: "harbor/ledger-api".into(),
        title: "Split the invoice job into stages".into(),
        author: "mira".into(),
        branch: "mira/invoice-stages".into(),
        base: "main".into(),
        head: "b19f5e2".into(),
        mine: Mine { verdict: Verdict::Commented, on: Some("77ac301".into()), day: Some("Tuesday".into()), requested: None, commits: 1, replies: 0 },
        body: "The invoice job runs as three stages, so a failed stage retries alone.".into(),
        seen: Some(Seen { day: "Tuesday".into(), time: "14:10".into(), old: "77ac301".into(), new: "b19f5e2".into(), gone: false }),
        new_commits: vec![commit("b19f5e2", "Name each stage in the log", "mira", "Wed 09:12")],
        files: vec![file(
            "src/jobs/invoice.rs",
            Modified,
            1,
            1,
            Viewed::Yes,
            true,
            "@@ -40,3 +40,3 @@\n-    run(stage).await\n+    run(stage).instrument(info_span!(\"stage\", name = stage.name())).await\n",
            Some("@@ -40,3 +40,3 @@\n-    run(stage).await\n+    run(stage).instrument(info_span!(\"stage\", name = stage.name())).await\n"),
        )],
        replies: Vec::new(),
        checks: vec![
            check("build", PullRequestCheckState::Passed, Some(201)),
            check("lint", PullRequestCheckState::Passed, Some(48)),
            check("test", PullRequestCheckState::Running, Some(190)),
        ],
        conversation: Vec::new(),
        threads: Vec::new(),
        critique: Critique::Finished {
            range: Span { said: "Mira's changes up to your review".into(), commits: "4b0e9d7 → 77ac301".into() },
            findings: vec![Finding {
                id: "g1".into(),
                title: "A failed stage is not named".into(),
                path: "src/jobs/invoice.rs".into(),
                line: 41,
                comment: "The error says which invoice failed but not which stage. Add the stage name to the error.".into(),
            }, Finding {
                id: "g2".into(),
                title: "Stages share one retry budget".into(),
                path: "src/jobs/invoice.rs".into(),
                line: 28,
                comment: "A slow first stage uses up the retries the later ones need. Give each stage its own.".into(),
            }],
        },
        summary: Summary::None,
        // Risks read his range on Tuesday: its suggestions are the ones above.
        tools: [("risks", ToolRun::Ran { on: "Tuesday's changes".into(), said: None })].into_iter().collect(),
        holder: None,
        notes: Vec::new(),
        kept: false,
        ended: None,
    }
}

/// The page his last visit had open, as it was kept then: no commits since he looked yet, and the
/// checks, replies and summary still to read.
pub fn kept(number: u64) -> Option<Page> {
    let mut page = page(number)?;
    page.kept = true;
    page.new_commits.clear();
    page.replies.clear();
    page.checks.clear();
    page.mine.commits = 0;
    page.mine.replies = 0;
    page.summary = Summary::None;
    Some(page)
}

/// A pull request's fresh page, as the read after a press brings it.
pub fn page(number: u64) -> Option<Page> {
    Some(match number {
        2198 => retry_page(),
        865 => avatar_page(),
        2214 => limits_page(),
        2190 => invoice_page(),
        _ => {
            let row = inbox().rows.into_iter().find(|row| row.number == number)?;
            Page {
                number,
                repository: row.repository,
                title: row.title,
                author: row.author.clone(),
                branch: "topic".into(),
                base: "main".into(),
                head: "9d02b6e".into(),
                // Approved ones were approved at the head; one asked of him waits since it was opened.
                mine: Mine {
                    verdict: row.mine,
                    on: (row.mine != Verdict::NotReviewed).then(|| "9d02b6e".into()),
                    day: (row.mine != Verdict::NotReviewed).then(|| if row.age == "6d" { "Saturday" } else { "Monday" }.into()),
                    requested: (row.mine == Verdict::NotReviewed && row.author != "you").then(|| format!("{} ago", row.age)),
                    commits: 0,
                    replies: 0,
                },
                body: String::new(),
                seen: None,
                new_commits: Vec::new(),
                files: Vec::new(),
                replies: Vec::new(),
                checks: Vec::new(),
                conversation: Vec::new(),
                threads: Vec::new(),
                critique: Critique::None,
                summary: Summary::None,
                tools: Default::default(),
                holder: None,
                notes: Vec::new(),
                kept: false,
                ended: None,
            }
        }
    })
}

/// Findings that already went out with an earlier review, and the day it went.
pub fn sent_findings() -> std::collections::HashMap<(u64, String), String> {
    [((2190, "g2".to_string()), "Tuesday".to_string())].into_iter().collect()
}

/// The written summary of the commits since he looked, once it is written.
pub fn summary(number: u64) -> String {
    match number {
        2198 => "Tobias now adds up to 250 ms of jitter to each retry wait and caps retries at five. Each retry is logged with its attempt.",
        2190 => "Mira now names the failing stage in the invoice job's log. Nothing else changed.",
        _ => "The author pushed new commits since you looked.",
    }
    .into()
}

/// What a find-problems tool finds when it runs now.
pub fn tool_findings(number: u64, tool: &str) -> Vec<Finding> {
    let found = |id: &str, title: &str, path: &str, line: u64, comment: &str| Finding { id: id.into(), title: title.into(), path: path.into(), line, comment: comment.into() };
    match (number, tool) {
        (2198, "risks") => vec![found("r-1", "A full queue drops a retry without saying so", "src/webhooks/queue.rs", 14, "The oldest retry is popped with no log line, so a lost delivery leaves no trace. Log the dropped hook at warn.")],
        (2198, "bugs") => vec![found("b-1", "A high attempt number panics", "src/webhooks/retry.rs", 6, "2u64.pow(attempt) overflows once attempt reaches 64, and the cap comes too late to stop it. Clamp the attempt before the power.")],
        (2190, "bugs") => vec![found("b-2", "A stage with no name logs an empty field", "src/jobs/invoice.rs", 40, "stage.name() is empty for the setup stage. Give it a name, or log its index.")],
        _ => Vec::new(),
    }
}

/// What an explain or walk-through tool says when it runs now.
pub fn explanation(number: u64, tool: &str) -> String {
    match (number, tool) {
        (2198, "how-it-works") => "A failed webhook waits half a second before its first retry, and the wait doubles each time up to thirty seconds. Up to 250 ms of jitter is added to each wait, so clients that failed together do not retry together. The queue keeps a fixed number of retries and drops its oldest once it is full.",
        (2198, "how-to-test") => "Point a webhook at an endpoint that answers 500, then watch the log: each retry logs its attempt, and the waits grow from half a second to thirty seconds.",
        (2198, "user-impact") => "A customer whose endpoint is down gets five tries instead of three, spread over about a minute instead of six seconds.",
        (2198, "why") => "Three retries two seconds apart gave up before a restarting endpoint came back, and every client retried at the same moment.",
        (2198, "walk") => "Send a webhook to a failing endpoint. Watch the log for five retries. Bring the endpoint back before the last one and see it delivered.",
        (2198, "tour") => "Start at src/webhooks/retry.rs, where delay works out each wait. Then src/webhooks/queue.rs, where a retry is queued and logged. The test in tests/webhooks_retry.rs checks the first wait.",
        _ => "Nothing to say about these changes.",
    }
    .into()
}

/// The drafts kept from an earlier visit.
pub fn drafts() -> Vec<DraftOf> {
    vec![DraftOf {
        id: "d0".into(),
        number: 2190,
        path: "src/jobs/invoice.rs".into(),
        line: 41,
        body: "The error says which invoice failed but not which stage. Add the stage name to the error.".into(),
        finding: Some("g1".into()),
    }]
}
