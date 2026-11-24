//! The issues he might pick up, mapped to features and ranked by his recent Jobs on them: one test for each way an issue
//! maps to a feature, one for the 30-day window, one for the order, and one through Fleet for what it reads and filters.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use api::{Features, Projects};
use core_model::Timestamp;
use ipc::{AreaSeed, FeatureSeed, Instant, IssueState, ManifestId, MatchedBy, NewProject, ProjectMember, RankedIssue, TrackedIssue, UpsertFeatures};
use store::{KeptFeature, KeptOpenPull};
use testkit::{FakeHarness, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct};

use super::daemon::fittings;
use super::main_ci::MANIFEST;
use super::session_host::{Shared, StandIn};
use super::tmp::TempDir;
use crate::daemon::Fleet;
use crate::feature_library::mapping::FeatureMap;
use crate::feature_library::ranking::{order_issues, recent_jobs, Ground, DAYS};

const REPOSITORY: &str = "01FIXTUREMANIFEST";

fn feature(slug: &str, title: &str, area: &str, paths: &[&str]) -> KeptFeature {
    KeptFeature {
        id: format!("id-{slug}"),
        project: "p".to_string(),
        area: area.to_string(),
        slug: slug.to_string(),
        title: title.to_string(),
        summary: "One line.".to_string(),
        state: "confirmed".to_string(),
        manifest_id: String::new(),
        entry_points: Vec::new(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        position: 0,
        added_at: "2026-11-01T00:00:00Z".to_string(),
    }
}

fn issue(reference: &str, title: &str, labels: &[&str], updated_at: &str) -> TrackedIssue {
    TrackedIssue {
        reference: reference.to_string(),
        title: title.to_string(),
        url: format!("https://forge.invalid/{reference}"),
        labels: labels.iter().map(|one| one.to_string()).collect(),
        assignees: Vec::new(),
        updated_at: Instant::carried(updated_at),
        excerpt: String::new(),
        linked_pull_request: false,
        author: None,
        state: IssueState::Open,
        fields: Vec::new(),
        holders: Vec::new(),
    }
}

fn pull(number: u64, title: &str, author: &str, files: &[&str]) -> KeptOpenPull {
    KeptOpenPull {
        manifest_id: REPOSITORY.to_string(),
        number,
        repository: "o/r".to_string(),
        title: title.to_string(),
        author: author.to_string(),
        draft: false,
        head_sha: "abc".to_string(),
        files: files.iter().map(|one| one.to_string()).collect(),
        files_capped: false,
        read_at: "2026-11-20T00:00:00Z".to_string(),
    }
}

/// Two areas: `reviews` holds the reviews feature, `board` holds the dashboard and the checks features.
fn library() -> FeatureMap {
    FeatureMap::new(
        vec![
            feature("reviews", "Pull requests and review", "review", &["crates/fleet/src/reviews"]),
            feature("project-dashboard", "Project Dashboard", "board", &["crates/bridge-native/src/project_board"]),
            feature("checks", "Checks and commands", "board", &["crates/checks-runner"]),
        ],
        Vec::new(),
    )
}

fn found(ground: &Ground<'_>, manifest: Option<&str>, one: &TrackedIssue) -> Vec<(String, Vec<MatchedBy>)> {
    ground.issue(manifest, one).0.into_iter().map(|matched| (matched.slug, matched.by)).collect()
}

fn recent(pairs: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
    pairs.iter().map(|(feature, jobs)| (format!("id-{feature}"), jobs.iter().map(|job| job.to_string()).collect())).collect()
}

#[test]
fn an_issue_maps_to_the_features_of_the_files_of_an_open_pull_request_that_names_it() {
    let map = library();
    let pulls = [pull(12, "Fix the review list (#12)", "ana", &["crates/fleet/src/reviews/lists.rs", "README.md"]), pull(120, "Unrelated #120", "ana", &["crates/checks-runner/src/run.rs"])];
    let nothing = BTreeMap::new();
    let ground = Ground { map: &map, recent: &nothing, pulls: &pulls };

    let named = issue("#12", "The list is wrong", &[], "2026-11-10T00:00:00Z");
    assert_eq!(found(&ground, Some(REPOSITORY), &named), [("reviews".to_string(), vec![MatchedBy::PullRequestFiles])], "#12 is in the title of #12, not of #120");
    assert_eq!(found(&ground, Some("another"), &named), [], "a pull request of another repository does not name it");
    assert_eq!(found(&ground, Some(REPOSITORY), &issue("#1", "Nothing", &[], "2026-11-10T00:00:00Z")), [], "#1 is not in #12");
}

#[test]
fn an_issue_maps_to_a_feature_by_a_label_equal_to_its_slug_or_its_area() {
    let map = library();
    let nothing = BTreeMap::new();
    let ground = Ground { map: &map, recent: &nothing, pulls: &[] };

    let by_slug = issue("#2", "Something", &["Reviews"], "2026-11-10T00:00:00Z");
    assert_eq!(found(&ground, None, &by_slug), [("reviews".to_string(), vec![MatchedBy::Label])]);
    let by_area = issue("#3", "Something", &["board"], "2026-11-10T00:00:00Z");
    let slugs: Vec<String> = found(&ground, None, &by_area).into_iter().map(|(slug, _)| slug).collect();
    assert_eq!(slugs, ["checks", "project-dashboard"], "an area's label names every feature in it, by title");
}

#[test]
fn an_issue_maps_to_a_feature_whose_slug_or_title_stands_in_its_title() {
    let map = library();
    let nothing = BTreeMap::new();
    let ground = Ground { map: &map, recent: &nothing, pulls: &[] };

    let by_slug = issue("#4", "The project-dashboard tiles overlap", &[], "2026-11-10T00:00:00Z");
    assert_eq!(found(&ground, None, &by_slug), [("project-dashboard".to_string(), vec![MatchedBy::Title])]);
    let by_title = issue("#5", "Project Dashboard: keep sizes", &[], "2026-11-10T00:00:00Z");
    assert_eq!(found(&ground, None, &by_title), [("project-dashboard".to_string(), vec![MatchedBy::Title])]);
    let inside = issue("#6", "The checkserver is slow", &[], "2026-11-10T00:00:00Z");
    assert_eq!(found(&ground, None, &inside), [], "`checks` is not in `checkserver`: whole words only");
}

#[test]
fn only_the_jobs_of_the_last_thirty_days_count_and_a_job_on_two_features_is_one_job() {
    let now = Timestamp::from_rfc3339("2026-11-23T12:00:00Z").epoch_millis().expect("a time");
    let times = [
        ("id-reviews", "job-a", "2026-11-22T10:00:00Z"),
        ("id-reviews", "job-b", "2026-10-24T12:00:01Z"),
        ("id-reviews", "job-old", "2026-10-24T11:59:59Z"),
        ("id-checks", "job-a", "2026-11-01T00:00:00Z"),
        ("id-checks", "job-unreadable", "not a time"),
    ]
    .map(|(feature, job, at)| (feature.to_string(), job.to_string(), at.to_string()));
    let kept = recent_jobs(&times, now);
    assert_eq!(DAYS, 30);
    assert_eq!(kept, recent(&[("reviews", &["job-a", "job-b"]), ("checks", &["job-a"])]), "the Job 30 days and a second old and the one with no time are left out, and one a second inside counts");

    let map = library();
    let ground = Ground { map: &map, recent: &kept, pulls: &[] };
    let both = issue("#7", "Reviews and checks", &["reviews", "checks"], "2026-11-10T00:00:00Z");
    let (features, jobs) = ground.issue(None, &both);
    assert_eq!(jobs, 2, "job-a is on both features and counts once");
    assert_eq!(features.iter().map(|one| (one.slug.as_str(), one.jobs)).collect::<Vec<_>>(), [("reviews", 2), ("checks", 1)]);
}

#[test]
fn rows_go_by_recent_jobs_then_newest_and_an_issue_that_matched_nothing_goes_last() {
    let map = library();
    let kept = recent(&[("reviews", &["a", "b", "c"]), ("checks", &["a"]), ("project-dashboard", &["a"])]);
    let ground = Ground { map: &map, recent: &kept, pulls: &[] };
    let ranked = |reference: &str, title: &str, labels: &[&str], at: &str| {
        let one = issue(reference, title, labels, at);
        let (features, jobs) = ground.issue(None, &one);
        RankedIssue { manifest_id: None, issue: one, features, jobs }
    };
    let mut rows = vec![
        ranked("#10", "Nothing alike, but the newest", &[], "2026-11-22T00:00:00Z"),
        ranked("#11", "One job's feature, older", &["checks"], "2026-11-01T00:00:00Z"),
        ranked("#12", "Three jobs' feature", &["reviews"], "2026-10-01T00:00:00Z"),
        ranked("#13", "One job's feature, newer", &["project-dashboard"], "2026-11-15T00:00:00Z"),
    ];
    order_issues(&mut rows);
    let order: Vec<&str> = rows.iter().map(|one| one.issue.reference.as_str()).collect();
    assert_eq!(order, ["#12", "#13", "#11", "#10"], "most Jobs, then newest, and the unmatched last whatever its date");
}

#[test]
fn a_pull_request_maps_by_its_own_files() {
    let map = library();
    let kept = recent(&[("project-dashboard", &["a", "b"])]);
    let ground = Ground { map: &map, recent: &kept, pulls: &[] };
    let (features, jobs) = ground.pull(&pull(30, "Tile sizes", "priya", &["crates/bridge-native/src/project_board/layout.rs", "docs/x.md"]));
    assert_eq!(jobs, 2);
    assert_eq!(features.iter().map(|one| (one.slug.as_str(), one.by.clone())).collect::<Vec<_>>(), [("project-dashboard", vec![MatchedBy::Files])]);
    assert!(ground.pull(&pull(31, "Docs only", "priya", &["docs/x.md"])).0.is_empty());
}

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// A Fleet serving one repository whose tracker lists `issues`, and a project of it with two features.
async fn a_rig(issues: Vec<adapter_traits::OpenIssue>) -> (Arc<Hosted>, TempDir, String, String) {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.trackers = Arc::new(FakeTrackers::empty().tracker(Arc::new(FakeTracker::listing(issues))));
    fittings.starting().manifest = config::Manifest::parse(Path::new("armada.yml"), MANIFEST).expect("parses");
    let fleet = Arc::new(Fleet::assembled(fittings).hosting_sessions_on(Arc::new(Shared(Arc::new(StandIn::default()))), Duration::from_secs(600)));
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
    let members = Some(vec![ProjectMember { manifest_id: ManifestId::carried(&manifest), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: None };
    let project = fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone();
    let seed = |slug: &str, path: &str| FeatureSeed {
        slug: slug.to_string(),
        area: "shop".to_string(),
        title: slug.to_string(),
        summary: "One line.".to_string(),
        state: None,
        paths: vec![path.to_string()],
        entry_points: Vec::new(),
        manifest_id: None,
    };
    let upsert = UpsertFeatures {
        project: project.clone(),
        areas: vec![AreaSeed { slug: "shop".into(), title: "Shop".into() }],
        features: vec![seed("cart", "src/cart"), seed("search", "src/search")],
    };
    fleet.upsert_features(None, upsert).await.expect("kept");
    (fleet, home, manifest, project)
}

#[tokio::test]
async fn fleet_ranks_the_issues_it_holds_by_the_jobs_it_recorded_and_leaves_out_what_is_taken() {
    let listed = |number: u64, title: &str, labels: &[&str]| {
        let mut one = FakeTracker::issue(number, title);
        one.labels = labels.iter().map(|label| adapter_traits::FromOutside::verbatim(*label)).collect();
        one
    };
    let mut assigned = listed(4, "Cart is assigned", &["cart"]);
    assigned.assignees = vec![adapter_traits::FromOutside::verbatim("ana")];
    let mut closed_by_a_pull = listed(5, "Cart has a pull request", &["cart"]);
    closed_by_a_pull.linked_pull_request = true;
    let (fleet, _home, manifest, project) = a_rig(vec![
        listed(1, "Nothing alike", &[]),
        listed(2, "Search is slow", &[]),
        listed(3, "Cart totals", &["cart"]),
        assigned,
        closed_by_a_pull,
    ])
    .await;
    let features: BTreeMap<String, String> = fleet.store().lock().await.features_in(&project).expect("read").into_iter().map(|one| (one.slug, one.id)).collect();
    {
        let mut store = fleet.store().lock().await;
        let now = fleet.now().as_str().to_string();
        store.keep_feature_write(&features["cart"], "job-1", "src/cart/a.rs", &now).expect("written");
        store.keep_feature_write(&features["cart"], "job-2", "src/cart/b.rs", &now).expect("written");
        store.keep_feature_write(&features["search"], "job-1", "src/search/a.rs", &now).expect("written");
        store.keep_feature_write(&features["search"], "job-long-ago", "src/search/b.rs", "2020-01-01T00:00:00Z").expect("written");
    }

    let ranked = fleet.list_ranked_issues(project.clone()).await.expect("ranked");
    let order: Vec<(&str, u32, Vec<&str>)> = ranked
        .might_pick_up
        .iter()
        .map(|one| (one.issue.reference.as_str(), one.jobs, one.features.iter().map(|matched| matched.slug.as_str()).collect()))
        .collect();
    assert_eq!(order, [("#3", 2, vec!["cart"]), ("#2", 1, vec!["search"]), ("#1", 0, vec![])], "assigned and closed-by-a-pull issues are not offered");
    assert_eq!(ranked.might_pick_up[0].manifest_id.as_ref().map(ManifestId::as_str), Some(manifest.as_str()));
    assert_eq!((ranked.days, ranked.might_be_interested.len()), (30, 0), "no account is known, so nobody's pull request is somebody else's");
}
