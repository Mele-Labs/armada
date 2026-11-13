//! The claim, written before the code: **a Job's Needs you is worked out once, in Fleet. Every `waiting_for`
//! source is one row of exactly one section, each row says what pressing it does and how it is put away, and
//! `dismiss_waiting` takes every row that says it can be. A Job takes back a Document it wrote with
//! `withdraw_document`, which leaves the file, tells nobody and is undone by a new revision.**
//! `crates/ipc/operations/get_needs_you.toml` and `withdraw_document.toml`.

use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    DismissWaiting, Instant, NeedsYou, NeedsYouAct, NeedsYouDismiss as Dismiss, NeedsYouSectionKind as Section, SessionId, WaitingAct,
    WaitingActKind, WaitingItem, WaitingOption, WaitingSource as Source, WithdrawDocument,
};

use super::document_answers::{one_page, record_of, revision_kept, sent, waits, written};
use super::session_host::{rig_placing, Rig};
use crate::session_host::needs_you::{needs_you_of, refused_dismissal};

const JOB: &str = "job-1";

fn item(id: &str, source: Source, act: Option<(WaitingActKind, &str)>, choices: &[&str], steps: &[&str]) -> WaitingItem {
    WaitingItem {
        id: id.into(),
        text: format!("text of {id}"),
        since: Instant::carried("2026-11-10T10:00:00Z"),
        source,
        act: act.map(|(kind, target)| WaitingAct { kind, target: target.into() }),
        options: choices.iter().map(|label| WaitingOption { label: (*label).into(), description: None, picture: None }).collect(),
        steps: steps.iter().map(|one| (*one).into()).collect(),
        place: None,
        approves: false,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
    }
}

/// One source as Fleet raises it, the section it must land in and how it is put away. `by_act` marks a
/// source whose section the table leaves to what pressing it does.
struct Case {
    item: WaitingItem,
    section: Section,
    dismiss: Dismiss,
    by_act: bool,
}

fn case(item: WaitingItem, section: Section, dismiss: Dismiss, by_act: bool) -> Case {
    Case { item, section, dismiss, by_act }
}

/// Every source once. `counted` fails to build for a source not named here.
fn cases() -> Vec<Case> {
    use WaitingActKind as Act;
    let own = (Act::Message, JOB);
    vec![
        case(item("a1", Source::Agent, None, &[], &[]), Section::ToDo, Dismiss::Reason, true),
        case(item("ask:1", Source::AskCard, Some((Act::Answer, "ask:1")), &["Yes", "No"], &[]), Section::Questions, Dismiss::None, false),
        case(item("walk:http://localhost:5173/", Source::Walk, Some((Act::Walk, "http://localhost:5173/")), &[], &[]), Section::Walks, Dismiss::Plain, false),
        case(item("perm:1", Source::Permission, Some((Act::Answer, "perm:1")), &["Allow", "Deny"], &[]), Section::Questions, Dismiss::None, false),
        case(item("window_ask:r1", Source::WindowAsk, Some((Act::Walk, "http://localhost:5173/")), &["This", "That"], &[]), Section::Questions, Dismiss::None, false),
        case(item("draft:r1", Source::Draft, Some((Act::Answer, "draft:r1")), &["Approve"], &[]), Section::ToReview, Dismiss::Reason, false),
        case(item("sketch:Flow", Source::Sketch, Some(own), &[], &[]), Section::ToReview, Dismiss::Plain, false),
        case(item("document:/w/a.html", Source::Document, Some(own), &[], &[]), Section::ToReview, Dismiss::Plain, false),
        case(item("first", Source::FirstMessage, Some(own), &[], &[]), Section::Questions, Dismiss::None, false),
        case(item("replied:1", Source::Replied, None, &[], &[]), Section::Questions, Dismiss::NotAQuestion, true),
        case(item("done:1", Source::Done, None, &[], &[]), Section::Done, Dismiss::Plain, false),
        case(item("plan:2", Source::Plan, Some(own), &[], &[]), Section::ToReview, Dismiss::Plain, false),
        case(item("slot-choice", Source::SlotChoice, Some((Act::Answer, "slot-choice")), &["Wait for a slot", "Add a slot"], &[]), Section::Questions, Dismiss::None, false),
        case(item("delivery", Source::Delivery, Some((Act::Answer, "delivery")), &["Auto land", "Open PR only", "Keep local"], &[]), Section::Questions, Dismiss::None, false),
        case(item("words:m:x", Source::WordsAddress, Some((Act::Answer, "words:m:x")), &["Other"], &[]), Section::Questions, Dismiss::Plain, false),
        case(item("call:7", Source::FleetCall, Some(own), &[], &[]), Section::Questions, Dismiss::Plain, false),
        case(item("task_done:p", Source::TaskDone, Some((Act::Answer, "task_done:p")), &["Mark done", "Keep open"], &[]), Section::Questions, Dismiss::None, false),
        case(item("watch_not_ours:w", Source::WatchNotOurs, Some((Act::Answer, "w")), &["Ignore", "Fix it"], &[]), Section::Questions, Dismiss::Plain, false),
        case(item("start:child", Source::HeldChild, Some((Act::Message, "child")), &[], &[]), Section::Questions, Dismiss::None, false),
        case(item("preview:b@t", Source::PreviewOffer, Some((Act::Run, "scripts/preview --restart")), &[], &[]), Section::Commands, Dismiss::Skip, true),
        case(item("rule_act:r", Source::RuleAct, Some((Act::Answer, "rule_act:r")), &["Do it", "Skip"], &[]), Section::Questions, Dismiss::Skip, false),
        case(item("suggestion:1", Source::OperatorSuggestion, Some((Act::Answer, "suggestion:1")), &["Do it", "Skip"], &[]), Section::Questions, Dismiss::Skip, false),
        case(item("browser_ask:t", Source::BrowserAsk, Some((Act::Answer, "browser_ask:t")), &["Allow", "Block"], &[]), Section::Questions, Dismiss::Plain, false),
        case(item("gate:g", Source::Gate, Some((Act::Answer, JOB)), &["Approve", "Skip this group"], &[]), Section::Questions, Dismiss::None, false),
    ]
}

/// A number for each source, in a match with no fallback, so adding a source breaks this build until `cases` names it.
fn counted(source: Source) -> usize {
    match source {
        Source::Agent => 0,
        Source::AskCard => 1,
        Source::Walk => 2,
        Source::Permission => 3,
        Source::WindowAsk => 4,
        Source::Draft => 5,
        Source::Sketch => 6,
        Source::Document => 7,
        Source::FirstMessage => 8,
        Source::Replied => 9,
        Source::Done => 10,
        Source::Plan => 11,
        Source::SlotChoice => 12,
        Source::Delivery => 13,
        Source::WordsAddress => 14,
        Source::FleetCall => 15,
        Source::TaskDone => 16,
        Source::WatchNotOurs => 17,
        Source::HeldChild => 18,
        Source::PreviewOffer => 19,
        Source::RuleAct => 20,
        Source::OperatorSuggestion => 21,
        Source::BrowserAsk => 22,
        Source::Gate => 23,
    }
}

fn view_of(items: &[WaitingItem]) -> NeedsYou {
    needs_you_of(JOB, items)
}

fn row_of<'a>(view: &'a NeedsYou, id: &str) -> (Section, &'a ipc::NeedsYouRow) {
    let found: Vec<_> = view
        .sections
        .iter()
        .flat_map(|section| section.rows.iter().map(move |row| (section.kind, row)))
        .filter(|(_, row)| row.id == id)
        .collect();
    assert_eq!(found.len(), 1, "`{id}` is one row of one section: {view:#?}");
    found[0]
}

fn wire(section: Section) -> &'static str {
    match section {
        Section::Questions => "questions",
        Section::ToDo => "to_do",
        Section::ToReview => "to_review",
        Section::Walks => "walks",
        Section::Commands => "commands",
        Section::PullRequests => "pull_requests",
        Section::Restart => "restart",
        Section::Done => "done",
    }
}

fn dismiss_word(dismiss: Dismiss) -> &'static str {
    match dismiss {
        Dismiss::None => "none",
        Dismiss::Plain => "plain",
        Dismiss::Skip => "skip",
        Dismiss::Reason => "reason",
        Dismiss::NotAQuestion => "not_a_question",
    }
}

#[test]
fn needs_you_places_every_source_in_one_section_and_says_how_it_is_put_away() {
    let all = cases();
    let mut seen: Vec<usize> = all.iter().map(|one| counted(one.item.source)).collect();
    seen.sort_unstable();
    assert_eq!(seen, (0..24).collect::<Vec<_>>(), "every source is named once");

    let items: Vec<WaitingItem> = all.iter().map(|one| one.item.clone()).collect();
    let view = view_of(&items);
    let rows: usize = view.sections.iter().map(|section| section.rows.len()).sum();
    assert_eq!(rows, items.len(), "nothing is dropped and nothing is twice");
    for one in &all {
        let (section, row) = row_of(&view, &one.item.id);
        assert_eq!(section, one.section, "{:?}", one.item.source);
        assert_eq!(row.dismiss, one.dismiss, "{:?}", one.item.source);
        assert_eq!(row.source, one.item.source);
        assert_eq!(row.text, one.item.text);
    }
    let order: Vec<Section> = view.sections.iter().map(|section| section.kind).collect();
    assert_eq!(order, [Section::Questions, Section::ToDo, Section::ToReview, Section::Walks, Section::Commands, Section::Done]);
    let labels: Vec<&str> = view.sections.iter().map(|section| section.label.as_str()).collect();
    assert_eq!(labels, ["Questions", "To do", "To review", "Walks", "Commands", "Done"]);
}

/// The doc states the table again, and says what the code does.
#[test]
fn needs_you_table_in_the_doc_says_what_the_code_does_for_each_source() {
    let doc = include_str!("../../../../docs/concepts/session/waiting-on-the-person.md");
    for one in cases() {
        let name = ipc::encode(&one.item.source).unwrap().trim_matches('"').to_string();
        let line = doc
            .lines()
            .find(|line| line.starts_with(&format!("| `{name}` |")))
            .unwrap_or_else(|| panic!("the doc's table has a row for `{name}`"));
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let section = if one.by_act { "by act" } else { wire(one.section) };
        assert!(cells[2].contains(section), "{name}: section `{section}` in {line}");
        assert!(cells[3].contains(dismiss_word(one.dismiss)), "{name}: dismiss `{}` in {line}", dismiss_word(one.dismiss));
    }
}

#[test]
fn needs_you_offers_no_dismissal_where_fleet_refuses_it() {
    let items: Vec<WaitingItem> = cases().into_iter().map(|one| one.item).collect();
    let view = view_of(&items);
    let mut refused = 0;
    for item in &items {
        let (_, row) = row_of(&view, &item.id);
        if refused_dismissal(item.source).is_some() {
            refused += 1;
            assert_eq!(row.dismiss, Dismiss::None, "{:?} is answered, not dismissed", item.source);
        }
    }
    assert_eq!(refused, 5, "delivery, slot_choice, task_done, held_child and gate");
}

#[test]
fn needs_you_fold_is_false_with_only_pages_owed_a_restart_or_a_done() {
    let page = |id, source| item(id, source, Some((WaitingActKind::Message, JOB)), &[], &[]);
    let pages = [page("plan:1", Source::Plan), page("sketch:A", Source::Sketch), page("document:/w/a.html", Source::Document)];
    let view = view_of(&pages);
    assert!(!view.fold, "only review pages are owed: no list to open");
    assert!(view.review_in_band, "more than one page");
    assert_eq!(view.glyphs.iter().map(|one| one.said.as_str()).collect::<Vec<_>>(), ["Plan", "Sketch", "Document"]);

    let restart = item("a2", Source::Agent, Some((WaitingActKind::Run, "scripts/preview --restart")), &[], &[]);
    let done = item("done:1", Source::Done, None, &[], &[]);
    let view = view_of(&[restart.clone(), done]);
    assert!(!view.fold, "a restart is the band's own key and a done is not listed");
    assert_eq!(view.sections.iter().map(|section| section.kind).collect::<Vec<_>>(), [Section::Restart, Section::Done]);
    assert!(view.glyphs.is_empty());

    let asks = item("a3", Source::Agent, None, &[], &[]);
    let view = view_of(&[pages[0].clone(), asks]);
    assert!(view.fold, "a question gives the band a list");
    assert!(!view.review_in_band, "one plan stays on the thread's card");
    assert_eq!(view.glyphs.len(), 1, "the question's mark, not the plan's");
    assert!(!view_of(&[]).fold);
}

#[test]
fn needs_you_review_is_in_the_band_for_one_document_alone_but_not_one_plan() {
    let page = |id, source| item(id, source, Some((WaitingActKind::Message, JOB)), &[], &[]);
    let document = view_of(&[page("document:/w/a.html", Source::Document)]);
    assert!(document.review_in_band && !document.fold);
    let plan = view_of(&[page("plan:1", Source::Plan)]);
    assert!(!plan.review_in_band && !plan.fold);
    assert_eq!(plan.sections.len(), 1, "it is still listed under To review");
    let and_more = view_of(&[page("document:/w/a.html", Source::Document), item("a", Source::Agent, None, &[], &[])]);
    assert!(!and_more.review_in_band, "a Document is the only thing owed in the band only when it is the only thing");
}

#[test]
fn needs_you_acts_read_as_the_bridges_read_them() {
    let sections = |items: &[WaitingItem]| view_of(items);
    let command = item("run", Source::Agent, Some((WaitingActKind::Run, "cargo xtask verify-foundations")), &[], &[]);
    let steps = item("steps", Source::Agent, Some((WaitingActKind::Run, "make")), &[], &["Stop it", "`make`"]);
    let follow = item("follow", Source::Agent, None, &[], &["Restart the Mac", "Open Files and Folders"]);
    let pr = item("pr", Source::Agent, Some((WaitingActKind::ApprovePr, "#12")), &[], &[]);
    let walk = item("walk:u", Source::Walk, Some((WaitingActKind::Walk, "http://localhost:5173/")), &[], &[]);
    let words = item("words", Source::Agent, Some((WaitingActKind::Answer, "")), &[], &[]);
    let pick = item("pick", Source::Agent, Some((WaitingActKind::Answer, "")), &["A", "B"], &[]);
    let other = item("other", Source::Replied, Some((WaitingActKind::Message, "job-2")), &[], &[]);
    let offer = item("preview:b@t", Source::PreviewOffer, Some((WaitingActKind::Run, "scripts/preview --restart --adopt")), &[], &[]);
    let dry = item("dry", Source::Agent, Some((WaitingActKind::Run, "scripts/preview --restart --dry-run")), &[], &[]);
    let main = item("main", Source::Replied, Some((WaitingActKind::Run, "scripts/restart --main")), &[], &[]);
    let pr_read = item("replied:pr", Source::Replied, Some((WaitingActKind::ApprovePr, "#9")), &[], &[]);
    let walked = item("walked", Source::Agent, Some((WaitingActKind::Run, "scripts/preview --restart")), &[], &["Press Refresh preview", "Open the Jobs page"]);
    let view = sections(&[command, steps, follow, pr, walk, words, pick, other, offer, dry, main, pr_read, walked]);

    let (section, row) = row_of(&view, "run");
    assert_eq!((section, &row.act), (Section::Commands, &NeedsYouAct::Run { command: "cargo xtask verify-foundations".into() }));
    let (section, row) = row_of(&view, "steps");
    assert_eq!(section, Section::Commands);
    assert!(matches!(&row.act, NeedsYouAct::Steps { steps, command: Some(command) } if steps.len() == 2 && command == "make"), "{:?}", row.act);
    let (section, row) = row_of(&view, "follow");
    assert_eq!(section, Section::ToDo, "a procedure with no command is for him to follow");
    assert_eq!(row.act, NeedsYouAct::Done { steps: vec!["Restart the Mac".into(), "Open Files and Folders".into()] });
    assert_eq!(row_of(&view, "pr").0, Section::PullRequests);
    assert_eq!(row_of(&view, "walk:u").0, Section::Walks);
    let (section, row) = row_of(&view, "words");
    assert_eq!((section, &row.act), (Section::Questions, &NeedsYouAct::Reply), "an answer with no choices takes words");
    let (_, row) = row_of(&view, "pick");
    assert!(matches!(&row.act, NeedsYouAct::Answer { options, other: true } if options.len() == 2), "{:?}", row.act);
    let (section, row) = row_of(&view, "other");
    assert_eq!((section, &row.act), (Section::Questions, &NeedsYouAct::OpenJob { id: "job-2".into() }));

    let (section, row) = row_of(&view, "preview:b@t");
    assert_eq!(section, Section::Commands, "Fleet's offer may be put away, so it is listed with its sentence");
    assert_eq!(
        row.act,
        NeedsYouAct::Restart {
            label: "Refresh preview".into(),
            build: ipc::BuildSource::Preview,
            adopt: true,
            command: "scripts/preview --restart --adopt".into()
        }
    );
    let (section, row) = row_of(&view, "dry");
    assert_eq!((section, &row.act), (Section::Commands, &NeedsYouAct::Run { command: "scripts/preview --restart --dry-run".into() }));
    let (section, row) = row_of(&view, "main");
    assert_eq!(section, Section::Restart, "any other restart is the band's key");
    assert!(matches!(&row.act, NeedsYouAct::Restart { label, build: ipc::BuildSource::Main, .. } if label == "Update to main"));
    assert_eq!(row.glyph.said, "Update to main");
    let (section, row) = row_of(&view, "walked");
    assert_eq!(section, Section::Restart, "a restart with steps is still the band's key");
    assert!(matches!(&row.act, NeedsYouAct::Restart { label, build: ipc::BuildSource::Preview, adopt: false, .. } if label == "Refresh preview"), "{:?}", row.act);
    let (section, row) = row_of(&view, "replied:pr");
    assert_eq!((section, row.dismiss), (Section::PullRequests, Dismiss::Plain), "a request to open a pull request is no sentence to call not a question");
}

/// The owner's example: a task he does is no question, and he has nothing to press but Done.
#[test]
fn needs_you_to_do_is_an_agent_item_that_asks_nothing_and_runs_nothing() {
    let restart_the_mac = "Restart the Mac, then check the armada rows in Files and Folders and tell me how many remain.";
    let mut task = item("mac", Source::Agent, None, &[], &[]);
    task.text = restart_the_mac.into();
    let asks = item("asks", Source::Agent, Some((WaitingActKind::Answer, "")), &[], &[]);
    let chooses = item("chooses", Source::Agent, None, &["A", "B"], &[]);
    let runs = item("runs", Source::Agent, Some((WaitingActKind::Run, "cargo xtask verify-foundations")), &[], &[]);
    let approves = item("approves", Source::Agent, Some((WaitingActKind::ApprovePr, "#3")), &[], &[]);
    let message = item("message", Source::Agent, Some((WaitingActKind::Message, JOB)), &[], &[]);
    let view = view_of(&[task, asks, chooses, runs, approves, message]);

    let (section, row) = row_of(&view, "mac");
    assert_eq!(section, Section::ToDo);
    assert_eq!((row.text.as_str(), &row.act, row.dismiss), (restart_the_mac, &NeedsYouAct::Done { steps: Vec::new() }, Dismiss::Reason));
    assert_eq!(row.glyph.said, "To do");
    assert_eq!(row_of(&view, "message").0, Section::ToDo, "a press that only opens the message box is no question either");
    assert_eq!(row_of(&view, "asks").0, Section::Questions, "an act that answers is a question");
    assert_eq!(row_of(&view, "chooses").0, Section::Questions, "so are choices");
    assert_eq!(row_of(&view, "runs").0, Section::Commands, "a run keeps its section");
    assert_eq!(row_of(&view, "approves").0, Section::PullRequests);
    assert_eq!(view.sections.iter().map(|section| section.kind).take(2).collect::<Vec<_>>(), [Section::Questions, Section::ToDo]);
    assert!(view.fold, "a task is a list row");
}

async fn view_in(rig: &Rig, id: &SessionId) -> NeedsYou {
    Arc::clone(&rig.fleet).get_needs_you(None, Some(id.clone())).await.expect("a view")
}

fn dismiss_of(id: &SessionId, item: &str) -> DismissWaiting {
    DismissWaiting { session_id: id.clone(), item_id: item.into(), reason: None, misread: false }
}

#[tokio::test]
async fn needs_you_document_row_says_plain_and_dismissing_it_clears_it_until_a_new_revision() {
    let rig = rig_placing(None);
    let (id, _work, file, path) = one_page(&rig).await;
    let item = format!("document:{path}");

    let view = view_in(&rig, &id).await;
    let (section, row) = row_of(&view, &item);
    assert_eq!((section, row.dismiss), (Section::ToReview, Dismiss::Plain));
    assert_eq!(row.act, NeedsYouAct::OpenDocument { path: path.clone() });
    assert!(view.review_in_band && !view.fold, "a Document alone is in the band, with no list to open");

    Arc::clone(&rig.fleet).dismiss_waiting(dismiss_of(&id, &item)).await.expect("the view's own dismissal is taken");
    assert!(view_in(&rig, &id).await.sections.is_empty(), "it is gone from the view");
    assert!(std::path::Path::new(&path).is_file(), "the file is left");

    written(&rig, &file, "second", "toolu_2").await;
    revision_kept(&rig, &id, &path, "2").await;
    assert_eq!(row_of(&view_in(&rig, &id).await, &item).0, Section::ToReview, "a new revision asks again");
}

#[tokio::test]
async fn needs_you_fleet_call_row_says_plain_and_dismissing_it_clears_it() {
    use ipc::{Does, FleetRule, Notice, ProposedFleet, RuleScope, SaveTeaching, TeachingProposal, TeachingReach};
    let rig = rig_placing(None);
    let id = rig.start().await;
    let rule = FleetRule { notice: Notice::WriteClash.word().to_string(), does: Does::Call, scope: RuleScope::Everywhere, matches: None, when: Vec::new() };
    let save = SaveTeaching {
        proposal: TeachingProposal { fleet: Some(ProposedFleet { rule, untestable: Vec::new() }), ..TeachingProposal::default() },
        reach: TeachingReach::Everywhere,
        repository: None,
        session_id: None,
        cite: None,
        source: None,
    };
    api::Teaching::save_teaching(Arc::clone(&rig.fleet), save).await.expect("saved");
    let words = "Another Job wrote src/a.rs too.";
    assert!(Arc::clone(&rig.fleet).told_by_fleet(id.as_str(), words, Notice::WriteClash).await.expect("answered"));

    let view = view_in(&rig, &id).await;
    let call = view.sections.iter().flat_map(|section| &section.rows).find(|row| row.text == words).expect("the call is a row").clone();
    assert_eq!((call.source, call.dismiss), (Source::FleetCall, Dismiss::Plain));
    assert_eq!(row_of(&view, &call.id).0, Section::Questions);
    assert_eq!(call.act, NeedsYouAct::Message);
    assert!(view.fold);

    Arc::clone(&rig.fleet).dismiss_waiting(dismiss_of(&id, &call.id)).await.expect("the view's own dismissal is taken");
    assert!(view_in(&rig, &id).await.sections.iter().flat_map(|section| &section.rows).all(|row| row.text != words));
}

#[tokio::test]
async fn needs_you_for_a_job_nobody_placed_or_fleet_does_not_hold_is_refused() {
    let rig = rig_placing(None);
    let Err(Refusal::Unacceptable(none)) = Arc::clone(&rig.fleet).get_needs_you(None, None).await else { panic!("a call that places no job is refused") };
    assert_eq!(none.code, "fleet.session_waiting_unnamed");
    let Err(Refusal::Unacceptable(unknown)) = Arc::clone(&rig.fleet).get_needs_you(None, Some(SessionId::carried("nobody"))).await else {
        panic!("a job Fleet does not hold is refused")
    };
    assert_eq!(unknown.code, "fleet.session_waiting_unnamed");
}

fn withdraw(id: &SessionId, path: &str) -> WithdrawDocument {
    WithdrawDocument { path: path.into(), session_id: Some(id.clone()) }
}

#[tokio::test]
async fn withdraw_document_takes_it_out_of_needs_you_leaves_the_file_and_tells_nobody_until_it_is_revised() {
    let rig = rig_placing(None);
    let (id, _work, file, path) = one_page(&rig).await;
    let item = format!("document:{path}");
    let told = sent(&rig.rows(&id).await).len();

    let record = rig.fleet.withdraw_document(None, withdraw(&id, &path)).await.expect("withdrawn");
    assert!(waits(&record).is_empty(), "it left his Needs you list: {:?}", waits(&record));
    assert!(view_in(&rig, &id).await.sections.is_empty());
    assert!(file.is_file(), "the file is not deleted");
    assert_eq!(sent(&rig.rows(&id).await).len(), told, "the Job made the call, so it is told nothing");

    written(&rig, &file, "second", "toolu_2").await;
    revision_kept(&rig, &id, &path, "2").await;
    assert_eq!(waits(&record_of(&rig, &id).await).len(), 1, "a new revision asks again");
    assert_eq!(waits(&record_of(&rig, &id).await)[0].0, item);
}

#[tokio::test]
async fn withdraw_document_reads_a_name_as_a_path_a_file_name_or_a_title() {
    for name in [".armada/documents/lease.html", "lease.html", "Lease expiry", "document:{path}"] {
        let rig = rig_placing(None);
        let (id, _work, _file, path) = one_page(&rig).await;
        let name = name.replace("{path}", &path);
        let record = rig.fleet.withdraw_document(None, withdraw(&id, &name)).await.unwrap_or_else(|why| panic!("`{name}`: {why:?}"));
        assert!(waits(&record).is_empty(), "`{name}` took it back");
    }
}

#[tokio::test]
async fn withdraw_document_refuses_a_page_the_job_does_not_hold_a_name_that_fits_two_and_no_job() {
    let rig = rig_placing(None);
    let (id, work, file, path) = one_page(&rig).await;

    let Err(Refusal::Unacceptable(none)) = rig.fleet.withdraw_document(None, withdraw(&id, "other.html")).await else {
        panic!("a page this job holds no Document at is refused")
    };
    assert_eq!(none.code, "fleet.document_withdraw_not_held");
    assert_eq!(waits(&record_of(&rig, &id).await).len(), 1, "nothing was written");

    // Two pages with one title are two Documents for that name.
    let twin = work.path().join(".armada/documents/twin.html");
    written(&rig, &twin, "again", "toolu_3").await;
    revision_kept(&rig, &id, &twin.to_string_lossy(), "1").await;
    let Err(Refusal::Unacceptable(many)) = rig.fleet.withdraw_document(None, withdraw(&id, "Lease expiry")).await else {
        panic!("a title two Documents carry is refused")
    };
    assert_eq!(many.code, "fleet.document_withdraw_ambiguous");
    assert!(many.message.contains(&path) && many.message.contains("twin.html"), "{}", many.message);
    assert!(file.is_file());

    let Err(Refusal::Unacceptable(unnamed)) =
        rig.fleet.withdraw_document(None, WithdrawDocument { path: path.clone(), session_id: None }).await
    else {
        panic!("a call that places no job is refused")
    };
    assert_eq!(unnamed.code, "fleet.document_withdraw_unnamed");
}

#[tokio::test]
async fn withdraw_document_refuses_a_document_of_another_job() {
    let rig = rig_placing(None);
    let (_first, _work, _file, path) = one_page(&rig).await;
    let second = rig.start().await;
    let Err(Refusal::Unacceptable(refused)) = rig.fleet.withdraw_document(None, withdraw(&second, &path)).await else {
        panic!("another job's page is not this job's")
    };
    assert_eq!(refused.code, "fleet.document_withdraw_not_held");
    assert_eq!(waits(&record_of(&rig, &_first).await).len(), 1, "the first job still asks");
}

const MAC: &str = "Restart the Mac, then check the armada rows in Files and Folders and tell me how many remain.";

async fn stated(rig: &Rig, id: &SessionId) {
    let set = ipc::SetWaitingFor {
        session_id: Some(id.clone()),
        items: vec![ipc::WaitingInput { id: "mac".into(), text: MAC.into(), act: None, options: Vec::new(), steps: Vec::new(), approves: false }],
    };
    rig.fleet.waiting_for(None, set).await.expect("stated");
}

fn done_with(id: &SessionId, item: &str, note: Option<&str>) -> ipc::DoneWaiting {
    ipc::DoneWaiting { session_id: id.clone(), item_id: item.into(), note: note.map(str::to_string) }
}

#[tokio::test]
async fn needs_you_done_closes_the_item_like_a_dismissed_one_and_sends_the_job_one_turn_with_his_note() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    stated(&rig, &id).await;
    let view = view_in(&rig, &id).await;
    let (section, row) = row_of(&view, "mac");
    assert_eq!((section, &row.act), (Section::ToDo, &NeedsYouAct::Done { steps: Vec::new() }));

    let record = Arc::clone(&rig.fleet).done_waiting(done_with(&id, "mac", Some(" 2 remain "))).await.expect("done");
    assert!(record.waiting_for.is_empty(), "it left his list: {:?}", record.waiting_for);
    assert_eq!(record.closed.iter().map(|one| one.item.id.as_str()).collect::<Vec<_>>(), ["mac"], "kept whole, as a dismissed one is");
    assert_eq!(record.closed[0].item.text, MAC);
    assert_eq!(sent(&rig.rows(&id).await), [format!("Done: {MAC}\n2 remain")], "one turn: the line and his note under it");
    assert!(view_in(&rig, &id).await.sections.is_empty());

    // The agent stating it again does not bring it back: the id is dismissed for good.
    stated(&rig, &id).await;
    assert!(view_in(&rig, &id).await.sections.is_empty(), "left off, as a dismissed item is");
}

#[tokio::test]
async fn needs_you_done_with_no_note_sends_the_line_alone() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    stated(&rig, &id).await;
    Arc::clone(&rig.fleet).done_waiting(done_with(&id, "mac", Some("  "))).await.expect("done");
    assert_eq!(sent(&rig.rows(&id).await), [format!("Done: {MAC}")]);
}

#[tokio::test]
async fn needs_you_done_on_a_gone_item_is_settled_already() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    stated(&rig, &id).await;
    Arc::clone(&rig.fleet).done_waiting(done_with(&id, "mac", None)).await.expect("done");
    let Err(Refusal::IllegalMove(gone)) = Arc::clone(&rig.fleet).done_waiting(done_with(&id, "mac", Some("again"))).await else {
        panic!("an item nothing holds is settled already")
    };
    assert_eq!(gone.code, "fleet.session_waiting_unheld");
    assert_eq!(sent(&rig.rows(&id).await).len(), 1, "the second press sent nothing");
}

// Its own rig: two Jobs in one test ran past `eventually`'s five seconds on a loaded CI runner.
#[tokio::test]
async fn needs_you_done_on_what_the_agent_did_not_state_is_refused() {
    let rig = rig_placing(None);
    let (page_job, _work, _file, path) = one_page(&rig).await;
    let Err(Refusal::Unacceptable(refused)) = Arc::clone(&rig.fleet).done_waiting(done_with(&page_job, &format!("document:{path}"), None)).await else {
        panic!("a page is approved, not done")
    };
    assert_eq!(refused.code, "fleet.session_waiting_empty");
    assert_eq!(waits(&record_of(&rig, &page_job).await).len(), 1, "it still waits");
}
