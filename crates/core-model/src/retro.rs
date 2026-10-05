//! Three small vocabularies a Job's retro is written in: which door an act came
//! through, whom a piece of friction got in the way of, and where its fix
//! lands.
//! `docs/concepts/retro.md`.

/// Which door a request that moved a Job came through.
///
/// **Not [`Actor`](crate::Actor).** That says who is accountable for a move;
/// this says how the request reached Fleet, which is the only evidence Fleet
/// has for the first. A move Fleet makes on its own carries none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Via {
    /// Bridge's own request, which names itself with a header.
    Bridge,
    /// A Helm session Fleet placed at the agent door.
    Helm,
    /// The agent door, for a session Fleet did not place as Helm: `armada mcp`
    /// in somebody's own agent session.
    Door,
    /// An HTTP request that named no caller: a script, `curl`, or an agent's
    /// shell. **Not Bridge**, which is all that absence can say.
    Http,
}

impl Via {
    pub const ALL: &'static [Via] = &[Via::Bridge, Via::Helm, Via::Door, Via::Http];

    pub fn as_wire(&self) -> &'static str {
        match self {
            Via::Bridge => "bridge",
            Via::Helm => "helm",
            Via::Door => "door",
            Via::Http => "http",
        }
    }

    pub fn from_wire(value: &str) -> Option<Via> {
        Via::ALL.iter().copied().find(|via| via.as_wire() == value)
    }

    /// Whether a person pressed something in Bridge. Every other door is an
    /// agent acting, on a person's ask or not, and Fleet cannot tell which.
    pub fn is_a_person_in_bridge(&self) -> bool {
        matches!(self, Via::Bridge)
    }
}

/// Whom one item of a retro got in the way of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Whose {
    /// The Drone doing the work.
    Drone,
    /// The person who owns the Job.
    Owner,
    /// Fleet itself: a gate, a Check or a rule that cost the Job and was wrong
    /// to.
    Fleet,
}

impl Whose {
    pub const ALL: &'static [Whose] = &[Whose::Drone, Whose::Owner, Whose::Fleet];

    pub fn as_wire(&self) -> &'static str {
        match self {
            Whose::Drone => "drone",
            Whose::Owner => "owner",
            Whose::Fleet => "fleet",
        }
    }

    pub fn from_wire(value: &str) -> Option<Whose> {
        Whose::ALL
            .iter()
            .copied()
            .find(|whose| whose.as_wire() == value)
    }
}

/// Where the fix for one item of a retro lands. **Apart from [`Whose`]**: whose
/// way a thing got in does not say which place its fix belongs in, and an item
/// names exactly one. A fix that spans two is written as two items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LandsIn {
    /// Armada itself: Fleet or Bridge.
    Armada,
    /// The tool set a person brings: Skills, MCP servers, sub-agents, agent
    /// files, plugins, commands, the allowlist, the models list.
    Kit,
    /// The repository the Job worked on: its `armada.yml`, its tests and its
    /// code.
    Manifest,
}

impl LandsIn {
    pub const ALL: &'static [LandsIn] = &[LandsIn::Armada, LandsIn::Kit, LandsIn::Manifest];

    pub fn as_wire(&self) -> &'static str {
        match self {
            LandsIn::Armada => "armada",
            LandsIn::Kit => "kit",
            LandsIn::Manifest => "manifest",
        }
    }

    pub fn from_wire(value: &str) -> Option<LandsIn> {
        LandsIn::ALL
            .iter()
            .copied()
            .find(|lands| lands.as_wire() == value)
    }
}

/// Where a retro item stands with the person who reads it.
///
/// **Apart from where its fix lands.** `Agreed` and `Accepted` are two
/// answers to the same press: a fix that lands in Armada or in a Manifest
/// becomes a Job a person approves, and a fix that lands in Kit has nothing
/// to dispatch and is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LessonState {
    /// Nobody has answered it. Every item starts here.
    Open,
    /// Agreed, and a Job proposed for it.
    Agreed,
    /// Agreed, and kept as it is: a Kit item.
    Accepted,
    /// Disagreed with. The row stays.
    Discarded,
}

impl LessonState {
    pub const ALL: &'static [LessonState] = &[
        LessonState::Open,
        LessonState::Agreed,
        LessonState::Accepted,
        LessonState::Discarded,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            LessonState::Open => "open",
            LessonState::Agreed => "agreed",
            LessonState::Accepted => "accepted",
            LessonState::Discarded => "discarded",
        }
    }

    pub fn from_wire(value: &str) -> Option<LessonState> {
        LessonState::ALL
            .iter()
            .copied()
            .find(|state| state.as_wire() == value)
    }
}
