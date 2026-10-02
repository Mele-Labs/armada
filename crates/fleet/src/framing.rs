//! Where a frame goes around what it holds: a Cluster around the Notes a
//! person grouped, and a Zone around what a read-in brought back. `#1620`.
//!
//! **A function of positions, so it is tested as one.** Nothing here reads the
//! store or mints; it is handed where the nodes are and answers where the
//! frame's corner goes and where each node sits from it.

use core_model::{StudioGraph, StudioNode, StudioNodeId, StudioPosition};

/// Room between a frame's edge and what it holds, in canvas units.
pub(crate) const INSET: i64 = 24;
/// Room above what a frame holds, for its title.
pub(crate) const HEAD: i64 = 48;

/// A frame drawn around nodes already on a Studio.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Framed {
    /// What holds the frame: what every member already sat in, or the board.
    pub(crate) within: Option<StudioNodeId>,
    /// The frame's corner, measured from what holds it.
    pub(crate) corner: StudioPosition,
    /// Each member, at its spot from the frame's corner.
    pub(crate) holding: Vec<(StudioNodeId, StudioPosition)>,
}

/// The frame around `members`, leaving each where it is on the board.
///
/// **It stays in the frame they share, where they share one**, so a Cluster
/// made of Notes in a read-in's Zone is drawn inside that Zone. Notes from two
/// places, or from nowhere, are framed on the board.
pub(crate) fn around(graph: &StudioGraph, members: &[&StudioNode]) -> Framed {
    let first = members.first().and_then(|node| node.within());
    let shared = members.iter().all(|node| node.within() == first);
    let within = if shared { first.cloned() } else { None };
    let framed_in = within.is_some();
    let at = |node: &StudioNode| match framed_in {
        true => node.position(),
        false => graph
            .on_the_board(node.id())
            .unwrap_or_else(|| node.position()),
    };
    let left = members.iter().map(|node| at(node).x).min().unwrap_or(0);
    let top = members.iter().map(|node| at(node).y).min().unwrap_or(0);
    let corner = StudioPosition {
        x: left - INSET,
        y: top - HEAD,
    };
    Framed {
        within,
        corner,
        holding: members
            .iter()
            .map(|node| (node.id().clone(), at(node).from_corner(corner)))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use core_model::{
        ManifestId, Studio, StudioAuthor, StudioId, StudioNodeContent, Timestamp, Ulid,
    };

    use super::*;

    fn at() -> Timestamp {
        Timestamp::from_rfc3339("2026-10-02T09:00:00.000Z")
    }

    fn node(id: &str, content: StudioNodeContent, x: i64, y: i64) -> StudioNode {
        StudioNode::added(
            StudioNodeId::carried(Ulid::carried(id)),
            content,
            StudioPosition { x, y },
            at(),
            StudioAuthor::Person,
        )
    }

    fn note(id: &str, x: i64, y: i64) -> StudioNode {
        let content = StudioNodeContent::Note {
            said: id.to_string(),
            capture: None,
        };
        node(id, content, x, y)
    }

    fn graph(nodes: Vec<StudioNode>) -> StudioGraph {
        StudioGraph {
            studio: Studio {
                id: StudioId::carried(Ulid::carried("01STUDIO")),
                manifest_id: ManifestId::carried(Ulid::carried("armada")),
                name: None,
                named_by: None,
                created_at: at(),
                touched_at: at(),
            },
            nodes,
            edges: vec![],
        }
    }

    #[test]
    fn a_frame_round_notes_on_the_board_leaves_each_where_it_was() {
        let one = note("01A", 100, 200);
        let two = note("01B", 400, 80);
        let board = graph(vec![one.clone(), two.clone()]);
        let framed = around(&board, &[&one, &two]);
        assert_eq!(framed.within, None);
        assert_eq!(framed.corner, StudioPosition { x: 76, y: 32 });
        for (id, from_corner) in &framed.holding {
            let was = board.on_the_board(id).unwrap();
            assert_eq!(from_corner.past(framed.corner), was);
        }
    }

    #[test]
    fn notes_in_one_zone_are_framed_inside_it() {
        let zone = node("01ZONE", StudioNodeContent::Zone, 1000, 1000);
        let zoned =
            |id, x, y| note(id, 0, 0).placed(Some(zone.id().clone()), StudioPosition { x, y });
        let one = zoned("01A", 40, 60);
        let two = zoned("01B", 40, 260);
        let board = graph(vec![zone.clone(), one.clone(), two.clone()]);
        let framed = around(&board, &[&one, &two]);
        assert_eq!(framed.within.as_ref(), Some(zone.id()));
        assert_eq!(framed.corner, StudioPosition { x: 16, y: 12 });
        assert_eq!(framed.holding[1].1, StudioPosition { x: 24, y: 248 });
    }

    #[test]
    fn notes_from_two_places_are_framed_on_the_board() {
        let zone = node("01ZONE", StudioNodeContent::Zone, 1000, 1000);
        let inside =
            note("01A", 0, 0).placed(Some(zone.id().clone()), StudioPosition { x: 40, y: 60 });
        let outside = note("01B", 0, 0);
        let board = graph(vec![zone, inside.clone(), outside.clone()]);
        let framed = around(&board, &[&inside, &outside]);
        assert_eq!(framed.within, None);
        assert_eq!(
            framed.holding[0].1.past(framed.corner),
            StudioPosition { x: 1040, y: 1060 }
        );
    }
}
