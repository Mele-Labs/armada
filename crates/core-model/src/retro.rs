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
