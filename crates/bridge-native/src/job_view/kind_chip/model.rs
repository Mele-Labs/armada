//! What the kind chip says and what each pick sends, worked out without a window: the label and hue of a sorting, the two
//! menus, and the `set_message_kind` request a pick makes. `docs/concepts/job-story.md`, *Sorting a message*.

use ipc::{Ask, AskKind, AskState, MessageKind, MessageSorting, SessionId, SetMessageKind, SortedBy};

use crate::dropdown::{Entry, Row};

/// The colour a chip wears: the mock's `.k-bug`, `.k-cr`, `.k-idea` and `.k-ans`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hue {
    Bug,
    Change,
    Idea,
    Neutral,
}

pub(crate) fn ask_label(kind: AskKind) -> &'static str {
    match kind {
        AskKind::Bug => "Bug",
        AskKind::ChangeRequest => "Change request",
        AskKind::Idea => "Idea",
        AskKind::Fix => "Fix",
    }
}

fn ask_hue(kind: AskKind) -> Hue {
    match kind {
        AskKind::Bug => Hue::Bug,
        AskKind::ChangeRequest | AskKind::Fix => Hue::Change,
        AskKind::Idea => Hue::Idea,
    }
}

/// The ask kind a new ask has: Fleet gives one for each, and a sorting without one reads as a change request, as Fleet does.
fn ask_kind_of(sorting: &MessageSorting) -> AskKind {
    sorting.ask_kind.unwrap_or(AskKind::ChangeRequest)
}

/// What the chip reads: a new ask says its ask's kind, the rest say what they are.
pub(crate) fn label(sorting: &MessageSorting) -> &'static str {
    match sorting.kind {
        MessageKind::NewAsk => ask_label(ask_kind_of(sorting)),
        MessageKind::Revision => "Revision",
        MessageKind::Answer => "Answer",
        MessageKind::Talk => "Talk",
    }
}

pub(crate) fn hue(sorting: &MessageSorting) -> Hue {
    match sorting.kind {
        MessageKind::NewAsk => ask_hue(ask_kind_of(sorting)),
        MessageKind::Revision | MessageKind::Answer | MessageKind::Talk => Hue::Neutral,
    }
}

/// The chip's tooltip: who sorted the message.
pub(crate) fn sorted_by(sorting: &MessageSorting) -> &'static str {
    match sorting.sorted_by {
        SortedBy::Fleet => "Sorted by Fleet",
        SortedBy::Owner => "Sorted by you",
    }
}

/// One line of the first menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    Ask(AskKind),
    Revision,
    Answer,
    Talk,
}

/// The first menu's lines, in the order the mock lists them.
pub(crate) const CHOICES: [Choice; 7] = [
    Choice::Ask(AskKind::Bug),
    Choice::Ask(AskKind::ChangeRequest),
    Choice::Ask(AskKind::Idea),
    Choice::Ask(AskKind::Fix),
    Choice::Revision,
    Choice::Answer,
    Choice::Talk,
];

impl Choice {
    /// A menu entry's id, a static string.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Choice::Ask(AskKind::Bug) => "kind:bug",
            Choice::Ask(AskKind::ChangeRequest) => "kind:change_request",
            Choice::Ask(AskKind::Idea) => "kind:idea",
            Choice::Ask(AskKind::Fix) => "kind:fix",
            Choice::Revision => "kind:revision",
            Choice::Answer => "kind:answer",
            Choice::Talk => "kind:talk",
        }
    }

    pub(crate) fn of(id: &str) -> Option<Choice> {
        CHOICES.into_iter().find(|choice| choice.id() == id)
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Choice::Ask(kind) => ask_label(kind),
            Choice::Revision => "Revision",
            Choice::Answer => "Answer",
            Choice::Talk => "Talk",
        }
    }

    /// Whether the message is sorted as this now.
    fn is(self, sorting: &MessageSorting) -> bool {
        match self {
            Choice::Ask(kind) => sorting.kind == MessageKind::NewAsk && ask_kind_of(sorting) == kind,
            Choice::Revision => sorting.kind == MessageKind::Revision,
            Choice::Answer => sorting.kind == MessageKind::Answer,
            Choice::Talk => sorting.kind == MessageKind::Talk,
        }
    }

    /// Whether the pick needs an ask of the Job to join.
    pub(crate) fn joins(self) -> bool {
        matches!(self, Choice::Revision | Choice::Answer)
    }
}

/// Why Revision and Answer are off while the Job holds no open ask.
pub(crate) const NO_ASK: &str = "No open ask in this Job";

/// The asks a revision or an answer may join: the ones still open.
pub(crate) fn open_asks(asks: &[Ask]) -> Vec<&Ask> {
    asks.iter().filter(|ask| matches!(ask.state, AskState::Backlog | AskState::Building)).collect()
}

/// The first menu: each kind, the one the message has marked, Revision and Answer off where there is no ask to join.
pub(crate) fn kind_entries(sorting: &MessageSorting, asks: &[Ask]) -> Vec<Entry> {
    let none = open_asks(asks).is_empty();
    CHOICES
        .into_iter()
        .map(|choice| {
            let row = Row::new(choice.id(), choice.label()).ticked(choice.is(sorting));
            Entry::Row(if choice.joins() && none { row.off(NO_ASK) } else { row })
        })
        .collect()
}

/// The second menu's entry ids: one for each ask it lists, which are static strings.
pub(crate) const ASK_IDS: [&str; 24] = [
    "ask:0", "ask:1", "ask:2", "ask:3", "ask:4", "ask:5", "ask:6", "ask:7", "ask:8", "ask:9", "ask:10", "ask:11", "ask:12", "ask:13", "ask:14", "ask:15", "ask:16", "ask:17", "ask:18", "ask:19", "ask:20", "ask:21", "ask:22", "ask:23",
];

/// The open asks the second menu lists, each a title and its kind, the ask the message joins marked, oldest first as Fleet
/// gives them. Past the table's twenty-four, an ask is not listed.
pub(crate) fn ask_entries(sorting: &MessageSorting, asks: &[Ask]) -> Vec<Entry> {
    open_asks(asks)
        .into_iter()
        .zip(ASK_IDS)
        .map(|(ask, id)| {
            let joined = matches!(sorting.kind, MessageKind::Revision | MessageKind::Answer) && sorting.ask_id.as_deref() == Some(ask.ask_id.as_str());
            Entry::Row(Row::new(id, ask.title.clone()).detail(ask_label(ask.kind)).ticked(joined))
        })
        .collect()
}

/// The ask an entry of the second menu names.
pub(crate) fn ask_picked<'a>(asks: &'a [Ask], id: &str) -> Option<&'a Ask> {
    let at = ASK_IDS.iter().position(|one| *one == id)?;
    open_asks(asks).get(at).copied()
}

/// What a pick sends. Revision and Answer carry the ask they join and are not sent without one.
pub(crate) fn request(job: &str, row_id: &str, choice: Choice, ask: Option<&str>) -> Option<SetMessageKind> {
    let (kind, ask_kind, ask_id) = match choice {
        Choice::Ask(kind) => (MessageKind::NewAsk, Some(kind), None),
        Choice::Talk => (MessageKind::Talk, None, None),
        Choice::Revision => (MessageKind::Revision, None, Some(ask?.to_string())),
        Choice::Answer => (MessageKind::Answer, None, Some(ask?.to_string())),
    };
    Some(SetMessageKind { session_id: SessionId::carried(job), row_id: row_id.to_string(), kind, ask_kind, ask_id })
}
