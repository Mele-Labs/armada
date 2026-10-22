//! The closed sets the wire carries, each over the domain enum that owns it.
//!
//! # Why these wrap a domain value instead of restating one
//!
//! `core-model` carries an `as_wire`/`from_wire` pair beside every enum, and
//! **the wire value is also the registry key**. A second copy of that mapping
//! here is the defect that was removed from `store`, so each type below holds
//! the domain value and spells it through the pair — there is no `match` in
//! this file mapping a variant to a name, and there cannot be one.
//!
//! What crosses the boundary is still the DTO. The domain enums carry no serde
//! derive and cannot be serialised at all; every impl below is written here,
//! and a variant added upstream is a major bump either way — the wire vocabulary
//! is the registry's, and only the registry may widen it.
//!
//! An arriving spelling the domain does not have is **refused, not defaulted**.
//! A peer that sent it does not share this vocabulary, and guessing which
//! variant it meant is how a Job renders as something it is not.

use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize, Serializer};

/// A closed set, over the `core-model` enum that owns its spellings.
macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident, $domain:ty, $what:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name($domain);

        impl $name {
            /// The wire value, which is also the registry key. Named as the
            /// domain pair names it, because it is the domain pair.
            pub fn as_wire(&self) -> &'static str {
                self.0.as_wire()
            }

            /// Read a spelling back. `None` where it is not one the registry
            /// has — the same refusal the deserializer makes, for a caller
            /// holding a string rather than a message.
            pub fn from_wire(spelling: &str) -> Option<$name> {
                <$domain>::from_wire(spelling).map($name)
            }

            /// The domain value. Total, because nothing builds one of these
            /// from a spelling the domain does not have.
            pub fn domain(&self) -> $domain {
                self.0
            }
        }

        impl From<$domain> for $name {
            fn from(value: $domain) -> Self {
                $name(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
                out.serialize_str(self.0.as_wire())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
                let spelling = String::deserialize(input)?;
                <$domain>::from_wire(&spelling)
                    .map($name)
                    .ok_or_else(|| D::Error::custom(format!("`{spelling}` is not {}", $what)))
            }
        }
    };
}

wire_enum! {
    /// What a node on a Studio is. `docs/concepts/studio.md`, *Nodes*.
    StudioNodeKind, core_model::StudioNodeKind, "a Studio node kind"
}
wire_enum! {
    /// Where a node on a Studio stands, for the kinds that have states.
    StudioNodeState, core_model::StudioNodeState, "a Studio node state"
}
wire_enum! {
    /// What an edge on a Studio says.
    StudioEdgeKind, core_model::StudioEdgeKind, "a Studio edge kind"
}
wire_enum! {
    /// A relation that may be proposed. **Never `produced`**, which the Studio
    /// draws itself, so a proposal naming it does not decode.
    StudioRelation, core_model::StudioRelation, "a relation that may be proposed"
}
wire_enum! {
    /// Who put something on a Studio: a person or Helm.
    StudioAuthor, core_model::StudioAuthor, "who put something on a Studio"
}
wire_enum! {
    /// What a source a scout was handed was read as. `#1293`.
    ScoutSourceKind, core_model::ScoutSourceKind, "a source a scout reads"
}
wire_enum! {
    /// Whether a person has accepted an edge.
    StudioEdgeStanding, core_model::StudioEdgeStanding, "a Studio edge standing"
}
wire_enum! {
    /// What one declared mechanical Check did. Six, from
    /// `domain/check-outcomes.toml`. **Two of them advance a step and only one
    /// of those is a pass** — `skipped` is a Check whose declared paths the step
    /// did not touch, which measured nothing and stopped nothing.
    CheckOutcome, core_model::CheckOutcome, "a check outcome"
}
wire_enum! {
    /// Who caused a transition, and a message that did not record which never
    /// will. `judge` and `check` since 22.0: a strict set, so each was a major.
    Actor, core_model::Actor, "an actor"
}
wire_enum! {
    /// Whether a Job's pull request is offered for review or parked as a
    /// draft. Since 23.8.
    PrMode, core_model::PrMode, "a pull request mode"
}
wire_enum! {
    /// Kit's own tier for one MCP server: whether a Drone gets it where the
    /// Manifest has said nothing. `#1275`.
    ReachesDrones, core_model::ReachesDrones, "Kit's reach for a server"
}
wire_enum! {
    /// What one Manifest says about one of Kit's servers. **Absent is not a
    /// third variant**: a Manifest that has said nothing carries no value, and
    /// that is Kit's default answering.
    ManifestReach, core_model::ManifestReach, "a Manifest's word about a server"
}
wire_enum! {
    /// What it takes to advance past one step. Five, from
    /// `domain/workflowdef-fields.toml`.
    ///
    /// **Two name a tier, one names an actor, and two name a policy.** `auto`
    /// and `auto_if_judge_passes` say which of Fleet's tiers is the whole gate;
    /// `human_always` says the tiers do not decide at all and the step holds at
    /// `awaiting_review` for a person. The two `manifest_rule:` forms cross
    /// **unresolved**, which is what the record holds — Bridge draws the
    /// policy's name rather than an answer, because what it resolves to is the
    /// repository's and is settled at the gate, not at freeze.
    AdvanceGate, core_model::AdvanceGate, "an advance gate"
}
wire_enum! {
    /// What kind of work product a step's evidence is, from the
    /// WorkflowDef schema, and **the workflow's word rather than the Drone's**:
    /// the Evidence tool has no parameter for it, so a submission is recorded
    /// under the type its frozen step declared.
    EvidenceType, core_model::EvidenceType, "an evidence type"
}

wire_enum! {
    /// Which door a request that moved a Job came through: `bridge`, `helm`,
    /// `door`, `http` or `phone`. Since 23.12. `docs/concepts/retro.md`.
    Via, core_model::Via, "a door a request came through"
}
wire_enum! {
    /// Whom one item of a retro got in the way of: `drone`, `agent` (a
    /// Session's), `owner` or `fleet`. Since 23.12.
    Whose, core_model::Whose, "whom a retro item got in the way of"
}
wire_enum! {
    /// Where the fix for one retro item lands: `armada`, `kit` or `manifest`.
    /// Since 23.15. `docs/concepts/retro.md`.
    LandsIn, core_model::LandsIn, "a place a retro item's fix lands"
}
wire_enum! {
    /// Where a retro item stands with the person: `open`, `agreed`, `accepted`
    /// or `discarded`. Since 23.26. `docs/concepts/retro.md`.
    LessonState, core_model::LessonState, "a state a retro item is in"
}
wire_enum! {
    /// Where a test map edge was learned: `measured`, `static`, `reading` or
    /// `declared`, sharpest first. `docs/concepts/test-map.md`.
    EdgeSource, core_model::EdgeSource, "a test map edge's source"
}
wire_enum! {
    /// What a test map edge's test names: one `test`, or the test `file` where
    /// the collector could not fence one.
    EdgeGrain, core_model::EdgeGrain, "a test map edge's grain"
}
