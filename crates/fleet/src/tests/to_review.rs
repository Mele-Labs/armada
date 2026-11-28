//! To review, written before the code: **one list of everything waiting on the person across a project's Jobs, each item
//! a row of its Job's Needs you, grouped by sub project, then feature, then neither, each group in the Jobs' own order
//! then oldest first, and each item naming the Job, its sub project, its features and the operation that answers it.**
//! `crates/ipc/operations/list_to_review.toml`.

use api::{Features, Sessions};
use ipc::{
    Instant, NeedsYouRow, SessionFeature, SessionId, SetWaitingFor, StartSubProject, ToReview, ToReviewAnswer as Answer, ToReviewCard, ToReviewHead as Head,
    ToReviewKind as Kind, WaitingAct, WaitingActKind as Act, WaitingInput, WaitingItem, WaitingOption, WaitingSource as Source,
};
use store::KeptSubProject;

use super::agent_starts::{rig, Rig};
use super::feature_library::{a_project, an_agent_for, held_slot, seeded, wrote, VOTING};
use crate::session_host::needs_you::needs_you_of;
use crate::session_host::to_review::{to_review_of, JobWaiting};

fn item(id: &str, source: Source, act: Option<(Act, &str)>, choices: &[&str], order: u32, since: &str) -> WaitingItem {
    WaitingItem {
        order,
        why: String::new(),
        id: id.into(),
        text: format!("text of {id}"),
        since: Instant::carried(since),
        source,
        act: act.map(|(kind, target)| WaitingAct { kind, target: target.into(), on: None }),
        options: choices.iter().map(|label| WaitingOption { label: (*label).into(), description: None, picture: None }).collect(),
        steps: Vec::new(),
        place: None,
        approves: false,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
        artifact: None,
        goal_met: None,
    }
}

fn feature(slug: &str, area_place: u32, place: u32) -> SessionFeature {
    SessionFeature {
        project: "shop".into(),
        slug: slug.into(),
        title: format!("Title of {slug}"),
        area: "takes".into(),
        area_title: "Takes".into(),
        area_place,
        place,
    }
}

fn job(id: &str, features: Vec<SessionFeature>, waiting: Vec<WaitingItem>) -> JobWaiting {
    JobWaiting { id: SessionId::carried(id), title: Some(format!("Job {id}")), features, waiting, projects: vec!["shop".into()], cards: Vec::new() }
}

fn sub(id: &str, jobs: &[&str], features: &[&str]) -> KeptSubProject {
    KeptSubProject {
        id: id.into(),
        project: "shop".into(),
        name: format!("Sub {id}"),
        goal: format!("Goal of {id}"),
        jobs: jobs.iter().map(|one| one.to_string()).collect(),
        features: features.iter().map(|one| one.to_string()).collect(),
        orchestrator: None,
        started_at: "2026-11-01T00:00:00Z".into(),
        not_yet_at: None,
        ended_at: None,
    }
}

/// Each item as (group head, job, item id).
fn read(list: &ToReview) -> Vec<(String, String, String)> {
    let head = |head: &Head| match head {
        Head::SubProject(sub) => format!("sub:{}", sub.id),
        Head::Feature(feature) => format!("feature:{}", feature.slug),
        Head::Neither => "neither".into(),
    };
    list.groups
        .iter()
        .flat_map(|group| group.items.iter().map(move |item| (head(&group.head), item.job.id.as_str().to_string(), item.row.id.clone())))
        .collect()
}

fn rows_by_id(mut rows: Vec<NeedsYouRow>) -> Vec<NeedsYouRow> {
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows
}

#[test]
fn every_item_of_a_job_is_one_item_whose_row_is_its_needs_you_row_with_its_kind_and_answer() {
    const AT: &str = "2026-11-20T10:00:00Z";
    let mut approves = item("approve", Source::Agent, None, &["Approve", "Ask for changes"], 0, AT);
    approves.approves = true;
    let waiting = vec![
        item("plan:3", Source::Plan, None, &[], 0, AT),
        item("sketch:Takes", Source::Sketch, None, &[], 0, AT),
        item("document:a.html", Source::Document, None, &[], 0, AT),
        item("draft:7", Source::Draft, None, &[], 0, AT),
        item("try:job-a", Source::ReadyToTry, Some((Act::Message, "job-a")), &[], 0, AT),
        item("conflict:a:b", Source::JobConflict, Some((Act::Answer, "conflict:a:b")), &["Tell a to change", "Tell b", "Hold both"], 0, AT),
        item("which", Source::Agent, None, &["One", "Two"], 0, AT),
        item("card", Source::AskCard, Some((Act::Answer, "card")), &["Allow"], 0, AT),
        item("buy", Source::Agent, None, &[], 0, AT),
        item("run", Source::Agent, Some((Act::Run, "make it")), &[], 0, AT),
        item("pr", Source::Agent, Some((Act::ApprovePr, "12")), &[], 0, AT),
        item("walk:http://x", Source::Walk, Some((Act::Walk, "http://x")), &[], 0, AT),
        item("first", Source::FirstMessage, None, &[], 0, AT),
        item("start:child", Source::HeldChild, Some((Act::Message, "child")), &[], 0, AT),
        item("done:1", Source::Done, Some((Act::Message, "job-a")), &[], 0, AT),
        approves,
    ];
    let cards = vec![ToReviewCard { branch: "armada/x".into(), tip: "abc".into(), about: "The pane".into() }];
    let mut one = job("job-a", Vec::new(), waiting.clone());
    one.cards = cards.clone();
    let list = to_review_of(Some("shop".into()), vec![one], &[]);

    let items: Vec<_> = list.groups.iter().flat_map(|group| group.items.iter()).collect();
    assert_eq!(items.len(), waiting.len(), "nothing dropped, nothing twice");
    let band = needs_you_of("job-a", &waiting).sections.into_iter().flat_map(|section| section.rows).collect();
    assert_eq!(rows_by_id(items.iter().map(|item| item.row.clone()).collect()), rows_by_id(band), "the band's own rows");

    let of = |id: &str| items.iter().find(|item| item.row.id == id).expect(id);
    let want = [
        ("plan:3", Kind::Plan, Answer::AnswerItems),
        ("sketch:Takes", Kind::Sketch, Answer::AnswerItems),
        ("document:a.html", Kind::Document, Answer::AnswerItems),
        ("draft:7", Kind::Draft, Answer::AnswerItems),
        ("try:job-a", Kind::ReadyToTry, Answer::AnswerReadyToTry { cards }),
        ("conflict:a:b", Kind::Clash, Answer::AnswerWaiting),
        ("which", Kind::Question, Answer::AnswerWaiting),
        ("card", Kind::Question, Answer::AnswerWaiting),
        ("buy", Kind::ToDo, Answer::DoneWaiting),
        ("run", Kind::Command, Answer::Press),
        ("pr", Kind::PullRequest, Answer::Press),
        ("walk:http://x", Kind::Walk, Answer::AnswerItems),
        ("first", Kind::Question, Answer::BeginSession { session_id: SessionId::carried("job-a") }),
        ("start:child", Kind::Question, Answer::BeginSession { session_id: SessionId::carried("child") }),
        ("done:1", Kind::Done, Answer::DismissWaiting),
        ("approve", Kind::Question, Answer::AnswerItems),
    ];
    for (id, kind, answer) in want {
        assert_eq!((of(id).kind, &of(id).answer), (kind, &answer), "{id}");
    }
    let first = of("plan:3");
    assert_eq!((first.job.id.as_str(), first.job.title.as_str(), first.project.as_deref()), ("job-a", "Job job-a", Some("shop")));
}

#[test]
fn groups_stand_sub_project_then_feature_then_neither_and_items_by_the_jobs_order_then_oldest() {
    let ask = |id: &str, order: u32, since: &str| item(id, Source::Agent, None, &["Yes", "No"], order, since);
    let jobs = vec![
        // Picked for the first sub project: its second ask is older, and still after its first.
        job("picked", vec![feature("b", 0, 1)], vec![ask("p1", 1, "2026-11-20T12:00:00Z"), ask("p2", 2, "2026-11-20T08:00:00Z")]),
        // In the same sub project too, with a first ask older than the other Job's.
        job("partner", Vec::new(), vec![ask("q1", 1, "2026-11-20T09:00:00Z")]),
        // Writes to a feature of the second sub project, so it joins it.
        job("joined", vec![feature("c", 1, 0)], vec![ask("j1", 1, "2026-11-20T10:00:00Z")]),
        // Writes to two features: listed under the first in the library's order.
        job("two", vec![feature("b", 0, 1), feature("a", 0, 0)], vec![ask("t1", 1, "2026-11-20T10:00:00Z")]),
        job("bee", vec![feature("b", 0, 1)], vec![ask("b1", 1, "2026-11-20T10:00:00Z")]),
        job("loose", Vec::new(), vec![ask("l1", 1, "2026-11-20T10:00:00Z")]),
        job("idle", Vec::new(), Vec::new()),
        // The second sub project's orchestrator, which none picked, asks whether its goal is met.
        job("orchestrates", Vec::new(), vec![item("goal_met:s2", Source::GoalMet, Some((Act::Answer, "goal_met:s2")), &["Agree", "Not yet"], 1, "2026-11-20T11:00:00Z")]),
    ];
    let mut second = sub("s2", &[], &["c"]);
    second.orchestrator = Some("orchestrates".into());
    let subs = [sub("s1", &["partner", "picked"], &[]), second];
    let list = to_review_of(None, jobs, &subs);

    let at = |head: &str, job: &str, id: &str| (head.to_string(), job.to_string(), id.to_string());
    assert_eq!(
        read(&list),
        [
            at("sub:s1", "partner", "q1"),
            at("sub:s1", "picked", "p1"),
            at("sub:s1", "picked", "p2"),
            at("sub:s2", "joined", "j1"),
            at("sub:s2", "orchestrates", "goal_met:s2"),
            at("feature:a", "two", "t1"),
            at("feature:b", "bee", "b1"),
            at("neither", "loose", "l1"),
        ]
    );
    let picked = &list.groups[0].items[1];
    let named = picked.sub_project.as_ref().expect("its sub project");
    assert_eq!((named.id.as_str(), named.name.as_str(), named.goal.as_str()), ("s1", "Sub s1", "Goal of s1"));
    match &list.groups[1].head {
        Head::SubProject(head) => assert_eq!(head.goal, "Goal of s2", "the group is headed by its goal"),
        other => panic!("headed by {other:?}"),
    }
    let met = &list.groups[1].items[1];
    assert_eq!((met.kind, &met.answer), (Kind::GoalMet, &Answer::AnswerWaiting), "Agree or Not yet, answered in place");
    assert_eq!(picked.features, [feature("b", 0, 1)], "the Job's features ride on its item");
    assert!(list.groups.iter().all(|group| !group.items.is_empty()), "no group is empty");
}

async fn ask(rig: &Rig, job: &SessionId, id: &str) {
    let item = WaitingInput { approves: false, id: id.into(), text: format!("Which {id}?"), act: None, options: Vec::new(), steps: Vec::new(), artifact: None };
    rig.fleet.set_waiting(None, SetWaitingFor { items: vec![item], session_id: Some(job.clone()) }).await.expect("asked");
}

#[tokio::test]
async fn a_projects_live_jobs_are_listed_by_sub_project_and_feature_and_another_projects_only_when_none_is_named() {
    let rig = rig(None);
    let shop = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&shop)).await.expect("seeded");
    let voting = an_agent_for(&rig, &shop).await;
    wrote(&rig, &voting, "crates/pages/voting/ranks.rs").await;
    held_slot(&rig, &voting, "4").await;
    let grouped = an_agent_for(&rig, &shop).await;
    let garden = a_project(&rig).await;
    let elsewhere = an_agent_for(&rig, &garden).await;
    let sub = std::sync::Arc::clone(&rig.fleet)
        .start_sub_project(StartSubProject { project: shop.clone(), name: "Rank votes".into(), goal: "Votes rank takes".into(), jobs: vec![grouped.clone()] })
        .await
        .expect("started");
    for (job, id) in [(&voting, "voting"), (&grouped, "grouped"), (&elsewhere, "elsewhere")] {
        ask(&rig, job, id).await;
    }

    let list = rig.fleet.list_to_review(Some(shop.clone())).await.expect("listed");
    assert_eq!(list.project.as_deref(), Some(shop.as_str()));
    // The orchestrator start_sub_project started may wait on him too, under the same sub project.
    let mine = [voting.as_str(), grouped.as_str(), elsewhere.as_str()];
    let heads: Vec<String> = read(&list).into_iter().filter(|(_, job, _)| mine.contains(&job.as_str())).map(|(head, _, id)| format!("{head} {id}")).collect();
    assert_eq!(heads, [format!("sub:{} grouped", sub.id), format!("feature:{VOTING} voting")], "Garden's Job is not Shop's");
    match &list.groups[0].head {
        Head::SubProject(head) => assert_eq!((head.id.as_str(), head.goal.as_str()), (sub.id.as_str(), "Votes rank takes")),
        other => panic!("headed by {other:?}"),
    }
    let item = list.groups.iter().flat_map(|group| &group.items).find(|item| item.job.id == voting).expect("the voting Job's item");
    assert_eq!((item.job.id.clone(), item.kind, &item.answer), (voting.clone(), Kind::ToDo, &Answer::DoneWaiting));
    assert_eq!(item.features.iter().map(|one| one.slug.as_str()).collect::<Vec<_>>(), [VOTING]);
    let band = rig.fleet.get_needs_you(None, Some(voting.clone())).await.expect("read");
    assert_eq!(vec![item.row.clone()], band.sections.into_iter().flat_map(|section| section.rows).collect::<Vec<_>>(), "To review and Needs you agree");

    let every = rig.fleet.list_to_review(None).await.expect("listed");
    assert_eq!(every.project, None);
    let jobs: Vec<String> = read(&every).into_iter().map(|(_, job, _)| job).collect();
    for job in [&voting, &grouped, &elsewhere] {
        assert!(jobs.contains(&job.as_str().to_string()), "{} is listed: {jobs:?}", job.as_str());
    }
}
