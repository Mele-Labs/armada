//! The places a piece of configuration is kept: Armada's, the repository's and yours in Kit.
//! `docs/design/who-sees-each-piece-of-configuration.md`.

/// Which of the three places a piece was read from.
///
/// **Declared in the order they lose, and the order is the rule**: `Ord` is what a reader compares,
/// so a fourth place is put where its variant is written and not in a precedence table beside it.
/// Kit's is last because yours wins over the repository's, wherever the two name the same thing.
///
/// A workflow has `WorkflowSource` and a review tool its own; a lifecycle and a Workspace (a set of
/// repositories) share this one, and a rule will.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PieceSource {
    /// Compiled into the binary.
    Armada,
    /// The repository's own `.armada/`, committed.
    Repository,
    /// Kit's folder, which a person keeps for every repository they work in.
    Kit,
}
