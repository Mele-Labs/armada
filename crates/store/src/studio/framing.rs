//! Which frame a node sits in: a Zone, or a Note's Cluster. `#1620`, decided
//! with the owner on 2 Oct 2026.
//!
//! **A node's `x` and `y` are measured from its frame's corner**, so a frame
//! moved is one write and everything in it comes with it. What may hold what
//! is `core_model`'s [`StudioNodeKind::holds`], asked here on every write that
//! names a frame.
//!
//! **A Note's Cluster is its record**: the Notes in one are the ones with a
//! `produced` edge into it. So a Note goes into a Cluster only where that edge
//! is, and never leaves the one it is in by being moved.

use core_model::{
    StudioEdgeKind, StudioId, StudioNode, StudioNodeId, StudioNodeKind, StudioPosition, Timestamp,
};
use rusqlite::{OptionalExtension, Transaction};

use super::{database, touched, StudioError};
use crate::open::Store;

impl Store {
    /// Move a node to where a person left it, in the frame named, or on the
    /// board where `within` is `None`.
    pub fn move_studio_node(
        &mut self,
        studio_id: &StudioId,
        node_id: &StudioNodeId,
        within: Option<&StudioNodeId>,
        to: StudioPosition,
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let tx = self.writing()?;
        touched(&tx, studio_id, at)?;
        let (kind, was) = placement(&tx, studio_id, node_id)?.ok_or_else(|| no_node(node_id))?;
        if let Some(cluster) = was.as_ref().filter(|was| within != Some(*was)) {
            if kind_of(&tx, studio_id, cluster)? == Some(StudioNodeKind::Cluster) {
                return Err(StudioError::StaysInItsCluster {
                    node_id: node_id.as_str().to_string(),
                });
            }
        }
        if let Some(frame) = within {
            fits(&tx, studio_id, kind, frame)?;
            if kind_of(&tx, studio_id, frame)? == Some(StudioNodeKind::Cluster)
                && !produced(&tx, studio_id, node_id, frame)?
            {
                return Err(StudioError::NotItsCluster {
                    node_id: node_id.as_str().to_string(),
                    cluster: frame.as_str().to_string(),
                });
            }
        }
        tx.execute(
            "UPDATE studio_nodes SET within = ?3, x = ?4, y = ?5 WHERE studio_id = ?1 AND id = ?2",
            (
                studio_id.as_str(),
                node_id.as_str(),
                within.map(StudioNodeId::as_str),
                to.x,
                to.y,
            ),
        )
        .map_err(database("moving a node"))?;
        tx.commit().map_err(database("moving a node"))
    }
}

/// Refuse a node of `kind` in `frame` where the frame is not on this Studio or
/// is not a kind that holds it. **Every write naming a frame asks this.**
pub(super) fn fits(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    kind: StudioNodeKind,
    frame: &StudioNodeId,
) -> Result<(), StudioError> {
    let holder = kind_of(tx, studio_id, frame)?.ok_or_else(|| no_node(frame))?;
    match holder.holds(kind) {
        true => Ok(()),
        false => Err(StudioError::CannotHold {
            frame: frame.as_str().to_string(),
            frame_kind: holder,
            kind,
        }),
    }
}

/// Refuse a node being added in a Cluster, as the move would: a node just
/// made is none of the Notes a Cluster was made of. A read-in's Notes are
/// written with their Cluster, and do not come through here.
pub(super) fn not_a_cluster(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    node: &StudioNode,
    frame: &StudioNodeId,
) -> Result<(), StudioError> {
    match kind_of(tx, studio_id, frame)? {
        Some(StudioNodeKind::Cluster) => Err(StudioError::NotItsCluster {
            node_id: node.id().as_str().to_string(),
            cluster: frame.as_str().to_string(),
        }),
        _ => Ok(()),
    }
}

/// Lift everything the nodes being removed hold onto whatever held each of
/// them, keeping where it sits on the board. **Innermost first**, so a Note in
/// a Cluster in a Zone that are all going ends up on the board, not in a
/// frame that no longer exists.
pub(super) fn lift_out_of(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    going: &[StudioNodeId],
) -> Result<(), StudioError> {
    let mut frames: Vec<(usize, &StudioNodeId)> = Vec::new();
    for frame in going {
        frames.push((depth(tx, studio_id, frame)?, frame));
    }
    frames.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, frame) in frames {
        let Some((corner, holder)) = corner_of(tx, studio_id, frame)? else {
            continue;
        };
        tx.execute(
            "UPDATE studio_nodes SET within = ?3, x = x + ?4, y = y + ?5 \
             WHERE studio_id = ?1 AND within = ?2",
            (
                studio_id.as_str(),
                frame.as_str(),
                holder.as_ref().map(StudioNodeId::as_str),
                corner.x,
                corner.y,
            ),
        )
        .map_err(database("lifting what a frame held"))?;
    }
    Ok(())
}

/// Several nodes put into a frame just made, each at its own spot in it.
pub(super) fn held_by(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    frame: &StudioNode,
    holding: &[(StudioNodeId, StudioPosition)],
) -> Result<(), StudioError> {
    for (node_id, to) in holding {
        let (kind, _) = placement(tx, studio_id, node_id)?.ok_or_else(|| no_node(node_id))?;
        fits(tx, studio_id, kind, frame.id())?;
        tx.execute(
            "UPDATE studio_nodes SET within = ?3, x = ?4, y = ?5 WHERE studio_id = ?1 AND id = ?2",
            (
                studio_id.as_str(),
                node_id.as_str(),
                frame.id().as_str(),
                to.x,
                to.y,
            ),
        )
        .map_err(database("putting a node in a frame"))?;
    }
    Ok(())
}

/// A node's kind and the frame it sits in, or `None` where it is not here.
fn placement(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    node_id: &StudioNodeId,
) -> Result<Option<(StudioNodeKind, Option<StudioNodeId>)>, StudioError> {
    let row: Option<(String, Option<String>)> = tx
        .query_row(
            "SELECT kind, within FROM studio_nodes WHERE studio_id = ?1 AND id = ?2",
            (studio_id.as_str(), node_id.as_str()),
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(database("reading where a node sits"))?;
    Ok(row.and_then(|(kind, within)| {
        let kind = StudioNodeKind::from_wire(&kind)?;
        Some((kind, within.map(carried)))
    }))
}

fn kind_of(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    node_id: &StudioNodeId,
) -> Result<Option<StudioNodeKind>, StudioError> {
    Ok(placement(tx, studio_id, node_id)?.map(|(kind, _)| kind))
}

/// Where a frame's corner is, measured from what holds it, and what that is.
fn corner_of(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    node_id: &StudioNodeId,
) -> Result<Option<(StudioPosition, Option<StudioNodeId>)>, StudioError> {
    let row: Option<(i64, i64, Option<String>)> = tx
        .query_row(
            "SELECT x, y, within FROM studio_nodes WHERE studio_id = ?1 AND id = ?2",
            (studio_id.as_str(), node_id.as_str()),
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(database("reading a frame's corner"))?;
    Ok(row.map(|(x, y, within)| (StudioPosition { x, y }, within.map(carried))))
}

/// How many frames a node is inside.
fn depth(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    node_id: &StudioNodeId,
) -> Result<usize, StudioError> {
    let mut depth = 0;
    let mut at = placement(tx, studio_id, node_id)?.and_then(|(_, within)| within);
    // Bounded: `holds` allows two frames deep, and the key refuses a loop.
    while let Some(frame) = at.filter(|_| depth < 8) {
        depth += 1;
        at = placement(tx, studio_id, &frame)?.and_then(|(_, within)| within);
    }
    Ok(depth)
}

/// Whether `from` produced `to`: how a Note is said to be in a Cluster.
fn produced(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    from: &StudioNodeId,
    to: &StudioNodeId,
) -> Result<bool, StudioError> {
    tx.query_row(
        "SELECT 1 FROM studio_edges \
         WHERE studio_id = ?1 AND from_node = ?2 AND to_node = ?3 AND kind = ?4",
        (
            studio_id.as_str(),
            from.as_str(),
            to.as_str(),
            StudioEdgeKind::Produced.as_wire(),
        ),
        |_| Ok(()),
    )
    .optional()
    .map(|found| found.is_some())
    .map_err(database("reading what a Cluster holds"))
}

fn carried(id: String) -> StudioNodeId {
    StudioNodeId::carried(core_model::Ulid::carried(id))
}

fn no_node(node_id: &StudioNodeId) -> StudioError {
    StudioError::NoSuchNode {
        node_id: node_id.as_str().to_string(),
    }
}
