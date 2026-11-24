//! An artifact's parts: the drawing, words, takes, buttons or page an artifact holds. **One ask is one item**: an
//! artifact asks him one thing, worded as a question, and nothing the Job shows joins another artifact on its own.
//! `add_part` puts one on, `answer_part` answers one, and `get_job_artifacts` serves them as
//! [`JobArtifact::parts`](crate::JobArtifact). `docs/concepts/artifact.md`, *Parts*.
//!
//! **Checked here, in the crate where bytes enter**, as a sketch's scene is: [`AddPart::checked`] says every
//! reason a part is refused in one sentence an agent can act on.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::sketch::MOST_TITLE_CHARS;
use crate::{ForLink, Instant, PageAct, RawScene, SessionId, ShowSketch, VoteForm};

/// Most parts one artifact holds.
pub const MOST_PARTS: usize = 12;
/// Most bytes the parts an agent added with `add_part` weigh once written, scenes included. They ride on the
/// Job's record, which is published whole on every change.
pub const MOST_PART_BYTES: usize = 48 * 1024;
/// Most takes one takes part holds.
pub const MOST_TAKES: usize = 6;
/// Most buttons one buttons part holds.
pub const MOST_BUTTONS: usize = 12;
/// Most characters in a part's id.
pub const MOST_PART_ID_CHARS: usize = 40;
/// Most characters in the question an ask puts to him.
pub const MOST_QUESTION_CHARS: usize = 200;

/// The question an ask puts to him, trimmed, or why it is refused. **Every ask is a question in plain words**, which
/// titles its item: one line that ends with `?`, with no markdown. `what` names the field in the refusal.
pub fn question_checked(said: Option<&str>, what: &str) -> Result<String, String> {
    let said = said.map(str::trim).unwrap_or_default();
    if said.is_empty() {
        return Err(format!("{what} is missing. An ask is a question in plain words, such as `Which layout reads better?`, and it titles his item"));
    }
    if said.chars().count() > MOST_QUESTION_CHARS {
        return Err(format!("{what} is longer than {MOST_QUESTION_CHARS} characters. Ask one short question, and put the detail in `for`"));
    }
    if said.contains(['\n', '\r', '`']) {
        return Err(format!("{what} holds a line break or a backtick. Write one line of plain words"));
    }
    if !said.ends_with('?') {
        return Err(format!("{what} `{said}` is not a question. Word what he decides as a question ending with `?`"));
    }
    Ok(said.to_string())
}

/// On an artifact's attachment: the parts the Job added with `add_part`, as JSON text.
pub const ARTIFACT_PARTS: &str = "parts";
/// On the attachment of a sketch, a draft or a page that joined an open artifact: that artifact's id. **The
/// artifact's one Needs you item answers it, so the part raises none of its own.** A part put again keeps it.
pub const ARTIFACT_IN: &str = "in";

/// What a part is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartKind {
    /// A scene `show_sketch` drew.
    Drawing,
    /// Markdown for him to read, as `propose_draft` writes it.
    Words,
    /// Two or more pictures to choose between: a vote.
    Takes,
    /// Buttons that act at once: a message, a start or an answer.
    Buttons,
    /// A page the Job wrote under `.armada/documents/`.
    Page,
    /// A scene file the Job wrote under its `scenes/` folder, drawn by native as a live prototype.
    Prototype,
}

impl PartKind {
    pub fn word(self) -> &'static str {
        match self {
            PartKind::Drawing => "drawing",
            PartKind::Words => "words",
            PartKind::Takes => "takes",
            PartKind::Buttons => "buttons",
            PartKind::Page => "page",
            PartKind::Prototype => "prototype",
        }
    }
}

/// Where a part stands. A part that does not ask stays `open`: nothing is owed on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartState {
    Open,
    Approved,
    SentBack,
    /// A takes part, answered with a pick.
    Picked,
    /// The Job took its ask back, or archived the artifact while it still asked.
    Withdrawn,
}

impl PartState {
    pub fn word(self) -> &'static str {
        match self {
            PartState::Open => "open",
            PartState::Approved => "approved",
            PartState::SentBack => "sent_back",
            PartState::Picked => "picked",
            PartState::Withdrawn => "withdrawn",
        }
    }

    pub fn named(word: &str) -> Option<PartState> {
        match word {
            "open" => Some(PartState::Open),
            "approved" => Some(PartState::Approved),
            "sent_back" => Some(PartState::SentBack),
            "picked" => Some(PartState::Picked),
            "withdrawn" => Some(PartState::Withdrawn),
            _ => None,
        }
    }
}

/// One take of a takes part: a picture to choose.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartTake {
    pub id: String,
    pub title: String,
    /// The picture as a scene's JSON text, a mock being a scene of one `mock` node. Absent for a take of words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
}

/// One button of a buttons part. **Fleet keeps the acts and refuses anything off the list**, as for a page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartAction {
    pub id: String,
    pub act: PageAct,
    pub label: String,
    /// The row or part it sits on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// `start`: what the new Job is asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brief: Option<String>,
    /// `start`: the repository's manifest id, where it is not the Job's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// `answer`: the Job's ask, by its waiting id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<String>,
    /// `answer`: one of the ask's labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option: Option<String>,
}

/// What a part holds, tagged by `kind`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PartContent {
    /// The scene as JSON text.
    Drawing { scene: String },
    Words { text: String },
    Takes {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        question: Option<String>,
        form: VoteForm,
        takes: Vec<PartTake>,
        /// A `score` vote's named parts. Empty for the other forms.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        parts: Vec<String>,
    },
    Buttons { actions: Vec<PartAction> },
    /// The page's path. The part's `revision` is the page's newest.
    Page { path: String },
    /// A scene the Job saved and its fixtures, as read at this revision.
    Prototype(PrototypePart),
}

/// One fixture file a scene names, read beside it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneFixture {
    /// The file name as the scene's `src` gives it, e.g. `jobs.json`.
    pub name: String,
    /// The file's text, raw JSON.
    pub text: String,
}

/// A prototype part: the scene text and what it names, read when the Job showed it or saved it again. **Fleet reads
/// the files and native parses them**, so `trouble` says only what Fleet could not read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrototypePart {
    /// The file name under the Job's `scenes/` folder, e.g. `ready.html`.
    pub scene: String,
    /// The scene's text at this revision.
    pub markup: String,
    /// Each fixture file the scene names, read beside it.
    pub fixtures: Vec<SceneFixture>,
    /// The state the Job last set. Absent means the scene's first. **Not `state`: the part is flattened into
    /// [`ArtifactPart`], whose own `state` is where the part stands, and two keys of one name read neither.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene_state: Option<String>,
    /// What Fleet could not read, in Fleet's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trouble: Option<String>,
}

/// An empty prototype, only so a `CheckedPart` can be built from the fields it names with the rest left at their
/// defaults. No part is ever sent holding it.
impl Default for PartContent {
    fn default() -> Self {
        PartContent::Prototype(PrototypePart::default())
    }
}

impl PartContent {
    pub fn kind(&self) -> PartKind {
        match self {
            PartContent::Drawing { .. } => PartKind::Drawing,
            PartContent::Words { .. } => PartKind::Words,
            PartContent::Takes { .. } => PartKind::Takes,
            PartContent::Buttons { .. } => PartKind::Buttons,
            PartContent::Page { .. } => PartKind::Page,
            PartContent::Prototype(_) => PartKind::Prototype,
        }
    }
}

/// How he answered a part.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartAnswer {
    /// His note on a send back, or his words with an approval or a pick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    /// The take he picked, for a pick vote, and the first place of a rank.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take: Option<String>,
    /// His words on a take, by its id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Instant>,
}

/// One part of an artifact, in the order the Job added them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "PartAsWritten", into = "PartAsWritten")]
pub struct ArtifactPart {
    /// Unique in its Job. A drawing, draft or page that joined is named by the ledger id it is kept under
    /// (`sketch:<title>`, `draft:<title>`, the page's path); a part added with `add_part` by the id the Job gave.
    pub id: String,
    #[serde(flatten)]
    pub content: PartContent,
    /// Whether it is put to him. A part that only informs has nothing to answer.
    pub asks: bool,
    pub state: PartState,
    /// 1 and on: how many times the Job put this part under its id. A redraw is a new revision and asks again.
    pub revision: u32,
    /// An id new on every revision, which `answer_part` names back, so an answer never lands on a part he did not
    /// read: a sketch's `drawn`, a draft's row id, a page's revision, a part's own.
    pub drawn: String,
    /// When the Job first put the part on, which is what orders the parts.
    pub added: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<PartAnswer>,
    /// The question it puts to him in plain words, which titles his item. Present on a part that asks, where the Job
    /// worded one; a part kept before every ask was a question has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}

/// [`ArtifactPart`] as it is written. **A takes part's question is written once**, as the takes' own `question`, which
/// shares the part's key; written twice, the part would not read back.
#[derive(Serialize, Deserialize)]
struct PartAsWritten {
    id: String,
    #[serde(flatten)]
    content: PartContent,
    asks: bool,
    state: PartState,
    revision: u32,
    drawn: String,
    added: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    answer: Option<PartAnswer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    question: Option<String>,
}

impl From<PartAsWritten> for ArtifactPart {
    fn from(written: PartAsWritten) -> Self {
        let PartAsWritten { id, mut content, asks, state, revision, drawn, added, answer, question } = written;
        // The part's own field reads the shared key, so the takes are given it back.
        if let PartContent::Takes { question: own, .. } = &mut content {
            if own.is_none() {
                own.clone_from(&question);
            }
        }
        ArtifactPart { id, content, asks, state, revision, drawn, added, answer, question }
    }
}

impl From<ArtifactPart> for PartAsWritten {
    fn from(part: ArtifactPart) -> Self {
        let ArtifactPart { id, content, asks, state, revision, drawn, added, answer, question } = part;
        let question = match &content {
            PartContent::Takes { question: own, .. } => question.filter(|_| own.is_none()),
            _ => question,
        };
        PartAsWritten { id, content, asks, state, revision, drawn, added, answer, question }
    }
}

impl ArtifactPart {
    /// Put to him and not yet answered.
    pub fn is_open_ask(&self) -> bool {
        self.asks && self.state == PartState::Open
    }
}

/// `add_part`: the Job puts one more part on the artifact of a title, or a new revision of one it put before.
/// `crates/ipc/operations/add_part.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddPart {
    /// The artifact's title. **The artifact of that title takes the part**, and one is made where there is none.
    pub title: String,
    pub part: NewPart,
    /// Whether it is put to him: words and takes default to asking, buttons never ask.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asks: Option<bool>,
    /// Which session adds it. **Read only where the connection places no hosted session**, as `ShowSketch`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// What the artifact is for, as `show_sketch`'s `for`. Kept where the call makes the artifact.
    #[serde(rename = "for", default, skip_serializing_if = "Option::is_none")]
    pub for_line: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<ForLink>,
    /// The feature, or features, the artifact is made for, as `show_sketch`'s `feature`. **Takes stand alone**, so
    /// they name their feature here rather than on a drawing beside them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<crate::FeatureNames>,
    /// The question the part puts to him, in plain words ending with `?`, which titles his item. **Required on a
    /// part that asks**; a takes part may give it as its own `question` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}

/// A part as the Job sends it, tagged by `kind`. **`id` is on each kind, not beside it**, so the tool's shape reads
/// whole. The part's id, which a later call with the same id replaces: a new revision of that part, asking again. Its
/// kind where left out. No `:` or `/`, which name the artifacts that joined.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NewPart {
    /// Refused with a pointer to `show_sketch`, which draws.
    Drawing {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    Words {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        text: String,
    },
    Takes {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        question: Option<String>,
        /// `pick`, `rank` or `score`. `pick` where left out.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        form: Option<VoteForm>,
        takes: Vec<NewTake>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        parts: Vec<String>,
    },
    Buttons {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        actions: Vec<PartAction>,
    },
}

impl NewPart {
    /// The id the Job gave it, if any.
    pub fn id(&self) -> Option<&str> {
        match self {
            NewPart::Drawing { id } | NewPart::Words { id, .. } | NewPart::Takes { id, .. } | NewPart::Buttons { id, .. } => id.as_deref(),
        }
    }
}

/// One take as the Job sends it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewTake {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The picture: a scene, checked as `show_sketch`'s is. A mock is a scene of one `mock` node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<RawScene>,
}

/// An `add_part` that passed every rule.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckedPart {
    pub title: String,
    /// The part's id: the one given, else its kind.
    pub id: String,
    pub content: PartContent,
    pub asks: bool,
    /// The question it asks. Present where it asks.
    pub question: Option<String>,
}

impl AddPart {
    /// The part, or every reason it is refused in one message an agent can act on.
    pub fn checked(&self) -> Result<CheckedPart, String> {
        let mut problems = Vec::new();
        let title = self.title.trim().to_string();
        if title.is_empty() {
            problems.push(String::from("`title` is blank. Name the artifact the part goes on"));
        } else if title.chars().count() > MOST_TITLE_CHARS {
            problems.push(format!("`title` is longer than {MOST_TITLE_CHARS} characters"));
        }
        let mut content = None;
        let kind = match &self.part {
            NewPart::Drawing { .. } => {
                problems.push(String::from("a drawing is not added here: call `show_sketch`, which shows it as its own artifact"));
                None
            }
            NewPart::Words { text, .. } => {
                if text.trim().is_empty() {
                    problems.push(String::from("a `words` part needs its `text`"));
                }
                content = Some(PartContent::Words { text: text.trim().to_string() });
                Some(PartKind::Words)
            }
            NewPart::Takes { question, form, takes, parts, .. } => {
                content = takes_checked(question.as_deref(), *form, takes, parts, &mut problems);
                Some(PartKind::Takes)
            }
            NewPart::Buttons { actions, .. } => {
                content = buttons_checked(actions, &mut problems);
                Some(PartKind::Buttons)
            }
        };
        let id = match self.part.id().map(str::trim) {
            Some("") => {
                problems.push(String::from("the part's `id` is blank. Leave it out, or name the part"));
                String::new()
            }
            Some(id) if id.contains([':', '/']) => {
                problems.push(String::from("the part's `id` holds `:` or `/`, which name the sketches, drafts and pages that joined. Name this part another id"));
                String::new()
            }
            Some(id) if id.chars().count() > MOST_PART_ID_CHARS => {
                problems.push(format!("the part's `id` is longer than {MOST_PART_ID_CHARS} characters"));
                String::new()
            }
            Some(id) => id.to_string(),
            None => kind.map(|kind| kind.word().to_string()).unwrap_or_default(),
        };
        // A button acts when he presses it, so it asks nothing.
        let asks = kind != Some(PartKind::Buttons) && self.asks.unwrap_or(true);
        if self.asks == Some(true) && kind == Some(PartKind::Buttons) {
            problems.push(String::from("`asks` is for words and takes. A button acts when he presses it"));
        }
        if let Err(why) = self.for_checked() {
            problems.push(why);
        }
        // A takes part's own `question` stands for the call's, so either says it.
        let own = match &self.part {
            NewPart::Takes { question, .. } => question.as_deref().map(str::trim).filter(|said| !said.is_empty()),
            _ => None,
        };
        let question = match (asks, self.question.as_deref().or(own)) {
            (false, None) => None,
            (_, said) => match question_checked(said, "`question`") {
                Ok(question) => Some(question),
                Err(why) if asks => {
                    problems.push(why);
                    None
                }
                Err(_) => None,
            },
        };
        if !problems.is_empty() {
            return Err(format!("the part was not added: {}. Fix these and call add_part again", problems.join("; ")));
        }
        let mut content = content.unwrap_or(PartContent::Words { text: String::new() });
        if let (PartContent::Takes { question: own, .. }, Some(asked)) = (&mut content, &question) {
            *own = Some(asked.clone());
        }
        let weight = crate::encode(&content).map(|written| written.len()).unwrap_or(0);
        if weight > MOST_PART_BYTES {
            return Err(format!("the part was not added: it weighs {weight} bytes and an artifact's parts hold at most {MOST_PART_BYTES}. Add less per part"));
        }
        Ok(CheckedPart { title, id, content, asks, question })
    }

    /// The agent's For line off the call, where it gave one, or the reason it is refused.
    pub fn for_checked(&self) -> Result<Option<crate::ArtifactFor>, String> {
        crate::ArtifactFor::checked(self.for_line.as_deref(), &self.links)
    }
}

fn takes_checked(question: Option<&str>, form: Option<VoteForm>, takes: &[NewTake], parts: &[String], problems: &mut Vec<String>) -> Option<PartContent> {
    let form = form.unwrap_or(VoteForm::Pick);
    if takes.len() < 2 {
        problems.push(format!("a `takes` part needs two or more takes, and holds {}", takes.len()));
    }
    if takes.len() > MOST_TAKES {
        problems.push(format!("a `takes` part holds at most {MOST_TAKES} takes, and this has {}", takes.len()));
    }
    let mut seen: Vec<&str> = Vec::new();
    let mut kept = Vec::new();
    for (at, take) in takes.iter().enumerate() {
        let id = take.id.trim();
        if id.is_empty() {
            problems.push(format!("takes[{at}] needs `id`, a non-empty string"));
            continue;
        }
        if seen.contains(&id) {
            problems.push(format!("takes[{at}] repeats the id `{id}`. A take's id is unique in its part"));
            continue;
        }
        seen.push(id);
        let scene = take.scene.as_ref().and_then(|raw| {
            let title = take.title.as_deref().map(str::trim).filter(|title| !title.is_empty()).unwrap_or(id);
            let shown = ShowSketch { feature: None, title: title.to_string(), scene: raw.clone(), session_id: None, against: None, for_line: None, links: Vec::new(), asks: true, question: None };
            match shown.checked_for("add_part") {
                Ok(sketch) => Some(sketch.scene),
                Err(why) => {
                    problems.push(format!("takes[{at}] `{id}` scene: {why}"));
                    None
                }
            }
        });
        let title = take.title.as_deref().map(str::trim).filter(|title| !title.is_empty()).unwrap_or(id).to_string();
        kept.push(PartTake { id: id.to_string(), title, scene });
    }
    let parts: Vec<String> = parts.iter().map(|part| part.trim().to_string()).filter(|part| !part.is_empty()).collect();
    match (form, parts.is_empty()) {
        (VoteForm::Score, true) => problems.push(String::from("a `score` vote names the parts it marks in `parts`, and names none")),
        (VoteForm::Pick | VoteForm::Rank, false) => problems.push(String::from("`parts` is for a `score` vote only")),
        _ => {}
    }
    Some(PartContent::Takes { question: question.map(str::trim).filter(|said| !said.is_empty()).map(str::to_string), form, takes: kept, parts })
}

fn buttons_checked(actions: &[PartAction], problems: &mut Vec<String>) -> Option<PartContent> {
    if actions.is_empty() {
        problems.push(String::from("a `buttons` part needs one or more actions"));
    }
    if actions.len() > MOST_BUTTONS {
        problems.push(format!("a `buttons` part holds at most {MOST_BUTTONS} actions, and this has {}", actions.len()));
    }
    let mut seen: Vec<&str> = Vec::new();
    let mut kept = Vec::new();
    for (at, action) in actions.iter().enumerate() {
        let id = action.id.trim();
        if id.is_empty() {
            problems.push(format!("actions[{at}] needs `id`, a non-empty string"));
            continue;
        }
        if seen.contains(&id) {
            problems.push(format!("actions[{at}] repeats the id `{id}`. An action's id is unique in its part"));
            continue;
        }
        seen.push(id);
        let said = |field: &Option<String>| field.as_deref().map(str::trim).filter(|word| !word.is_empty()).map(str::to_string);
        if action.label.trim().is_empty() {
            problems.push(format!("actions[{at}] `{id}` needs a `label`, the words on the button"));
        }
        match action.act {
            PageAct::Start if said(&action.brief).is_none() => problems.push(format!("actions[{at}] `{id}` is a `start` and needs `brief`, what the new Job is asked")),
            PageAct::Answer if said(&action.ask).is_none() || said(&action.option).is_none() => {
                problems.push(format!("actions[{at}] `{id}` is an `answer` and needs `ask`, the ask's waiting id, and `option`, one of its labels"));
            }
            PageAct::Close => problems.push(format!("actions[{at}] `{id}` is a `close`, which only a page's `<armada-action>` runs, with the issues it lists")),
            _ => {}
        }
        kept.push(PartAction {
            id: id.to_string(),
            act: action.act,
            label: action.label.trim().to_string(),
            item: said(&action.item),
            brief: action.brief.as_deref().map(str::trim).filter(|word| !word.is_empty()).map(str::to_string),
            repository: said(&action.repository),
            ask: said(&action.ask),
            option: said(&action.option),
        });
    }
    Some(PartContent::Buttons { actions: kept })
}

/// How he answered a part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartHow {
    /// Words, or a drawing.
    Approved,
    /// Words, a drawing or takes, sent back with his note asking for changes. On takes it picks none of them.
    SentBack,
    /// A takes part: the pick he sends.
    Picked,
}

/// `answer_part`: he answers one asking part of an artifact on its own. `crates/ipc/operations/answer_part.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerPart {
    pub session_id: SessionId,
    /// The artifact's id, as `JobArtifact::id`.
    pub artifact: String,
    /// The part's id.
    pub part: String,
    /// The part's `drawn` as he read it. **A part put again since is refused.**
    pub drawn: String,
    pub how: PartHow,
    /// `picked`: the take, which a `pick` vote holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take: Option<String>,
    /// The note a send back needs, or words with an approval or a pick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    /// `picked`: his words on any take, by its id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub notes: BTreeMap<String, String>,
    /// Pictures and tags that go with the words.
    #[serde(default, skip_serializing_if = "crate::AnswerExtras::is_empty")]
    pub extras: crate::AnswerExtras,
}
