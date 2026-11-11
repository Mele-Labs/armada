//! **Every act the Operator would take asks first.** A note ruled filed is one go-ahead in Your turn and
//! nothing is filed until Yes; a rule the person kept for it, in this repository or in every one, does it
//! as it is raised and says so in the Log; one kept for another repository does not. A rule is forgotten
//! in the chat or by a press, and the next one asks again. A red main is asked once, and done by a rule.

use std::sync::Arc;
use std::time::Duration;

use api::Next;
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorPage, OperatorRule, OperatorTurnKind, RuleScope};
use ipc::Event;

use super::autopilot_decisions_repository::{jobs_titled, red_main};
use super::backlog::manifest;
use super::operator_notes::{filing, leave, TEXT};
use super::operator_sources::{rulings, switched, Answers};
use super::session_host::{eventually, rig_placing, Hosted, Rig};
use crate::main_ci::{MainChange, MainChanged};
use crate::operator::{keep_the_raises, Answering, Calling};

const SIDEBAR: &str = "Sidebar width resets after restart";

/// A note no open issue covers, ruled filed: 1 is file.
pub(super) const FILE_IT: &str = "item: 1\nway: 1\nbecause: no open issue covers it";

async fn page(rig: &Rig) -> OperatorPage {
    rig.fleet.operator_page_wire().await.expect("read")
}

pub(super) async fn filed(rig: &Rig, text: &str) -> bool {
    rig.fleet.open_issues(&manifest(rig)).await.expect("read").iter().any(|issue| issue.title.as_written() == text)
}

/// A rule kept in the store as an Always here or an Always leaves it, answering its number.
pub(super) async fn keep_rule(rig: &Rig, manifest_id: Option<&str>, what: &str, pick: &str, said: &str) -> i64 {
    let scope = manifest_id.map_or(RuleScope::Everywhere, |manifest_id| RuleScope::Repository { manifest_id: manifest_id.to_string() });
    let rule = OperatorRule::PickRule { scope, what: what.into(), pick: pick.into() };
    let one = store::KeptOrder { n: 0, form: rule.kept(), said: said.into(), until: None, at: "2026-11-10T09:00:00.000Z".into() };
    rig.fleet.store().lock().await.add_operator_order(&one).expect("kept")
}

/// The Operator seated on `call`, its raises kept, and the notes already there marked as read.
pub(super) async fn seated(rig: &Rig, call: Arc<Answers>) {
    assert!(rig.fleet.operator_seated_with(call));
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    rig.fleet.sources_handed().await;
}

/// A note the person leaves, handed on the next tick.
pub(super) async fn left(rig: &Rig, id: &str, text: &str) {
    leave(rig, id, "open", None, text);
    rig.fleet.sources_handed().await;
}

async fn told_of_the_page(watching: &mut api::Subscription) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) if matches!(delivered.event, Event::OperatorPageChanged(_)) => return,
                Some(_) => continue,
                None => panic!("the stream ended"),
            }
        }
    })
    .await
    .expect("the page's change is told");
}

#[tokio::test]
async fn an_act_with_no_rule_becomes_one_go_ahead_in_your_turn_and_nothing_is_done_until_yes() {
    let rig = filing().await;
    seated(&rig, Answers::with(&[FILE_IT])).await;

    left(&rig, "n-1", TEXT).await;

    eventually(|| async { !page(&rig).await.turn.is_empty() }).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let turn = page(&rig).await.turn;
    assert_eq!(turn.len(), 1, "asked once, never twice: {turn:#?}");
    assert_eq!((turn[0].kind, turn[0].keeps), (OperatorTurnKind::GoAhead, true));
    assert!(turn[0].item.id.starts_with("suggestion:"), "{}", turn[0].item.id);
    assert!(!filed(&rig, TEXT).await, "nothing is filed until he says yes");
    let log = rulings(&rig).await;
    assert_eq!(log.iter().map(|row| row.outcome.as_str()).collect::<Vec<_>>(), ["proposed_file"], "the row says it asked: {log:#?}");

    let press = OperatorAnswerPress { id: turn[0].id.clone(), answer: OperatorAnswer::Yes, then: None, option: None, project: None };
    let answered = rig.fleet.operator_answered(press).await.expect("answered");

    assert!(answered.filed.is_some(), "{answered:#?}");
    assert!(filed(&rig, TEXT).await);
}

#[tokio::test]
async fn an_act_an_always_here_covers_in_this_repository_is_done_as_it_is_raised_and_the_log_says_by_which_rule() {
    let rig = filing().await;
    keep_rule(&rig, Some(manifest(&rig).as_str()), "note:file", "just_file", "File a note as an issue").await;
    seated(&rig, Answers::with(&[FILE_IT])).await;

    left(&rig, "n-1", TEXT).await;

    eventually(|| async { filed(&rig, TEXT).await }).await;
    eventually(|| async { rulings(&rig).await.iter().any(|row| row.by == "operator" && row.outcome == "file" && row.because.starts_with("By a rule, File ") && row.because.contains(" as an issue")) }).await;
    let after = page(&rig).await;
    assert!(after.turn.is_empty(), "it was not asked: {:#?}", after.turn);
    assert!(after.digest.handled.acts.iter().any(|act| act.outcome.starts_with("By a rule: ")), "{:#?}", after.digest.handled);
    assert!(rulings(&rig).await.iter().all(|row| row.by != "human"), "no one pressed anything");
}

#[tokio::test]
async fn a_rule_kept_for_another_repository_does_not_cover_this_one() {
    let rig = filing().await;
    keep_rule(&rig, Some("01SOMEOTHERREPOSITORY"), "note:file", "just_file", "File a note as an issue").await;
    seated(&rig, Answers::with(&[FILE_IT])).await;

    left(&rig, "n-1", TEXT).await;

    eventually(|| async { page(&rig).await.turn.len() == 1 }).await;
    assert!(!filed(&rig, TEXT).await, "it asks, since the rule is for another repository");
}

#[tokio::test]
async fn an_always_covers_every_repository() {
    let rig = filing().await;
    keep_rule(&rig, None, "note:file", "just_file", "File a note as an issue").await;
    seated(&rig, Answers::with(&[FILE_IT])).await;

    left(&rig, "n-1", TEXT).await;

    eventually(|| async { filed(&rig, TEXT).await }).await;
    assert!(page(&rig).await.turn.is_empty());
}

#[tokio::test]
async fn a_rule_forgotten_in_the_chat_makes_the_next_act_ask_again() {
    let rig = filing().await;
    let rule = keep_rule(&rig, Some(manifest(&rig).as_str()), "note:file", "just_file", "File a note as an issue").await;
    let said = format!("cites: none\njob: none\nstart: none\norder: none\nforget: {rule}\nlift: none\nsaid: I have forgotten it.");
    seated(&rig, Answers::with(&[FILE_IT, said.as_str(), FILE_IT])).await;
    left(&rig, "n-1", TEXT).await;
    eventually(|| async { filed(&rig, TEXT).await }).await;
    assert_eq!(page(&rig).await.digest.learned.len(), 1);

    let talked = rig.fleet.operator_talked("forget that rule", None).await.expect("answered");

    assert_eq!(talked.forgot, Some(rule));
    assert!(page(&rig).await.digest.learned.is_empty(), "it is gone from Learned");
    left(&rig, "n-2", SIDEBAR).await;
    eventually(|| async { page(&rig).await.turn.len() == 1 }).await;
    assert!(!filed(&rig, SIDEBAR).await, "it asks again");
}

#[tokio::test]
async fn a_rule_forgotten_by_a_press_tells_the_page_and_makes_the_next_act_ask_again() {
    let rig = filing().await;
    let rule = keep_rule(&rig, Some(manifest(&rig).as_str()), "note:file", "just_file", "File a note as an issue").await;
    seated(&rig, Answers::with(&[FILE_IT])).await;
    let mut watching = rig.fleet.events().subscribe();

    rig.fleet.operator_order_removed(rule).await.expect("removed");

    told_of_the_page(&mut watching).await;
    assert!(page(&rig).await.digest.learned.is_empty());
    left(&rig, "n-1", TEXT).await;
    eventually(|| async { page(&rig).await.turn.len() == 1 }).await;
    assert!(!filed(&rig, TEXT).await, "it asks again");
}

#[tokio::test]
async fn a_red_main_a_rule_covers_starts_the_fix_with_no_ask_and_says_so() {
    let rig = rig_placing(None);
    assert!(rig.fleet.operator_seated_with(Answers::with(&[])));
    switched(&rig, "features.operator", true).await;
    keep_rule(&rig, Some(manifest(&rig).as_str()), "red_main", "start_fix", "Start a fix when main goes red").await;
    let commit = "b".repeat(40);
    red_main(&rig, &commit).await;
    let main = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();

    rig.fleet.red_offered(&[MainChanged { change: MainChange::WentRed, now: main }]).await;

    eventually(|| async { jobs_titled(&rig, "Fix main").await == 1 }).await;
    let after = page(&rig).await;
    assert!(after.turn.is_empty(), "it was not asked: {:#?}", after.turn);
    assert!(after.digest.handled.acts.iter().any(|act| act.outcome == "By a rule: Start a fix when main goes red"), "{:#?}", after.digest.handled);
}

/// A call that turns the Operator off while it is out, then answers that the note is one to file.
struct SwitchedOffMidCall {
    rig: Arc<Hosted>,
}

impl Calling for SwitchedOffMidCall {
    fn ask(&self, _question: String) -> Answering<'_> {
        let fleet = Arc::clone(&self.rig);
        Box::pin(async move {
            let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([("features.operator".to_string(), Some(ipc::SettingValue::Bool(false)))]) };
            api::Settings::save_settings(fleet.as_ref(), save).await.expect("saved");
            Ok(FILE_IT.to_string())
        })
    }
}

#[tokio::test]
async fn an_operator_turned_off_while_its_call_is_out_asks_nothing_and_does_nothing() {
    let rig = filing().await;
    assert!(rig.fleet.operator_seated_with(Arc::new(SwitchedOffMidCall { rig: Arc::clone(&rig.fleet) })));
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    rig.fleet.sources_handed().await;

    left(&rig, "n-1", TEXT).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(page(&rig).await.turn.is_empty(), "nothing is asked of him");
    assert!(!filed(&rig, TEXT).await, "nothing is filed");
    assert!(rulings(&rig).await.iter().all(|row| row.outcome != "proposed_file" && row.outcome != "file"), "{:#?}", rulings(&rig).await);
}
