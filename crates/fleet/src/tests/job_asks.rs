//! `raise_conflict` and `ask_held`: what Fleet refuses, what it answers itself, where an ask stands while the
//! Operator is off, what the Operator may tell a Job or raise to Your turn, and how a second conflict folds.

use std::sync::Arc;

use adapter_traits::PullRequestStanding;
use api::{HostedSessions, Sessions};
use ipc::{AnswerWaiting, AskHeld, CloseSession, RaiseConflict, SessionId, WaitingSource};

use super::operator_raises::{page, queued, seated};
use super::operator_sources::on;
use super::pull_requesting::facts;
use super::rule_acting::notices_in;
use super::session_host::{eventually, rig_placing, Rig};
use crate::operator::keep_the_raises;
use crate::session_host::address_of;

fn address(job: &SessionId) -> String {
    address_of(job.as_str())
}

fn conflict(from: &SessionId, with: &SessionId) -> RaiseConflict {
    RaiseConflict {
        with: address(with),
        files: vec!["crates/ipc/src/operator_page.rs".into()],
        wire: vec!["OperatorTurnItem".into()],
        mine: "Adds a field to the card.".into(),
        theirs: "Renames the card.".into(),
        breaks: "The native card no longer reads the item.".into(),
        session_id: Some(from.clone()),
    }
}

fn holding(from: &SessionId, pull: Option<u64>, branch: Option<&str>) -> AskHeld {
    AskHeld { pull, branch: branch.map(str::to_string), session_id: Some(from.clone()) }
}

/// Two live Jobs, the first the sender.
async fn two(rig: &Rig) -> (SessionId, SessionId) {
    (rig.start_titled(Some("Add the field")).await, rig.start_titled(Some("Rename the card")).await)
}

async fn items(rig: &Rig, job: &SessionId, source: WaitingSource) -> Vec<ipc::WaitingItem> {
    Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session.waiting_for.into_iter().filter(|one| one.source == source).collect()
}

async fn notes_to(rig: &Rig, job: &SessionId) -> Vec<String> {
    notices_in(rig, job).await.into_iter().map(|(_, text)| text).collect()
}

async fn chose(rig: &Rig, job: &SessionId, item: &str, choice: u32) {
    let answer = AnswerWaiting { session_id: job.clone(), item_id: item.into(), choice: Some(choice), text: None, mode: None, fixed: None };
    Arc::clone(&rig.fleet).answer_waiting(answer).await.expect("answered");
}

async fn refusal(rig: &Rig, asked: RaiseConflict) -> String {
    let refused = Arc::clone(&rig.fleet).raise_conflict(None, asked).await.unwrap_err();
    assert_eq!((refused.error().code.as_str(), refused.status()), ("fleet.conflict_refused", 422), "{refused:?}");
    refused.error().message.clone()
}

#[tokio::test]
async fn a_conflict_naming_the_sender_is_refused() {
    let rig = rig_placing(None);
    let (one, _) = two(&rig).await;
    let said = refusal(&rig, RaiseConflict { with: address(&one), ..conflict(&one, &one) }).await;
    assert!(said.contains("`with`") && said.contains("own address"), "{said}");
}

#[tokio::test]
async fn a_conflict_with_an_ended_or_an_unknown_job_is_refused() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: two.clone() }).await.expect("ended");

    let ended = refusal(&rig, conflict(&one, &two)).await;
    assert!(ended.contains(&address(&two)) && ended.contains("ended"), "{ended}");
    let unknown = refusal(&rig, RaiseConflict { with: "s-00000000".into(), ..conflict(&one, &two) }).await;
    assert!(unknown.contains("s-00000000") && unknown.contains("no Job"), "{unknown}");
    let not_an_address = refusal(&rig, RaiseConflict { with: "the other Job".into(), ..conflict(&one, &two) }).await;
    assert!(not_an_address.contains("no Job's address"), "{not_an_address}");
    assert!(items(&rig, &one, WaitingSource::JobConflict).await.is_empty());
}

#[tokio::test]
async fn a_conflict_with_no_files_and_no_wire_is_refused() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    let said = refusal(&rig, RaiseConflict { files: vec![" ".into()], wire: Vec::new(), ..conflict(&one, &two) }).await;
    assert!(said.contains("`files`") && said.contains("`wire`"), "{said}");
}

#[tokio::test]
async fn a_conflict_missing_what_either_work_does_or_what_breaks_is_refused_naming_the_part() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    for part in ["mine", "theirs", "breaks"] {
        let mut asked = conflict(&one, &two);
        match part {
            "mine" => asked.mine = String::new(),
            "theirs" => asked.theirs = " ".into(),
            _ => asked.breaks = String::new(),
        }
        let said = refusal(&rig, asked).await;
        assert!(said.contains(&format!("`{part}`")), "{part}: {said}");
    }
    assert!(items(&rig, &one, WaitingSource::JobConflict).await.is_empty());
}

#[tokio::test]
async fn a_landing_question_naming_neither_or_both_is_refused() {
    let rig = rig_placing(None);
    let one = rig.start().await;
    for (asked, part) in [(holding(&one, None, None), "neither"), (holding(&one, Some(5), Some("a-branch")), "both")] {
        let refused = Arc::clone(&rig.fleet).ask_held(None, asked).await.unwrap_err();
        assert_eq!((refused.error().code.as_str(), refused.status()), ("fleet.held_refused", 422));
        assert!(refused.error().message.contains(part), "{part}: {}", refused.error().message);
    }
}

#[tokio::test]
async fn a_landing_question_about_a_pull_request_or_a_branch_the_repository_lacks_is_refused() {
    let rig = rig_placing(None);
    let one = rig.start().await;
    rig.fleet.vcs().now_pull_request(None);

    let pull = Arc::clone(&rig.fleet).ask_held(None, holding(&one, Some(99), None)).await.unwrap_err();
    assert_eq!(pull.error().code, "fleet.held_refused");
    assert!(pull.error().message.contains("#99"), "{}", pull.error().message);
    let branch = Arc::clone(&rig.fleet).ask_held(None, holding(&one, None, Some("no-such-branch"))).await.unwrap_err();
    assert_eq!(branch.error().code, "fleet.held_refused");
    assert!(branch.error().message.contains("no-such-branch"), "{}", branch.error().message);
    assert!(items(&rig, &one, WaitingSource::JobHeld).await.is_empty());
}

#[tokio::test]
async fn a_merged_or_closed_pull_request_is_answered_by_fleet_to_the_job_with_no_call_and_nothing_is_kept() {
    let rig = rig_placing(None);
    let one = rig.start().await;
    on(&rig).await;
    let call = seated(&rig, &[]).await;

    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Merged)));
    let merged = Arc::clone(&rig.fleet).ask_held(None, holding(&one, Some(12), None)).await.expect("answered");
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Closed)));
    let closed = Arc::clone(&rig.fleet).ask_held(None, holding(&one, Some(13), None)).await.expect("answered");

    assert_eq!(merged.answer.as_deref(), Some("held? #12: merged."), "no merge time is kept for it, so none is told");
    assert_eq!(closed.answer.as_deref(), Some("held? #13: closed."));
    let told = notes_to(&rig, &one).await;
    assert!(told.iter().any(|text| text == "held? #12: merged.") && told.iter().any(|text| text == "held? #13: closed."), "{told:?}");
    rig.fleet.sources_handed().await;
    assert!(call.asked().is_empty(), "the Operator was never called");
    assert!(rig.fleet.store().lock().await.open_job_ask(&merged.id).expect("read").is_none());
    assert!(items(&rig, &one, WaitingSource::JobHeld).await.is_empty());
}

#[tokio::test]
async fn a_merge_the_timeline_kept_is_told_with_its_time() {
    let rig = rig_placing(None);
    let one = rig.start().await;
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Merged)));
    {
        let mut store = rig.fleet.store().lock().await;
        let merged = store::TimelineEntry {
            at: "2026-11-14T14:05:00.000Z".to_string(),
            kind: "pr.merged".to_string(),
            subject_kind: "pull_request".to_string(),
            subject_id: "12".to_string(),
            subject_url: None,
            title: None,
            repo: Some(rig.manifest.as_str().to_string()),
            tone: "idle".to_string(),
            says: "merged".to_string(),
            detail: None,
            items: Vec::new(),
            service: None,
            service_name: None,
            to_id: None,
            live: false,
            over: false,
        };
        store.append_timeline(&merged).expect("kept");
    }

    let answered = Arc::clone(&rig.fleet).ask_held(None, holding(&one, Some(12), None)).await.expect("answered");

    let said = answered.answer.expect("answered by Fleet");
    assert!(said.starts_with("held? #12: merged at ") && said.ends_with('.'), "{said}");
}

#[tokio::test]
async fn while_the_operator_is_off_a_conflict_stands_in_the_senders_needs_you_with_three_buttons() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;

    let raised = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");

    let mut pair = [address(&one), address(&two)];
    pair.sort();
    assert_eq!((raised.id.as_str(), raised.folded), (format!("conflict:{}:{}", pair[0], pair[1]).as_str(), false));
    let held = items(&rig, &one, WaitingSource::JobConflict).await;
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0].id, raised.id);
    assert_eq!(held[0].text, "The native card no longer reads the item.");
    assert_eq!(
        held[0].options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(),
        ["Tell Add the field to change", "Tell Rename the card", "Hold both"]
    );
    assert!(items(&rig, &two, WaitingSource::JobConflict).await.is_empty(), "only the sender holds it");
}

#[tokio::test]
async fn telling_the_sender_to_change_tells_it_the_conflict_and_the_other_job_nothing() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    let raised = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");

    chose(&rig, &one, &raised.id, 0).await;

    let told = notes_to(&rig, &one).await;
    let said = told.iter().find(|text| text.contains("He says your side changes.")).expect("the sender is told");
    for part in ["Conflict with Rename the card", "The native card no longer reads the item.", "Yours: Adds a field to the card.", "Theirs: Renames the card."] {
        assert!(said.contains(part), "{part}: {said}");
    }
    assert!(notes_to(&rig, &two).await.iter().all(|text| !text.contains("Conflict with")), "the other Job is told nothing");
    assert!(items(&rig, &one, WaitingSource::JobConflict).await.is_empty(), "the item goes");
}

#[tokio::test]
async fn telling_the_other_job_to_change_tells_it_from_its_own_side() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    let raised = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");

    chose(&rig, &one, &raised.id, 1).await;

    let told = notes_to(&rig, &two).await;
    let said = told.iter().find(|text| text.contains("He says your side changes.")).expect("the other Job is told");
    assert!(said.contains("Conflict with Add the field") && said.contains("Yours: Renames the card.") && said.contains("Theirs: Adds a field to the card."), "{said}");
    assert!(notes_to(&rig, &one).await.iter().all(|text| !text.contains("Conflict with")));
}

#[tokio::test]
async fn holding_both_tells_both_jobs_to_hold_landing() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    let raised = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");

    chose(&rig, &one, &raised.id, 2).await;

    for job in [&one, &two] {
        assert!(notes_to(&rig, job).await.iter().any(|text| text.contains("He says hold landing until he rules.")), "{job:?}");
    }
    // A second press settles nothing: the first answer won.
    let again = AnswerWaiting { session_id: one.clone(), item_id: raised.id.clone(), choice: Some(0), text: None, mode: None, fixed: None };
    assert!(Arc::clone(&rig.fleet).answer_waiting(again).await.is_err());
}

#[tokio::test]
async fn words_alone_and_a_dismissal_do_not_answer_a_conflict() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    let raised = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");

    let words = AnswerWaiting { session_id: one.clone(), item_id: raised.id.clone(), choice: None, text: Some("just do it".into()), mode: None, fixed: None };
    assert!(Arc::clone(&rig.fleet).answer_waiting(words).await.is_err());
    assert_eq!(items(&rig, &one, WaitingSource::JobConflict).await.len(), 1, "it stands");
}

#[tokio::test]
async fn a_second_conflict_between_the_same_two_jobs_folds_into_the_open_one_whoever_raises_it() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    let first = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");

    let second = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&two, &one)).await.expect("raised");

    assert_eq!((second.id.as_str(), second.folded), (first.id.as_str(), true));
    assert_eq!(items(&rig, &one, WaitingSource::JobConflict).await.len(), 1, "nothing new is shown");
    assert!(items(&rig, &two, WaitingSource::JobConflict).await.is_empty());
    let kept = rig.fleet.store().lock().await.open_job_ask(&first.id).expect("read").expect("open");
    assert_eq!(kept.also_raised_by, [format!("Rename the card ({})", address(&two))]);
    assert_eq!(kept.session_id, one.as_str(), "it stays the first one's");
}

#[tokio::test]
async fn a_conflict_asked_again_after_it_was_answered_is_a_new_one() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    let first = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");
    chose(&rig, &one, &first.id, 2).await;

    let again = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");

    assert!(!again.folded);
    assert_eq!(items(&rig, &one, WaitingSource::JobConflict).await.len(), 1);
}

#[tokio::test]
async fn while_the_operator_is_off_a_landing_question_stands_in_the_askers_needs_you_and_its_answer_reaches_the_asker() {
    let rig = rig_placing(None);
    let one = rig.start().await;
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open)));

    let asked = Arc::clone(&rig.fleet).ask_held(None, holding(&one, Some(14), None)).await.expect("asked");

    assert_eq!(asked.answer, None);
    let held = items(&rig, &one, WaitingSource::JobHeld).await;
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!((held[0].id.as_str(), held[0].text.as_str()), (asked.id.as_str(), "Should #14 land?"));
    assert_eq!(held[0].options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(), ["Land it", "Hold it"]);

    chose(&rig, &one, &asked.id, 1).await;

    assert!(notes_to(&rig, &one).await.iter().any(|text| text == "held? #14: He says hold it."));
    assert!(items(&rig, &one, WaitingSource::JobHeld).await.is_empty());
}

#[tokio::test]
async fn a_branch_is_asked_about_and_land_it_tells_the_asker() {
    let rig = rig_placing(None);
    let one = rig.start().await;
    let branch = "fleet/add-the-field".to_string();
    rig.fleet.vcs().move_ref_to(branch.clone(), "c1");

    let asked = Arc::clone(&rig.fleet).ask_held(None, holding(&one, None, Some(branch.as_str()))).await.expect("asked");
    let again = Arc::clone(&rig.fleet).ask_held(None, holding(&one, None, Some(branch.as_str()))).await.expect("asked");

    assert_eq!(asked.id, again.id);
    assert_eq!(items(&rig, &one, WaitingSource::JobHeld).await.len(), 1, "asked again while open, it is one");
    chose(&rig, &one, &asked.id, 0).await;
    assert!(notes_to(&rig, &one).await.iter().any(|text| text == &format!("held? {branch}: He says land it.")));
}

#[tokio::test]
async fn while_the_operator_is_on_a_conflict_it_raises_reaches_your_turn_with_both_jobs_and_what_breaks() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    on(&rig).await;
    let call = seated(&rig, &[Some("item: 1\nway: 2\nbecause: both Jobs write the same card"), None]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let raised = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");
    assert!(items(&rig, &one, WaitingSource::JobConflict).await.is_empty(), "it waits for the Operator, not for the Job's Needs you");

    rig.fleet.sources_handed().await;

    let turn = queued(&rig, &one, &raised.id).await;
    assert_eq!(turn.item.source, WaitingSource::JobConflict);
    let card = turn.conflict.expect("the card's fields");
    assert_eq!((card.raised_by.address.as_str(), card.raised_by.title.as_str()), (address(&one).as_str(), "Add the field"));
    assert_eq!((card.with.address.as_str(), card.with.title.as_str()), (address(&two).as_str(), "Rename the card"));
    assert_eq!((card.files.as_slice(), card.wire.as_slice()), (["crates/ipc/src/operator_page.rs".to_string()].as_slice(), ["OperatorTurnItem".to_string()].as_slice()));
    assert_eq!((card.mine.as_str(), card.theirs.as_str(), card.breaks.as_str()), ("Adds a field to the card.", "Renames the card.", "The native card no longer reads the item."));
    assert!(turn.held.is_none());
    assert_eq!(turn.item.options.len(), 3);
    let asked = call.asked();
    assert!(asked[0].contains("A Job raised a conflict with another Job") && asked[0].contains("If both land: The native card no longer reads the item."), "{}", asked[0]);
    assert!(asked[0].contains("The ways it is offered: 1. tell:") && asked[0].contains("2. raise"), "{}", asked[0]);
}

#[tokio::test]
async fn while_the_operator_is_on_it_may_tell_the_job_whose_side_changes_and_nothing_reaches_your_turn() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    on(&rig).await;
    let said = format!("item: 1\nway: 1\njob: {}\nbecause: the second Job renamed the card first", two.as_str());
    seated(&rig, &[Some(&said)]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let raised = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");

    rig.fleet.sources_handed().await;

    eventually(|| async { notes_to(&rig, &two).await.iter().any(|text| text.contains("The Operator says your side changes.") && text.contains("the second Job renamed the card first")) }).await;
    assert!(notes_to(&rig, &one).await.iter().all(|text| !text.contains("Conflict with")));
    assert!(rig.fleet.store().lock().await.open_job_ask(&raised.id).expect("read").is_none(), "it is answered");
    assert!(page(&rig).await.turn.iter().all(|turn| turn.item.id != raised.id));
}

#[tokio::test]
async fn a_conflict_the_operator_could_not_rule_goes_to_the_senders_needs_you_so_it_is_never_lost() {
    let rig = rig_placing(None);
    let (one, two) = two(&rig).await;
    on(&rig).await;
    seated(&rig, &[None, None]).await;
    let raised = Arc::clone(&rig.fleet).raise_conflict(None, conflict(&one, &two)).await.expect("raised");

    rig.fleet.sources_handed().await;

    eventually(|| async { items(&rig, &one, WaitingSource::JobConflict).await.iter().any(|item| item.id == raised.id) }).await;
}

#[tokio::test]
async fn while_the_operator_is_on_a_landing_question_it_raises_reaches_your_turn_with_the_pull_request() {
    let rig = rig_placing(None);
    let one = rig.start_titled(Some("Add the field")).await;
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open)));
    on(&rig).await;
    let call = seated(&rig, &[Some("item: 1\nway: 2\nbecause: no ruling covers it"), None]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let asked = Arc::clone(&rig.fleet).ask_held(None, holding(&one, Some(14), None)).await.expect("asked");

    rig.fleet.sources_handed().await;

    let turn = queued(&rig, &one, &asked.id).await;
    let held = turn.held.expect("the card's fields");
    assert_eq!((held.pull, held.branch.as_deref(), held.title.as_deref()), (Some(14), None, Some("Take a draft out")));
    assert_eq!((held.asked_by.address.as_str(), held.asked_by.title.as_str()), (address(&one).as_str(), "Add the field"));
    assert!(turn.conflict.is_none());
    assert_eq!(turn.item.options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(), ["Land it", "Hold it"]);
    assert!(call.asked()[0].contains("asks whether #14 \"Take a draft out\" should land"), "{}", call.asked()[0]);
}
