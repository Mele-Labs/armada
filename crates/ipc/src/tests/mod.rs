//! What the wire must keep true: the DTOs a Bridge reads.
//!
//! The DTOs round-trip, a spelling the domain does not have is refused rather
//! than defaulted, and **an unknown field does not break a parse** — the last
//! is the one a review cannot check by reading, is the whole basis of the
//! minor-skew row, and would break silently the day somebody added
//! `deny_unknown_fields` for tidiness. Each module below holds one subject's
//! share of that.

/// An empty optional field leaves no key rather than a `null`.
mod absent;
mod accounts;
mod notice_mutes;
mod annotations;
mod building;
/// The allowlist rows and the Kit changes.
mod commanding;
/// One Job, whole: the step rows, the gates' answers and the fields a Board
/// row leaves behind.
/// One entry added to a document this crate did not write.
mod document;
/// The agent's door: a tool call becomes a request on the HTTP surface, and an
/// answer too large to carry says so.
mod door;
/// What the stream carries, under the names `operations/` declares.
mod events;
mod helm;
mod helm_box;
/// A Job's Now: a Drone's activity, issues, waiting, answer modes, Check acts.
mod thought;
/// A Job's timeline, in the three shapes a row can be.
mod limits;
mod review_runs;
mod layout;
mod key_bindings;
mod kit_pieces;
mod time_format;
mod mods;
mod preferences;
mod settings;
/// The one DTO on this seam a peer *writes*, and what it refuses.
mod protocol_id;
/// A given-back worktree, and the absent halves Bridge reads as absent.
/// A person's run: the two kinds it streams under, and the record it leaves.
mod rehearsal;
/// A repository to clone, as Bridge spells it.
mod repositories;
mod requester;
/// Requests' file and wire: a secret's value is in no answer and no file.
mod requests;
mod servers;
/// The laps a run of a Check recorded, as the file that carries them from the forge.
mod test_map;
mod session_activity;
mod session_build;
mod loadout;
/// A pinned note: flat alone, and in order in a batch.
mod session_notes;
mod session_plan;
mod session_starts;
mod session_summaries;
/// A Studio's graph, and the relation no proposal may name.
mod studio;
mod studio_sketch;
mod studio_zones;
/// The Board row, and the redaction it exists for.
mod timeline;
mod bay_history;
/// Walk notes: absent where empty, so a 23.17 peer sees nothing new.
mod window_asks;
mod sketch;
