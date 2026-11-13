//! What an arrow's `flow`, `line`, `head` and `path` words come to when it is drawn.

use ipc::{SceneEdge, SceneHead, SceneLine, ScenePath};

/// How the line is laid down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pattern {
    Solid,
    Dashed,
    Dotted,
}

/// The line an arrow is drawn with: its `line` where it says one, else dashed where it is a flow, else solid, as
/// Fleet's docs have it. A `flow: true` over a `line` of `solid` reads as dashed, since a flow is the arrow that moves.
pub(crate) fn pattern(edge: &SceneEdge) -> Pattern {
    match (edge.line, edge.flow) {
        (Some(SceneLine::Dotted), _) => Pattern::Dotted,
        (Some(SceneLine::Dashed), _) | (Some(SceneLine::Solid), Some(true)) => Pattern::Dashed,
        (Some(SceneLine::Solid), _) => Pattern::Solid,
        (None, Some(true)) => Pattern::Dashed,
        (None, _) => Pattern::Solid,
    }
}

/// Whether the dashes crawl along the line while no step plays. Only a dashed flow crawls: an arrow that names its
/// `line` and is not a flow lies still.
pub(crate) fn crawls(edge: &SceneEdge) -> bool {
    pattern(edge) == Pattern::Dashed && edge.flow == Some(true)
}

/// Which ends carry a head: the start, and the end. An arrow's end where it says nothing.
pub(crate) fn heads(edge: &SceneEdge) -> (bool, bool) {
    match edge.head {
        None | Some(SceneHead::End) => (false, true),
        Some(SceneHead::Start) => (true, false),
        Some(SceneHead::Both) => (true, true),
        Some(SceneHead::NoHead) => (false, false),
    }
}

/// How an arrow runs: an elbow where it says nothing.
pub(crate) fn path(edge: &SceneEdge) -> ScenePath {
    edge.path.unwrap_or(ScenePath::Elbow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(line: Option<SceneLine>, head: Option<SceneHead>, path: Option<ScenePath>, flow: Option<bool>) -> SceneEdge {
        SceneEdge { id: "e".into(), from: "a".into(), to: "b".into(), label: None, flow, step: None, color: None, line, head, path }
    }

    fn lined(line: Option<SceneLine>, flow: Option<bool>) -> SceneEdge {
        edge(line, None, None, flow)
    }

    #[test]
    fn each_line_word_is_the_pattern_it_names() {
        assert_eq!(pattern(&lined(Some(SceneLine::Solid), None)), Pattern::Solid);
        assert_eq!(pattern(&lined(Some(SceneLine::Dashed), None)), Pattern::Dashed);
        assert_eq!(pattern(&lined(Some(SceneLine::Dotted), None)), Pattern::Dotted);
    }

    #[test]
    fn a_flow_reads_as_dashed_and_an_arrow_that_says_nothing_is_solid() {
        assert_eq!(pattern(&lined(None, Some(true))), Pattern::Dashed);
        assert_eq!(pattern(&lined(None, None)), Pattern::Solid);
        assert_eq!(pattern(&lined(None, Some(false))), Pattern::Solid);
        assert_eq!(pattern(&lined(Some(SceneLine::Solid), Some(true))), Pattern::Dashed);
        assert_eq!(pattern(&lined(Some(SceneLine::Dotted), Some(true))), Pattern::Dotted);
    }

    #[test]
    fn only_a_dashed_flow_crawls() {
        assert!(!crawls(&lined(None, None)), "an arrow that says nothing is solid");
        assert!(crawls(&lined(None, Some(true))));
        assert!(crawls(&lined(Some(SceneLine::Dashed), Some(true))));
        assert!(!crawls(&lined(Some(SceneLine::Dashed), None)), "a named line lies still");
        assert!(!crawls(&lined(Some(SceneLine::Dotted), Some(true))));
        assert!(!crawls(&lined(Some(SceneLine::Solid), None)));
        assert!(!crawls(&lined(None, Some(false))));
    }

    #[test]
    fn each_head_word_is_the_ends_that_carry_one() {
        let heads_of = |head| heads(&edge(None, head, None, None));
        assert_eq!(heads_of(None), (false, true), "the end where it says nothing");
        assert_eq!(heads_of(Some(SceneHead::End)), (false, true));
        assert_eq!(heads_of(Some(SceneHead::Start)), (true, false));
        assert_eq!(heads_of(Some(SceneHead::Both)), (true, true));
        assert_eq!(heads_of(Some(SceneHead::NoHead)), (false, false));
    }

    #[test]
    fn each_path_word_is_how_the_arrow_runs() {
        let path_of = |word| path(&edge(None, None, word, None));
        assert_eq!(path_of(None), ScenePath::Elbow, "an elbow where it says nothing");
        assert_eq!(path_of(Some(ScenePath::Elbow)), ScenePath::Elbow);
        assert_eq!(path_of(Some(ScenePath::Straight)), ScenePath::Straight);
        assert_eq!(path_of(Some(ScenePath::Curve)), ScenePath::Curve);
    }
}
