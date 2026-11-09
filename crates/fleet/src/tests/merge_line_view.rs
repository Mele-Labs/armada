//! The merge line as a screen draws it: the rules Bridge worked out in TypeScript, held where Fleet does.

use ipc::{
    ClaimHolder, FixingElsewhere, HolderKind, HubMergeable, HubMerged, HubPullCi, HubPullRequest, HubQueue,
    HubQueueState, HubReview, HubSession, Instant, MainChecking, MainCiState, MainFailedJob, MainMerge, MainStanding,
    MainView, ManifestId, ManifestSummary, MergeLine, MergeLineHub, MergeLineView, MergeLines, RepositorySummary,
};

use crate::merge_line_view::views;

const ROOT: &str = "/repo";

fn at(minute: u32) -> Instant {
    Instant::carried(format!("2026-11-03T10:{minute:02}:00.000Z"))
}

fn manifest(named: &str, root: &str) -> ManifestSummary {
    ManifestSummary {
        id: ManifestId::carried("01M1CNPKTV0018H2M1CXDNBK06"),
        repository: named.to_string(),
        path: format!("{root}/armada.yml"),
        records_root: format!("/records/{named}"),
        version: 1,
        checks: Vec::new(),
        auto_merge: String::new(),
        review_gate: String::new(),
        kit: None,
        connectors_off: false,
    }
}

fn set_up(named: &str, root: &str) -> RepositorySummary {
    RepositorySummary {
        root: root.to_string(),
        records_root: format!("/records/{named}"),
        manifest: Some(manifest(named, root)),
        implicit_id: None,
    }
}

fn loose(root: &str) -> RepositorySummary {
    RepositorySummary { root: root.to_string(), records_root: format!("/records{root}"), manifest: None, implicit_id: None }
}

fn pull(number: u64) -> HubPullRequest {
    HubPullRequest {
        number,
        title: "t".into(),
        branch: format!("b/{number}"),
        url: format!("https://forge/pull/{number}"),
        author: None,
        ci: None,
        queue: None,
        opened_at: None,
        updated_at: None,
        draft: false,
        mergeable: None,
        review: None,
        additions: None,
        deletions: None,
        owner: None,
        merge_by_hand: false,
        mine: true,
    }
}

fn queued(mut pull: HubPullRequest, state: HubQueueState, position: Option<u32>) -> HubPullRequest {
    pull.queue = Some(HubQueue { state, position, enqueued_at: None });
    pull
}

fn line(root: &str, hub: Option<MergeLineHub>) -> MergeLine {
    MergeLine { root: root.into(), line: Vec::new(), off: Vec::new(), landed: Vec::new(), sent_back: Vec::new(), hub }
}

fn only(hub: MergeLineHub) -> MergeLineView {
    let lines = MergeLines { lines: vec![line(ROOT, Some(hub))] };
    views(&lines, &[loose(ROOT)]).lines.remove(0)
}

fn names(every: &[RepositorySummary]) -> Vec<String> {
    let lines = MergeLines { lines: every.iter().map(|one| line(&one.root, None)).collect() };
    views(&lines, every).lines.into_iter().map(|one| one.name).collect()
}

fn main_at(state: MainCiState) -> MainStanding {
    MainStanding {
        state,
        commit: "a".repeat(40),
        read_at: at(0),
        red_since: None,
        failed: Vec::new(),
        merge: None,
        red_commit: None,
        checking: Vec::new(),
    }
}

fn job(name: &str, check: Option<&str>, tests: &[&str]) -> MainFailedJob {
    MainFailedJob {
        name: name.into(),
        check: check.map(Into::into),
        log_url: Some(format!("https://forge/log/{name}")),
        tests: tests.iter().map(|one| one.to_string()).collect(),
    }
}

fn red(failed: Vec<MainFailedJob>) -> MainStanding {
    MainStanding { failed, ..main_at(MainCiState::Red) }
}

fn numbers(pulls: &[HubPullRequest]) -> Vec<u64> {
    pulls.iter().map(|one| one.number).collect()
}

fn session(id: &str, title: Option<&str>) -> HubSession {
    HubSession { id: id.into(), title: title.map(Into::into) }
}

/// The merge of a red whose pull request the Job `held` held, on a branch or none.
fn merge_of(held: Option<HubSession>, branch: Option<&str>) -> Option<MainMerge> {
    let mut main = red(vec![job("ci", None, &[])]);
    main.merge = Some(MainMerge { number: 2175, url: None, branch: branch.map(Into::into), session: held });
    match only(MergeLineHub { main: Some(main), ..Default::default() }).main {
        Some(MainView::Red { merge, .. }) => merge,
        other => panic!("a red, read as {other:?}"),
    }
}

// -- the name

#[test]
fn a_set_up_repository_is_named_as_its_manifest_reads_and_never_by_its_id() {
    let every = [set_up("storefront", "/Users/user/code/storefront"), loose("/Users/user/old/api")];
    assert_eq!(names(&every), ["storefront", "api"]);
}

#[test]
fn the_manifests_name_beats_the_folder_which_is_the_workspace_in_a_monorepo() {
    assert_eq!(names(&[set_up("billing", "/Users/user/code/platform")]), ["billing"]);
}

#[test]
fn a_repository_not_set_up_reads_as_its_folder_even_where_a_set_up_name_shares_it() {
    let every = [set_up("api", "/Users/user/services/api"), loose("/Users/user/code/api/")];
    assert_eq!(names(&every)[1], "code/api");
}

#[test]
fn a_collision_widens_both_sides_whichever_is_set_up() {
    let every = [set_up("api", "/Users/user/services/api"), set_up("api", "/Users/user/old/api")];
    assert_eq!(names(&every), ["services/api", "old/api"]);
}

#[test]
fn the_parent_is_added_only_where_two_share_a_folder_and_the_root_past_that() {
    let every = [loose("/Users/user/code/api"), loose("/Users/user/old/api"), loose("/Volumes/code/api")];
    assert_eq!(names(&every), ["/Users/user/code/api", "old/api", "/Volumes/code/api"]);
}

#[test]
fn a_root_the_listing_has_not_caught_up_with_is_its_own_name() {
    let lines = MergeLines { lines: vec![line("/gone/away", None)] };
    assert_eq!(views(&lines, &[loose(ROOT)]).lines[0].name, "/gone/away");
}

#[test]
fn a_line_is_a_view_in_the_order_it_was_served() {
    let lines = MergeLines { lines: vec![line("/b/two", None), line("/a/one", None)] };
    let roots: Vec<_> = views(&lines, &[]).lines.into_iter().map(|one| one.root).collect();
    assert_eq!(roots, ["/b/two", "/a/one"]);
}

#[test]
fn a_line_with_no_hub_is_a_name_and_nothing_else() {
    let view = views(&MergeLines { lines: vec![line(ROOT, None)] }, &[]).lines.remove(0);
    assert_eq!(
        (view.main, view.pulls, view.landed, view.checkout, view.read_at, view.quiet_since),
        (None, vec![], vec![], None, None, None)
    );
}

// -- the pull requests

#[test]
fn an_open_pull_request_keeps_its_queue_state_and_place() {
    let mut first = queued(pull(7), HubQueueState::Queued, Some(2));
    first.ci = Some(HubPullCi::Passed);
    let waiting = queued(pull(8), HubQueueState::WaitingForCi, None);
    let view = only(MergeLineHub { pull_requests: vec![first.clone(), waiting.clone()], ..Default::default() });
    assert_eq!(view.pulls, [first, waiting]);
}

#[test]
fn the_queue_draws_first_by_position_then_those_waiting_for_ci_then_the_rest_as_listed() {
    let view = only(MergeLineHub {
        pull_requests: vec![
            pull(1),
            queued(pull(2), HubQueueState::WaitingForCi, None),
            queued(pull(3), HubQueueState::Queued, Some(2)),
            pull(4),
            queued(pull(5), HubQueueState::InQueue, Some(1)),
        ],
        ..Default::default()
    });
    assert_eq!(numbers(&view.pulls), [5, 3, 2, 1, 4]);
}

#[test]
fn your_own_pull_request_passes_through_whole() {
    let mut own = pull(9);
    own.author = Some("nick".into());
    own.opened_at = Some(at(1));
    own.updated_at = Some(at(2));
    own.draft = true;
    own.review = Some(HubReview::Approved);
    own.additions = Some(10);
    own.deletions = Some(3);
    own.merge_by_hand = true;
    own.ci = Some(HubPullCi::Failed);
    let view = only(MergeLineHub { pull_requests: vec![own.clone()], ..Default::default() });
    assert_eq!(view.pulls, [own]);
}

#[test]
fn someone_elses_pull_request_outside_the_queue_is_not_listed_and_one_waiting_for_ci_is_not_in_it() {
    let mut theirs = pull(3);
    theirs.mine = false;
    let mut waiting = queued(pull(4), HubQueueState::WaitingForCi, None);
    waiting.mine = false;
    let view = only(MergeLineHub { pull_requests: vec![pull(1), theirs, waiting], ..Default::default() });
    assert_eq!(numbers(&view.pulls), [1]);
}

#[test]
fn someone_elses_pull_request_in_the_queue_is_quiet() {
    let mut theirs = queued(pull(3), HubQueueState::Queued, Some(2));
    theirs.mine = false;
    theirs.author = Some("sam".into());
    theirs.ci = Some(HubPullCi::Failed);
    theirs.draft = true;
    theirs.merge_by_hand = true;
    theirs.mergeable = Some(HubMergeable::Conflicting);
    theirs.review = Some(HubReview::ChangesRequested);
    theirs.additions = Some(4);
    theirs.opened_at = Some(at(1));
    theirs.queue.as_mut().unwrap().enqueued_at = Some(at(2));
    let quiet = only(MergeLineHub { pull_requests: vec![theirs], ..Default::default() }).pulls.remove(0);
    let expected = HubPullRequest {
        author: Some("sam".into()),
        queue: Some(HubQueue { state: HubQueueState::InQueue, position: Some(2), enqueued_at: None }),
        mine: false,
        ..pull(3)
    };
    assert_eq!(quiet, expected);
}

#[test]
fn a_quiet_pull_takes_its_place_in_the_queue_order() {
    let mut theirs = queued(pull(2), HubQueueState::Queued, Some(1));
    theirs.mine = false;
    let view = only(MergeLineHub {
        pull_requests: vec![pull(1), queued(pull(3), HubQueueState::Queued, Some(2)), theirs],
        ..Default::default()
    });
    assert_eq!(numbers(&view.pulls), [2, 3, 1]);
}

// -- main

#[test]
fn a_green_main_is_a_mark_and_carries_the_commits_still_checking() {
    let mut main = main_at(MainCiState::Green);
    let checking = MainChecking { commit: "b".repeat(40), pull_request: None };
    main.checking = vec![checking.clone()];
    let view = only(MergeLineHub { main: Some(main), ..Default::default() });
    assert_eq!(view.main, Some(MainView::Green { checking: vec![checking] }));
}

#[test]
fn a_main_running_one_nothing_ran_on_and_one_not_read_have_no_mark() {
    for state in [MainCiState::Running, MainCiState::NothingRan] {
        assert_eq!(only(MergeLineHub { main: Some(main_at(state)), ..Default::default() }).main, None);
    }
    assert_eq!(only(MergeLineHub::default()).main, None);
}

#[test]
fn a_red_with_no_failed_job_is_no_main() {
    let view = only(MergeLineHub { main: Some(red(Vec::new())), ..Default::default() });
    assert_eq!(view.main, None);
}

#[test]
fn a_red_names_the_check_where_the_job_maps_to_one_and_the_job_where_it_does_not() {
    let mut main = red(vec![
        job("lint", Some("lint-rust"), &["a::b", "c::d"]),
        job("deploy", None, &[]),
        job("docs", Some("docs"), &[]),
    ]);
    main.red_since = Some(at(4));
    let checking = MainChecking { commit: "c".repeat(40), pull_request: None };
    main.checking = vec![checking.clone()];
    let Some(MainView::Red { failed, also, since, merge, taken_by, elsewhere, checking: held }) =
        only(MergeLineHub { main: Some(main), ..Default::default() }).main
    else {
        panic!("a red")
    };
    assert_eq!(failed.check, "lint-rust");
    assert!(!failed.unmapped);
    assert_eq!(failed.test.as_deref(), Some("a::b"));
    assert_eq!(failed.log_url.as_deref(), Some("https://forge/log/lint"));
    let rest: Vec<_> = also.iter().map(|one| (one.check.as_str(), one.unmapped, one.test.as_deref())).collect();
    assert_eq!(rest, [("deploy", true, None), ("docs", false, None)]);
    assert_eq!(since, Some(at(4)));
    assert_eq!((merge, taken_by, elsewhere), (None, None, None));
    assert_eq!(held, [checking]);
}

#[test]
fn a_job_that_maps_to_no_check_is_unmapped_as_the_first_failure_too() {
    let main = Some(red(vec![job("deploy", None, &[])]));
    let Some(MainView::Red { failed, .. }) = only(MergeLineHub { main, ..Default::default() }).main else {
        panic!("a red")
    };
    assert_eq!((failed.check.as_str(), failed.unmapped, failed.test), ("deploy", true, None));
}

#[test]
fn the_job_that_merged_is_named_by_its_title_else_its_branch_else_its_id() {
    let titled = merge_of(Some(session("c279", Some("Session Messages Fail to Send"))), Some("armada/session-c279e883"));
    assert_eq!(titled.unwrap().session, Some(session("c279", Some("Session Messages Fail to Send"))));

    let by_branch = merge_of(Some(session("c279", None)), Some("armada/session-c279e883")).unwrap();
    assert_eq!(by_branch.number, 2175);
    assert_eq!(by_branch.session, Some(session("c279", Some("armada/session-c279e883"))));

    let by_id = merge_of(Some(session("c279", None)), None).unwrap();
    assert_eq!(by_id.session, Some(session("c279", Some("c279"))));
}

#[test]
fn a_red_no_job_merged_names_none() {
    assert_eq!(merge_of(None, Some("armada/x")).unwrap().session, None);
}

#[test]
fn the_job_taking_a_red_is_named_by_its_title_else_its_id_and_another_machines_claim_rides_on_it() {
    let claim = FixingElsewhere {
        machine: "studio".into(),
        holder: ClaimHolder { kind: HolderKind::Session, id: "ab12".into() },
        since: at(3),
    };
    let hub = |fixing: HubSession| MergeLineHub {
        main: Some(red(vec![job("ci", None, &[])])),
        fixing_session: Some(fixing),
        fixing_elsewhere: Some(claim.clone()),
        ..Default::default()
    };
    let Some(MainView::Red { taken_by, elsewhere, .. }) = only(hub(session("s1", Some("Cache the read")))).main else {
        panic!("a red")
    };
    assert_eq!(taken_by, Some(session("s1", Some("Cache the read"))));
    assert_eq!(elsewhere, Some(claim.clone()));

    let Some(MainView::Red { taken_by, .. }) = only(hub(session("s1", None))).main else { panic!("a red") };
    assert_eq!(taken_by, Some(session("s1", Some("s1"))));
}

#[test]
fn a_green_main_carries_no_claim_even_where_the_hub_names_one() {
    let view = only(MergeLineHub {
        main: Some(main_at(MainCiState::Green)),
        fixing_session: Some(session("s1", None)),
        ..Default::default()
    });
    assert_eq!(view.main, Some(MainView::Green { checking: Vec::new() }));
}

// -- the rest of the hub

#[test]
fn what_landed_and_when_the_forge_was_read_pass_through() {
    let merged = HubMerged {
        number: 5,
        title: "t".into(),
        branch: "b/5".into(),
        url: "https://forge/pull/5".into(),
        author: None,
        merged_at: at(5),
        commit: Some("d".repeat(40)),
        main_run: None,
    };
    let view = only(MergeLineHub {
        merged: vec![merged.clone()],
        forge_read_at: Some(at(6)),
        quiet_since: Some(at(7)),
        ..Default::default()
    });
    assert_eq!((view.landed, view.read_at, view.quiet_since), (vec![merged], Some(at(6)), Some(at(7))));
}
