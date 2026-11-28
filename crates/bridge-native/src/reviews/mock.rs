//! The Reviews screen on invented data: every inbox group, a refresh under way, a changed pull request
//! with replies, a force-pushed one, a viewed file that changed, a critique and drafts. No name here is
//! a real company's, repository's or ticket's.

use ipc::{
    DiffSide, PullRequestCheckRow, PullRequestCheckState, PullRequestCommit, PullRequestEntry, PullRequestFileStatus, PullRequestReviewer, PullRequestReviewerKind,
    PullRequestVerdict, PullReviewChecks, PullReviewList, PullReviewThread, PullReviewThreadComment,
};

use super::model::{
    self,
    ChangeSet, Critique, DraftOf, Explain, Finding, Flag, FlagKind, Hunk, Inbox, InboxRow, Kept, Mine, Overview, Page, PageFile, Practice, Reply, Rewrite, Seen, Span,
    Summary, Verdict, Viewed,
};

/// The pull request his last visit had open, which opens again at once from what was kept.
pub const LAST_OPEN: u64 = 2190;

#[allow(clippy::too_many_arguments)]
fn row(number: u64, repository: &str, title: &str, author: &str, age: &str, changed: bool, new_commits: Option<u32>, lists: &[PullReviewList], checks: PullReviewChecks, mine: Verdict) -> InboxRow {
    InboxRow {
        number,
        repository: repository.into(),
        title: title.into(),
        author: author.into(),
        age: age.into(),
        updated_at: String::new(),
        asked_at: None,
        changed,
        new_commits,
        lists: lists.to_vec(),
        checks,
        mine,
        reviewing: false,
        ignored: false,
        seen: false,
    }
}

/// Fleet's calls for the reviews asked of him by name, as it would send them: two first looks, neither opened.
pub fn asked() -> ipc::StandingCalls {
    let call = |number: u64, title: &str, author: &str, minutes: u32| {
        let key = format!("review:harbor/ledger-api#{number}");
        format!(
            r#"{{"key":"{key}","owner":"{key}","kind":"review_requested","title":"{title}","text":"Review requested on harbor/ledger-api#{number}, opened by {author}","since":"2026-11-16T09:{minutes:02}:00.000Z","tone":"waiting","opened":false,"review":{{"manifest_id":"ledger-api","repository":"harbor/ledger-api","number":{number},"title":"{title}","author":"{author}","address":"https://example.com/harbor/ledger-api/pull/{number}"}}}}"#
        )
    };
    let calls = [call(2214, "Add rate limits to the export endpoint", "mira", 0), call(2231, "Say which stage of an invoice failed", "mira", 30)];
    ipc::decode("StandingCalls", format!(r#"{{"calls":[{}]}}"#, calls.join(",")).as_bytes()).unwrap_or_else(|why| panic!("the asked reviews do not decode: {why}"))
}

/// The inbox as the last read left it, drawn while the refresh runs.
pub fn inbox() -> Inbox {
    use PullReviewList::*;
    let rows = vec![
        row(2231, "harbor/ledger-api", "Say which stage of an invoice failed", "mira", "30m", false, None, &[Requested], PullReviewChecks::Passed, Verdict::NotReviewed),
        row(2214, "harbor/ledger-api", "Add rate limits to the export endpoint", "mira", "2h", false, None, &[Requested], PullReviewChecks::Passed, Verdict::NotReviewed),
        row(2219, "harbor/ledger-api", "Log the export batch size", "mira", "3h", false, None, &[Requested], PullReviewChecks::Passed, Verdict::NotReviewed),
        row(2198, "harbor/ledger-api", "Retry failed webhooks with backoff", "tobias", "40m", true, Some(3), &[Requested, Assigned], PullReviewChecks::Running, Verdict::Commented),
        row(2190, "harbor/ledger-api", "Split the invoice job into stages", "mira", "1d", false, None, &[Requested], PullReviewChecks::Passed, Verdict::Commented),
        row(2205, "harbor/ledger-api", "Batch the ledger sync writes", "tobias", "1d", false, None, &[Reviewing], PullReviewChecks::Passed, Verdict::NotReviewed),
        row(865, "harbor/web-shell", "Cache the avatar images", "priya", "1h", true, Some(2), &[Assigned], PullReviewChecks::Failed, Verdict::ChangesRequested),
        row(143, "harbor/infra-kit", "Pin the runner image", "you", "3d", false, None, &[Mine], PullReviewChecks::Passed, Verdict::NotReviewed),
        row(871, "harbor/web-shell", "Move the settings page to the new form kit", "jun", "5h", false, None, &[Team], PullReviewChecks::Running, Verdict::NotReviewed),
        row(2177, "harbor/ledger-api", "Drop the old CSV export", "jun", "4d", false, None, &[Assigned], PullReviewChecks::Passed, Verdict::Approved),
        row(859, "harbor/web-shell", "Fix the focus ring on menus", "priya", "6d", false, None, &[Assigned], PullReviewChecks::Passed, Verdict::Approved),
        row(2163, "harbor/ledger-api", "Bump the lint rules", "jun", "2d", false, None, &[Team], PullReviewChecks::Passed, Verdict::NotReviewed),
    ];
    // When each was asked of him: oldest first under Review requested, not as the list came.
    let asked = [(2231, "2026-11-16T09:30:00Z"), (2214, "2026-11-16T09:00:00Z"), (2219, "2026-11-15T11:00:00Z"), (2198, "2026-11-14T15:00:00Z"), (2190, "2026-11-12T10:00:00Z")];
    // He ran a review in Armada on these: one asked of him, and one on no list of the forge's.
    let reviewing = [2190, 2205];
    // He has seen one at its head, so it sits dimmed at the bottom of its group; he ignored another.
    let (seen, ignored) = ([2219], [2163]);
    let rows = rows
        .into_iter()
        .map(|mut row| {
            row.asked_at = asked.iter().find(|(number, _)| *number == row.number).map(|(_, at)| at.to_string());
            row.reviewing = reviewing.contains(&row.number);
            row.seen = seen.contains(&row.number);
            row.ignored = ignored.contains(&row.number);
            row
        })
        .collect();
    Inbox { rows, refreshing: true }
}

/// What the refresh reads: Split the invoice job got a commit, it stays under review requested.
pub fn refreshed(mut inbox: Inbox) -> Inbox {
    if let Some(row) = inbox.rows.iter_mut().find(|row| row.number == 2190) {
        row.changed = true;
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

/// His own login on the mock forge, as his reviews and comments are signed.
pub const ME: &str = "you";

/// A person asked to review, and their latest review where they have one.
fn person(login: &str, verdict: Option<PullRequestVerdict>) -> PullRequestReviewer {
    PullRequestReviewer { login: login.into(), kind: PullRequestReviewerKind::Person, verdict, name: None, url: None, team: None }
}

fn team(slug: &str) -> PullRequestReviewer {
    PullRequestReviewer { login: slug.into(), kind: PullRequestReviewerKind::Team, verdict: None, name: None, url: None, team: None }
}

/// Where a mock pull request is on the forge.
fn address(repository: &str, number: u64) -> String {
    format!("https://example.com/{repository}/pull/{number}")
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
            thread("t3", "src/webhooks/retry.rs", 21, &[("reviewbot", "<!-- reviewbot: finding 41 -->\n### Unbounded retry\n\nThe loop has **no cap** on attempts. See `max_attempts`.\n\n<details><summary>Trace</summary>\nretry::run -> queue::push -> retry::run\n</details>\n\n<div><a href=\"https://example.com/acme/billing/fix/41\"><picture><source srcset=\"https://example.com/badge.png\"><img alt=\"Fix in editor\" src=\"https://example.com/badge.png\"></picture></a></div>")]),
        ],
        critique: Critique::None,
        summary: Summary::None,
        overview: retry_overview(),
        holder: Some("Retry failed webhooks".into()),
        notes: vec!["Ask for a test that waits for the second attempt.".into()],
        kept: false,
        ended: None,
        reviewers: vec![person(ME, Some(PullRequestVerdict::Commented)), person("jun", None)],
        address: address("harbor/ledger-api", 2198),
        draft: false,
    }
}

/// What the Judge has read of the pull request a Job holds: a finished reading as Code, with two notes, each about a file the
/// page holds. The Job is Fleet's to keep these on; the mock page has none.
pub fn judge_readings() -> std::collections::HashMap<u64, ipc::JudgeReading> {
    let wire = r#"{"id":"reading-pr-2198","artifact":{"kind":"pull_request","id":"2198","title":"Retry failed webhooks with backoff"},"prompt":"","judge":"code","judge_name":"Code","state":"done","asked_at":"2026-11-13T09:20:00Z","finished_at":"2026-11-13T09:21:10Z","read":false,
        "notes":[{"id":"n1","claim":"The retry loop has no cap on attempts","where":"src/webhooks/retry.rs:21","cites":["src/webhooks/retry.rs:21"]},
                 {"id":"n2","claim":"A full queue drops its oldest webhook without telling the sender","where":"src/webhooks/queue.rs:13","cites":["src/webhooks/queue.rs:13"]}]}"#;
    ipc::decode::<ipc::JudgeReading>("judge reading", wire.as_bytes()).map(|reading| std::collections::HashMap::from([(2198, reading)])).unwrap_or_default()
}

/// The review tools that make findings, which a review can be asked to look for: Fleet's review tools
/// later.
pub fn review_tools() -> Vec<model::ReviewTool> {
    let tool = |id: &str, name: &str, said: &str, first: bool| model::ReviewTool { id: id.into(), name: name.into(), said: said.into(), first };
    vec![
        tool("bugs", "Possible bugs", "possible bugs", true),
        tool("risks", "Risks", "risks", true),
        tool("api-breaking-changes", "API breaking changes", "API breaking changes", false),
        tool("user-impact", "User impact", "user impact", false),
        tool("practices", "This repo's practices", "this repo's practices", false),
    ]
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
        overview: Overview { why: "Avatars are fetched on every page. This keeps them for the session.".into(), ..Overview::default() },
        holder: None,
        notes: Vec::new(),
        kept: false,
        ended: None,
        reviewers: vec![person(ME, Some(PullRequestVerdict::ChangesRequested)), team("web-shell-team")],
        address: address("harbor/web-shell", 865),
        draft: false,
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
        overview: Overview { why: "Exports are limited to ten a minute for each account.".into(), ..Overview::default() },
        holder: None,
        notes: Vec::new(),
        kept: false,
        ended: None,
        reviewers: vec![person(ME, None), person("tobias", None)],
        address: address("harbor/ledger-api", 2214),
        draft: false,
    }
}

const STAGE_LOGS: &str = "-        run(stage).await?;\n+        run(stage).instrument(info_span!(\"stage\", name = stage.name())).await\n+            .map_err(|error| error.context(format!(\"invoice {} failed\", self.id)))?;\n";

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
            2,
            1,
            Viewed::Yes,
            true,
            &format!("@@ -38,4 +38,5 @@\n pub async fn run_stages(&mut self) -> Result<()> {{\n     for stage in self.stages_left() {{\n{STAGE_LOGS}     }}\n"),
            Some(&format!("@@ -38,4 +38,5 @@\n pub async fn run_stages(&mut self) -> Result<()> {{\n     for stage in self.stages_left() {{\n{STAGE_LOGS}     }}\n")),
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
            asked: model::ReviewAsk::first(&review_tools()),
        },
        summary: Summary::None,
        overview: invoice_overview(),
        holder: None,
        notes: Vec::new(),
        kept: false,
        ended: None,
        reviewers: vec![person(ME, Some(PullRequestVerdict::Commented)), person("tobias", Some(PullRequestVerdict::Approved))],
        address: address("harbor/ledger-api", 2190),
        draft: false,
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
    // Find problems reads the commits since he looked again, and the newest commit's set is still being written.
    page.overview.flags.clear();
    page.overview.sets.pop();
    Some(page)
}

/// A pull request's fresh page, as the read after a press brings it.
pub fn page(number: u64) -> Option<Page> {
    Some(match number {
        2198 => retry_page(),
        865 => avatar_page(),
        2214 => limits_page(),
        2190 => invoice_page(),
        2231 => stage_page(),
        _ => {
            let row = inbox().rows.into_iter().find(|row| row.number == number)?;
            // Asked of his team, of him, or of a teammate when he wrote it; a reviewed one keeps his verdict.
            let reviewers = match (row.author == ME, row.mine) {
                (true, _) => vec![person("priya", None)],
                (false, Verdict::NotReviewed) if row.lists.contains(&PullReviewList::Team) => vec![team("web-shell-team")],
                (false, Verdict::NotReviewed) => vec![person(ME, None)],
                (false, Verdict::Commented) => vec![person(ME, Some(PullRequestVerdict::Commented))],
                (false, Verdict::ChangesRequested) => vec![person(ME, Some(PullRequestVerdict::ChangesRequested))],
                (false, Verdict::Approved) => vec![person(ME, Some(PullRequestVerdict::Approved))],
            };
            let address = address(&row.repository, number);
            let reviewing = row.reviewing;
            let mut page = Page {
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
                overview: Overview::default(),
                holder: None,
                notes: Vec::new(),
                kept: false,
                ended: None,
                reviewers,
                address,
                draft: number == 143,
            };
            // A review he ran here finished with nothing found, so the page shows No problems found.
            if reviewing {
                page.overview.read = true;
                page.critique = Critique::Finished { range: page.range(), findings: Vec::new(), asked: model::ReviewAsk::first(&review_tools()) };
            }
            page
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

/// The drafts kept from an earlier visit.
pub fn drafts() -> Vec<DraftOf> {
    vec![DraftOf {
        id: "d0".into(),
        number: 2190,
        path: "src/jobs/invoice.rs".into(),
        line: 41,
        body: "The error says which invoice failed but not which stage. Add the stage name to the error.".into(),
        finding: Some("g1".into()),
        excerpt: model::excerpt(&invoice_page().files, "src/jobs/invoice.rs", 41),
    }]
}

#[allow(clippy::too_many_arguments)]
fn flag(id: &str, kind: FlagKind, set: &str, title: &str, said: &str, path: &str, line: u64, fix: &str, comment: &str) -> Flag {
    Flag { id: id.into(), kind, set: set.into(), title: title.into(), said: said.into(), path: path.into(), line, fix: fix.into(), comment: comment.into() }
}

fn practices(kept: &[(Kept, &str)]) -> Vec<Practice> {
    kept.iter().map(|(kept, said)| Practice { said: (*said).into(), kept: *kept }).collect()
}

fn hunk(path: &str, caption: &str, patch: &str, hot: Option<usize>, flag: Option<&str>) -> Hunk {
    Hunk { path: path.into(), caption: caption.into(), patch: patch.into(), hot, flag: flag.map(str::to_string) }
}

/// The note Armada keeps over a set's practices: the folder it covers, and the doc it read them from.
fn kept_note(folder: &str, doc: &str, day: &str) -> String {
    format!("Kept by Armada for every PR that touches {folder} in ledger-api. Read from {doc} on {day}, and read again when that doc changes.")
}

#[allow(clippy::too_many_arguments)]
fn set(id: &str, title: &str, purpose: &str, files: u32, kind: &str, note: String, kept: &[(Kept, &str)], hunks: Vec<Hunk>) -> ChangeSet {
    ChangeSet { id: id.into(), title: title.into(), purpose: purpose.into(), files, kind: kind.into(), kept_note: note, practices: practices(kept), hunks }
}

const MIGRATION_UP: &str = "+ALTER TABLE invoices ADD COLUMN stage text;\n+\n+UPDATE invoices SET stage = log.stage FROM job_log log WHERE log.invoice_id = invoices.id;\n";
const MIGRATION_DOWN: &str = "+ALTER TABLE invoices DROP COLUMN stage;\n";
const STAGE_DB: &str = " pub struct InvoiceRow {\n     pub id: InvoiceId,\n+    pub stage: Option<Stage>,\n }\n";
const STAGE_SCHEMA: &str = "     total_cents -> BigInt,\n+    stage -> Nullable<Text>,\n";
const STAGE_JOB: &str = "     Err(error) => {\n+        invoice.stage = Some(stage);\n+        invoice.save().await?;\n         invoice.retries += 1;\n";
const STAGE_KIND: &str = "+#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n+pub enum Stage {\n+    Price,\n+    Tax,\n+    Send,\n+}\n";
const STAGE_RETRY: &str = "-    retry(invoice).await\n+    retry_from(invoice, invoice.stage).await\n";
const STAGE_API: &str = "     \"total\": invoice.total(),\n+    \"stage\": invoice.stage.map(Stage::name),\n";
const STAGE_DOCS: &str = "+| stage | string or null | The stage the invoice last failed in. |\n";

/// #2231: one change in four sets, a blocker, a risk and a possible bug, as the sketch he agreed has it.
fn stage_overview() -> Overview {
    use Kept::*;
    Overview {
        why: "Support needs to see which stage of an invoice failed, so they can retry only that stage.".into(),
        flags: vec![
            flag("x1", FlagKind::Blocker, "migration", "The backfill locks invoices for minutes", "It updates every invoice in one statement. Update 10,000 rows at a time.", "migrations/0042_invoice_stage.sql", 3, "Batch the backfill", "This locks invoices until every row is updated. Update 10,000 rows at a time."),
            flag("x2", FlagKind::Risk, "api", "Strict clients may reject the new field", "Every invoice in the API now has a stage. Clients that refuse unknown fields will break.", "src/api/invoices.rs", 57, "Send the stage to clients that ask", "Clients that refuse unknown fields break on the new stage. Send it only to clients that ask for version 2."),
            flag("x3", FlagKind::PossibleBug, "logic", "A crash can retry forever", "The failed stage is saved before the retry count goes up.", "src/jobs/invoice.rs", 88, "Count the retry first", "A crash between these lines saves the stage but never counts the retry. Count the retry before you save the stage."),
        ],
        sets: vec![
            set("migration", "Database migration", "Adds a stage column to invoices and fills it from the job log.", 2, "migrations", kept_note("migrations/", "docs/migrations.md", "Tuesday"), &[
                (Met, "Has a down migration"),
                (Met, "New column is nullable"),
                (Missed, "Backfill runs in batches. It runs as one statement."),
                (DoesNotApply, "Index built concurrently. No index added."),
            ], vec![
                hunk("migrations/0042_invoice_stage.sql", "Adds the stage column and fills it from the job log, first, because everything after it reads that column.", MIGRATION_UP, Some(2), Some("x1")),
                hunk("migrations/0042_invoice_stage.down.sql", "Drops the column again, so the migration can be undone before anything relies on it.", MIGRATION_DOWN, None, None),
            ]),
            set("data", "Data layer", "Reads and writes the stage with each invoice.", 2, "the data layer", kept_note("src/db/", "docs/database.md", "Monday"), &[
                (Met, "Reads go through the invoice repository"),
                (Met, "Writes run in the invoice's transaction"),
                (Missed, "A new column has a test that reads it back. No test reads the stage."),
            ], vec![
                hunk("src/db/schema.rs", "Tells the code the new column exists, right after the migration that adds it.", STAGE_SCHEMA, None, None),
                hunk("src/db/invoices.rs", "Carries the stage on each invoice row, once the schema knows the column.", STAGE_DB, None, None),
            ]),
            set("logic", "Business logic", "The invoice job records the stage it failed in.", 3, "jobs", kept_note("src/jobs/", "docs/jobs.md", "Tuesday"), &[
                (Met, "A failure is logged with its invoice"),
                (Met, "Each stage can run again on its own"),
                (DoesNotApply, "Slow work runs off the request. No request changed."),
            ], vec![
                hunk("src/jobs/stage.rs", "Names the three stages, before the job code that records them.", STAGE_KIND, None, None),
                hunk("src/jobs/invoice.rs", "Saves the stage a job failed in, now that the invoice row can hold it.", STAGE_JOB, Some(1), Some("x3")),
                hunk("src/jobs/retry.rs", "Starts a retry from the saved stage, after the job has saved it.", STAGE_RETRY, None, None),
            ]),
            set("api", "API change", "Returns the stage on GET /invoices.", 2, "the API", kept_note("src/api/", "docs/api.md", "Tuesday"), &[
                (Met, "The new field is in the API docs"),
                (Missed, "A new field waits for a version. Every client gets it."),
            ], vec![
                hunk("src/api/invoices.rs", "Sends each invoice's stage to clients, once the job records it.", STAGE_API, Some(1), Some("x2")),
                hunk("docs/api/invoices.md", "Says what the new field means, last, after the change it describes.", STAGE_DOCS, None, None),
            ]),
        ],
        read: true,
    }
}

fn stage_page() -> Page {
    use PullRequestFileStatus::*;
    let added = |path: &str, patch: &str, add: u64| file(path, Added, add, 0, Viewed::No, false, patch, None);
    let changed = |path: &str, patch: &str, add: u64, del: u64| file(path, Modified, add, del, Viewed::No, false, patch, None);
    Page {
        number: 2231,
        repository: "harbor/ledger-api".into(),
        title: "Say which stage of an invoice failed".into(),
        author: "mira".into(),
        branch: "mira/invoice-stage".into(),
        base: "main".into(),
        head: "3c81f07".into(),
        mine: Mine { verdict: Verdict::NotReviewed, on: None, day: None, requested: Some("30m ago".into()), commits: 0, replies: 0 },
        body: "Each invoice now keeps the stage it failed in, and the API returns it.".into(),
        seen: None,
        new_commits: Vec::new(),
        files: vec![
            added("migrations/0042_invoice_stage.sql", &format!("@@ -0,0 +1,3 @@\n{MIGRATION_UP}"), 3),
            added("migrations/0042_invoice_stage.down.sql", &format!("@@ -0,0 +1,1 @@\n{MIGRATION_DOWN}"), 1),
            changed("src/db/schema.rs", &format!("@@ -14,1 +14,2 @@\n{STAGE_SCHEMA}"), 1, 0),
            changed("src/db/invoices.rs", &format!("@@ -8,3 +8,4 @@\n{STAGE_DB}"), 1, 0),
            added("src/jobs/stage.rs", &format!("@@ -0,0 +1,6 @@\n{STAGE_KIND}"), 6),
            changed("src/jobs/invoice.rs", &format!("@@ -86,2 +86,4 @@\n{STAGE_JOB}"), 2, 0),
            changed("src/jobs/retry.rs", &format!("@@ -21,1 +21,1 @@\n{STAGE_RETRY}"), 1, 1),
            changed("src/api/invoices.rs", &format!("@@ -56,1 +56,2 @@\n{STAGE_API}"), 1, 0),
            changed("docs/api/invoices.md", &format!("@@ -30,0 +31,1 @@\n{STAGE_DOCS}"), 1, 0),
        ],
        replies: Vec::new(),
        checks: vec![check("build", PullRequestCheckState::Passed, Some(176)), check("test", PullRequestCheckState::Passed, Some(342))],
        conversation: Vec::new(),
        threads: Vec::new(),
        critique: Critique::None,
        summary: Summary::None,
        overview: stage_overview(),
        holder: None,
        notes: Vec::new(),
        kept: false,
        ended: None,
        reviewers: vec![person(ME, None), person("tobias", None), team("ledger-reviewers")],
        address: address("harbor/ledger-api", 2231),
        draft: false,
    }
}

const BACKOFF: &str = "-    Duration::from_secs(2)\n+    let base = Duration::from_millis(500 * 2u64.pow(attempt));\n+    let capped = base.min(CAP);\n+    capped + jitter()\n";
const BACKOFF_EXPORT: &str = " mod retry;\n+pub use retry::delay;\n";
const DROP_OLDEST: &str = "+        if self.items.len() >= self.limit {\n+            self.items.pop_front();\n+        }\n         self.items.push_back(hook);\n";
const FIRST_WAIT: &str = "+#[test]\n+fn the_first_retry_waits_half_a_second() {\n+    let first = delay(0);\n+    assert!(first >= std::time::Duration::from_millis(500));\n+}\n";

fn retry_overview() -> Overview {
    use Kept::*;
    let note = || kept_note("src/webhooks/", "docs/webhooks.md", "Monday");
    Overview {
        why: "A restarting endpoint missed every retry, because all three came within six seconds. Waiting longer each time gives it the time to come back.".into(),
        flags: vec![
            flag("y1", FlagKind::Risk, "queue", "A full queue drops a retry without saying so", "The oldest retry goes with no log line, so a lost delivery leaves no trace.", "src/webhooks/queue.rs", 14, "Log the dropped retry", "The oldest retry is popped with no log line, so a lost delivery leaves no trace. Log the dropped hook at warn."),
            flag("y2", FlagKind::PossibleBug, "wait", "A high attempt number panics", "The power overflows once the attempt reaches 64, before the cap applies.", "src/webhooks/retry.rs", 6, "Clamp the attempt", "2u64.pow(attempt) overflows once attempt reaches 64, and the cap comes too late to stop it. Clamp the attempt before the power."),
        ],
        sets: vec![
            set("wait", "Retry wait", "Works out how long a failed webhook waits before its next try.", 2, "webhooks", note(), &[
                (Met, "Every wait has a cap"),
                (Met, "Clients that failed together retry apart"),
            ], vec![hunk("src/webhooks/retry.rs", "Works out each wait, doubling up to a cap with a little jitter, first because the rest uses it.", BACKOFF, Some(1), Some("y2")), hunk("src/webhooks/mod.rs", "Lets the rest of the code call the new wait, right after it is written.", BACKOFF_EXPORT, None, None)]),
            set("queue", "Retry queue", "Keeps retries in order and drops the oldest once it is full.", 1, "webhooks", note(), &[
                (Met, "Every retry is logged"),
                (Missed, "A dropped delivery is logged at warn. It drops with no log line."),
            ], vec![hunk("src/webhooks/queue.rs", "Drops the oldest retry when the queue is full, once each retry knows its wait.", DROP_OLDEST, Some(1), Some("y1"))]),
            set("tests", "Tests and docs", "Tests the first wait, and says how retries back off.", 2, "webhooks", note(), &[
                (Met, "The docs say how often a delivery is tried"),
                (DoesNotApply, "A test for each new error. No error added."),
            ], vec![hunk("tests/webhooks_retry.rs", "Proves the first retry waits half a second, after the wait it tests is in place.", FIRST_WAIT, None, None), hunk("docs/webhooks.md", "Tells readers a delivery is now tried up to five times, last, once the change it describes is done.", "-A failed delivery is tried again three times.\n+A failed delivery is tried again up to five times.\n", None, None)]),
        ],
        read: true,
    }
}

fn invoice_overview() -> Overview {
    use Kept::*;
    let note = || kept_note("src/jobs/", "docs/jobs.md", "Tuesday");
    Overview {
        why: "A failed invoice job ran again from the start. In stages, only the stage that failed runs again.".into(),
        flags: vec![flag("z1", FlagKind::PossibleBug, "stages", "A stage with no name logs an empty field", "The setup stage has no name, so its log line has an empty field.", "src/jobs/invoice.rs", 40, "Name the setup stage", "stage.name() is empty for the setup stage. Give it a name, or log its index.")],
        sets: vec![
            set("stages", "Job stages", "Splits the invoice job into stages that each retry alone.", 1, "jobs", note(), &[
                (Met, "A failure is logged with its invoice"),
                (Missed, "Each stage has its own retries. They share one budget."),
            ], vec![hunk("src/jobs/invoice.rs", "Runs only the stages still left, first, because the log lines in the next set sit inside this loop.", "-    for stage in STAGES {\n-        run(stage).await?;\n+    for stage in self.stages_left() {\n+        run(stage).await?;\n", None, None)]),
            set("logs", "Stage logs", "Names each stage in the job's log.", 1, "jobs", note(), &[(Met, "Each stage logs when it starts and ends")], vec![hunk(
                "src/jobs/invoice.rs",
                "Names each stage in the log and in its error, inside the loop the last set changed.",
                STAGE_LOGS,
                Some(1),
                Some("z1"),
            )]),
        ],
        read: true,
    }
}

/// What How it works or How to test says about a change set when it runs now.
pub fn explanation(number: u64, set: &str, explain: Explain) -> String {
    match (number, set, explain) {
        (2231, "migration", Explain::HowItWorks) => "The migration adds a stage column that may be empty, then copies each invoice's last failed stage from the job log in one update. The down migration drops the column.",
        (2231, "migration", Explain::HowToTest) => "Run the migration on a copy of the invoices table, and check each invoice's stage against its last job log line. Then run the down migration and check the column is gone.",
        (2231, "data", Explain::HowItWorks) => "The invoice row carries the stage, and the repository reads and writes it with the rest of the invoice.",
        (2231, "data", Explain::HowToTest) => "Save an invoice with a stage, read it back, and check the stage came back.",
        (2231, "logic", Explain::HowItWorks) => "When a stage fails, the job saves that stage on the invoice and counts a retry. A retry starts from the saved stage.",
        (2231, "api", Explain::HowItWorks) => "GET /invoices returns each invoice's stage, or null where nothing failed.",
        (_, _, Explain::HowItWorks) => "This set works as its purpose says. Nothing in it reaches past the files it touches.",
        (_, _, Explain::HowToTest) => "Run the tests for these files, then try the change by hand on a copy of the data.",
    }
    .into()
}

/// What Find problems writes when he starts it on a pull request it has not read: #2214's one change
/// set with a risk, and nothing on any other.
pub fn found(number: u64) -> Overview {
    use Kept::*;
    let mut overview = Overview { read: true, ..Overview::default() };
    if number == 2214 {
        overview.flags = vec![flag("w1", FlagKind::Risk, "limits", "Nothing resets the count", "The limit reads a count, but no code sets it back to zero each minute.", "src/export/limits.rs", 3, "Reset the count each minute", "allowed() reads a count that nothing resets. Reset it at the start of each minute, or count only the last minute's exports.")];
        overview.sets = vec![set("limits", "Export limit", "Allows ten exports a minute for each account.", 1, "the API", kept_note("src/export/", "docs/api.md", "Tuesday"), &[
            (Met, "A limit is a named constant"),
            (Missed, "A limit has a test at its edge. No test calls allowed(10)."),
        ], vec![hunk("src/export/limits.rs", "Names the limit, then checks a count against it.", "+pub const PER_MINUTE: u32 = 10;\n+\n+pub fn allowed(count: u32) -> bool {\n+    count < PER_MINUTE\n+}\n", Some(3), Some("w1"))])];
    }
    overview
}

/// What Summarize, Explain or Simplify writes about a pull request's description.
pub fn rewrite(number: u64, rewrite: Rewrite) -> String {
    match (number, rewrite) {
        (2198, Rewrite::Summarize) => "Failed webhooks now wait longer before each retry: half a second, doubling, up to thirty seconds, with jitter.",
        (2198, Rewrite::Explain) => "A webhook that fails is sent again later. It used to be tried three times, two seconds apart, so an endpoint that was restarting missed every try.\n\nNow each wait is twice the last, starting at **half a second** and capped at **thirty seconds**. A little random jitter keeps clients that failed together from retrying together.",
        (2198, Rewrite::Simplify) => "A failed webhook used to be tried three times, two seconds apart. Now it waits longer each time, up to thirty seconds.",
        (2231, Rewrite::Summarize) => "Each invoice now records the stage it failed in, and the API returns that stage.",
        (2231, Rewrite::Explain) => "An invoice job runs in stages. When one fails, support could not tell which.\n\nThis adds a `stage` column to invoices, fills it from the job log, saves it when a stage fails, and returns it on `GET /invoices`. A retry then starts from that stage.",
        (2231, Rewrite::Simplify) => "Invoices now remember which step failed, and the API shows it.",
        (_, Rewrite::Summarize) => "The description says what changes and why, in a few lines.",
        (_, Rewrite::Explain) => "The description names the change and the reason for it. The files tab shows where it lands.",
        (_, Rewrite::Simplify) => "This pull request makes the change its title names.",
    }
    .into()
}
