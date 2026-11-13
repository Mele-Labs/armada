//! The Operator's page: Your turn, the queue the person works through, beside the digest Since you
//! last looked. `crates/ipc/operations/operator_page.toml`.
//!
//! **Fleet keeps it all; Bridge only draws it.** An item is a `WaitingItem` as it was raised, with
//! the Operator's read and pick beside it, so its kind, its thing and its acts are the item's own
//! and no second vocabulary stands beside them.

use serde::{Deserialize, Serialize};

use crate::operator_talk::OperatorHoldUntil;
use crate::{Instant, MainCiState, OperatorRead, SessionId, WaitingAct, WaitingItem};

/// Whose act an item is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorTurnKind {
    /// His to decide: a plan, a walk, a review, a question.
    #[default]
    Decision,
    /// What the Operator would do itself, waiting on his word.
    GoAhead,
}

/// How soon an item wants him, which Fleet decides so no screen has to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorUrgency {
    /// A red main.
    Urgent,
    /// His decision.
    #[default]
    Decision,
    /// A go-ahead.
    GoAhead,
}

/// His press on an item. Later and Ask are not here: Later is `operator_later`, and Ask is Bridge
/// opening the chat with `talk_to_operator` carrying the item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorAnswer {
    /// Do the pick.
    Yes,
    /// Do the pick, and keep it as a rule for this item's project.
    AlwaysHere,
    /// Do the pick, and keep it as a rule everywhere.
    Always,
    /// Drop the item.
    No,
}

/// What follows filing an issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorThen {
    JustFile,
    /// File it, then start a Job on it.
    FileAndStart,
    /// File it, then hand it to a live Job already working on the same files.
    FileAndHand,
}

/// A live Job an issue would be handed to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorHandTo {
    pub session_id: SessionId,
    pub job: String,
}

/// One Then offered on a go-ahead to file an issue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorThenOffer {
    pub then: OperatorThen,
    /// On `file_and_hand` only: the Job. **That choice is offered only where one is live.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_to: Option<OperatorHandTo>,
    /// The Operator's read marks this one as its pick.
    #[serde(default, skip_serializing_if = "is_false")]
    pub pick: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// What an answer follows: one of his own answers, or a setting of his. Since 13 Nov 2026.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorFollowsKind {
    /// His answer to the same question in the same project.
    Answer,
    /// A preference or a repository's value he saved.
    Setting,
}

/// What the Operator's pick follows, as Fleet checked it. Since 13 Nov 2026.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorFollows {
    pub kind: OperatorFollowsKind,
    /// The Job where he gave the answer. `answer` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// That Job's title. `answer` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    /// What he was asked. `answer` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// What he said: his choice and his words, or `key = value` for a setting.
    pub said: String,
    /// When he said it. `answer` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Instant>,
}

/// One item of Your turn.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTurnItem {
    /// `turn:<n>`, the id every act on the page names.
    pub id: String,
    pub kind: OperatorTurnKind,
    /// How soon it wants him: `urgent` on a red main, else `decision` or `go_ahead` as its kind.
    #[serde(default)]
    pub urgency: OperatorUrgency,
    /// The Job whose list holds it. Absent on a go-ahead no Job holds and on a red main.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// That Job's title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    /// The repository's id, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<String>,
    /// The repository's name, where Fleet serves it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// The item as it was raised. Its `source` is its kind, `text` the thing, `act` and `options` what it asks.
    pub item: WaitingItem,
    /// What the Operator made of it and what it would pick. Absent where no call read it: the Operator
    /// was paused for the week's plan, or the call failed. An empty `pick` is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<OperatorRead>,
    /// When it joined the queue.
    pub raised_at: Instant,
    /// Last in the queue until then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub later_until: Option<Instant>,
    /// On a go-ahead to file an issue: the Thens offered, in the order to draw them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<OperatorThenOffer>,
    /// Whether Always here and Always are offered: only where Fleet can do the pick by itself, which
    /// a go-ahead and a red main are.
    #[serde(default, skip_serializing_if = "is_false")]
    pub keeps: bool,
    /// What else raised the same ask, folded into this one, each as its tooltip names it, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also_raised_by: Vec<String>,
    /// The Job's id, where it has two or more asks waiting: `turn` lists them together, at the place of
    /// the first of them, in the order the Job raised them, and Bridge draws them as one entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// What the Operator's pick follows, on a go-ahead question that no rule of his covers yet: his own earlier answer in
    /// the project, or a setting. Absent where it follows nothing. Since 13 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follows: Option<OperatorFollows>,
}

/// What the Operator is reading now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorOnIt {
    /// What it is reading, in a few words.
    pub reading: String,
    pub since: Instant,
}

/// A pull request that merged since he last looked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorLanded {
    pub session_id: SessionId,
    /// The Job that held it.
    pub job: String,
    pub number: String,
    pub title: String,
    /// When it merged. Since 10 Nov 2026.
    pub at: Instant,
}

/// One thing the Operator did on its own since he last looked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorHandledAct {
    pub at: Instant,
    /// What it was about: a Job's title, an item's first line.
    pub title: String,
    /// Written `Kind: what`, as the Log's rows are.
    pub outcome: String,
}

/// Where the week's Log is: the journal Fleet keeps for the Operator, a markdown file for each ISO week.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorLogFile {
    /// `2026-W46`.
    pub week: String,
    /// The file's absolute path on this machine.
    pub path: String,
}

/// What the Operator handled itself since he last looked.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorHandled {
    /// Newest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acts: Vec<OperatorHandledAct>,
    /// The week's Log, where anything is written in it yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<OperatorLogFile>,
}

/// A live Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorRunning {
    pub session_id: SessionId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

/// A Job waiting to start.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorComingUp {
    pub session_id: SessionId,
    pub title: String,
    /// What it waits on, in words.
    pub needs: String,
}

/// One rule the Operator keeps from an Always or an Always here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorLearned {
    pub id: i64,
    /// Where it holds: a repository, a project, or everywhere, which is an Always. Since 10 Nov 2026.
    pub scope: RuleScope,
    /// What a person reads for the scope: the repository's name, or the project's. Absent on everywhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// The rule in Fleet's words, such as `Start a fix when main is red`.
    pub said: String,
    pub at: Instant,
}

/// One hold that stands over paths: only the Job it names writes under them, until it pushes or ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorHold {
    /// The order's number, which `remove_operator_order` and the chat's `lift` name.
    pub id: i64,
    /// The repository it holds in.
    pub manifest_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// The hold in Fleet's words, such as `Hold src/a until Fix the clock pushes`.
    pub said: String,
    /// The paths held, as the order names them. Empty on an answer from before they were sent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    /// The only Job that writes under them. Absent on an answer from before it was sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<SessionId>,
    /// What lifts the hold. Absent on an answer from before it was sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<OperatorHoldUntil>,
    pub at: Instant,
}

/// A question the Operator answered by following something he had already said. Since 13 Nov 2026.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorRuled {
    /// `turn:<n>`, the row of Your turn that kept it.
    pub id: String,
    pub session_id: SessionId,
    /// The Job's title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    /// The question it answered.
    pub question: String,
    /// The option it picked.
    pub pick: String,
    pub follows: OperatorFollows,
    pub at: Instant,
}

/// Since he last looked, and what stands.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorDigest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub landed: Vec<OperatorLanded>,
    #[serde(default)]
    pub handled: OperatorHandled,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub running: Vec<OperatorRunning>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coming_up: Vec<OperatorComingUp>,
    /// The rules, across projects. Rules only: a hold is not one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub learned: Vec<OperatorLearned>,
    /// The holds that stand, beside Learned.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holds: Vec<OperatorHold>,
    /// Questions it answered by following what he had said, over the last seven days, newest first. Since 13 Nov 2026.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ruled: Vec<OperatorRuled>,
}

/// One repository's main, as the head draws it: a pill, red when main is red. Since 10 Nov 2026.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorMain {
    pub manifest_id: String,
    /// The repository's name, as `OperatorTurnItem` gives it.
    pub repository: String,
    /// How main's CI stands, as the merge line's `main` has it. **Red includes a red a newer commit's
    /// run is still on.** Absent where Fleet has not read main yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<MainCiState>,
    /// The commit the state was decided from, whole: the red's own on a red. Absent with `state`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// When main first read red, kept while it stays red. `red` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<Instant>,
}

/// The slots in use across every repository's pool: `used` of `total`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorSlots {
    /// Held by a Job, a process or work stranded in it, or under a take or a release.
    pub used: u32,
    /// Every slot of every pool, free, unmade or closed ones too.
    pub total: u32,
}

/// `operator_page`: the whole page.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorPage {
    /// Whether the Operator is on. Off, Your turn is empty: nothing is queued while it is off.
    pub on: bool,
    /// The page's one line, rewritten each time he comes back. Absent before the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<String>,
    /// When he last looked: what the digest counts from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub looked_at: Option<Instant>,
    /// Present while the Operator is reading something to review.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_it: Option<OperatorOnIt>,
    /// In the order he left them, what he put Later last.
    #[serde(default)]
    pub turn: Vec<OperatorTurnItem>,
    #[serde(default)]
    pub digest: OperatorDigest,
    /// Each repository Fleet serves, first first. Since 10 Nov 2026.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mains: Vec<OperatorMain>,
    /// Absent where no pool is read. Since 10 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slots: Option<OperatorSlots>,
}

/// `operator_answer`: one press on an item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorAnswerPress {
    /// The item's `id`.
    pub id: String,
    pub answer: OperatorAnswer,
    /// On an Always here: the project the page is filtered to, which the rule then holds in. Absent holds
    /// in the item's repository. Since 10 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// On a go-ahead to file an issue: which Then. Absent takes the Operator's pick, else `just_file`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub then: Option<OperatorThen>,
    /// The label of the option he chose, one the item lists in `options`. Absent takes the Operator's
    /// pick, so a press that names none is as it was. Refused as not offered where the item lists none such.
    /// Since 10 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option: Option<String>,
}

/// What `operator_answer` did.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorAnswered {
    /// What Bridge opens: the act of an item Fleet cannot do itself, such as a walk or a pull request.
    /// The item stays in Your turn until its Job's list drops it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<WaitingAct>,
    /// The Job a fix or a filed issue started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<SessionId>,
    /// The issue filed, as its tracker names it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filed: Option<String>,
    /// The live Job the issue was handed to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handed_to: Option<SessionId>,
    pub page: OperatorPage,
}

/// `operator_turn_order`: the items as he dragged them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTurnOrder {
    /// Item ids, first to last. Items it does not name follow, as they were.
    pub ids: Vec<String>,
}

/// `operator_later`: one item goes last until `until`, or for the rest of the local day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorLater {
    pub id: String,
    /// When it comes back, which Bridge works out from the choice he pressed. Absent, the local day's end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Instant>,
}

/// `operator.page_changed`: the page is not what it was, so read it again. Nothing else rides.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorPageChanged {}

/// `operator_in_front`: Bridge's window came to the front.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorInFront {
    /// True where he came back after time away and the page's top line is being rewritten.
    pub rewriting: bool,
}

/// Where a rule holds. **A rule for the repository beats one for a project, which beats one for everywhere.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleScope {
    /// Any repository, and an item in none. What an Always keeps.
    Everywhere,
    /// The items of one repository.
    Repository { manifest_id: String },
    /// The items of any repository the project holds, a folder member counting as its repository.
    Project { project: String },
}

/// A rule the Operator keeps from an Always or an Always here, in the store's orders beside the
/// holds. It is none of `OperatorOrderForm`'s forms, so a list of orders never serves it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperatorRule {
    /// Where `what` arrives within `scope`, do `pick`, with no press.
    PickRule { scope: RuleScope, what: String, pick: String },
}

/// A rule as the store may hold it: before a scope was one of three, a rule named the repository it
/// held in, or none for everywhere.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RuleStored {
    PickRule {
        #[serde(default)]
        scope: Option<RuleScope>,
        #[serde(default)]
        manifest_id: Option<String>,
        what: String,
        pick: String,
    },
}

impl OperatorRule {
    /// The rule as the store keeps it.
    pub fn kept(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// What the store kept. `None` where it is no rule, such as a standing order.
    pub fn read(text: &str) -> Option<OperatorRule> {
        let RuleStored::PickRule { scope, manifest_id, what, pick } = serde_json::from_str(text).ok()?;
        let scope = scope.unwrap_or_else(|| manifest_id.map_or(RuleScope::Everywhere, |manifest_id| RuleScope::Repository { manifest_id }));
        Some(OperatorRule::PickRule { scope, what, pick })
    }
}

/// A raised item as the store keeps its body.
pub fn item_kept(item: &WaitingItem) -> String {
    serde_json::to_string(item).unwrap_or_default()
}

/// What the store kept of an item. `None` where it does not read.
pub fn item_read(text: &str) -> Option<WaitingItem> {
    serde_json::from_str(text).ok()
}
