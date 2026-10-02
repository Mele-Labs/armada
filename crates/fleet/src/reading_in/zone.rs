//! Where what a scout brought back is laid out: inside the read-in's Zone,
//! right of its Finding, with each Cluster drawn round its Notes. `#1620`,
//! decided with the owner on 2 Oct 2026.
//!
//! **A function of the answer, so it is tested as one.** Nothing here mints or
//! writes; it answers which Cluster each Note sits in and where everything
//! goes, measured from the corner of whatever holds the Finding.

use core_model::StudioPosition;
use ipc::ReadIn;

use super::{laid_out, ACROSS, DOWN};
use crate::framing::{HEAD, INSET};

/// Where each thing a read-in made goes, in the order the answer gave them.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Laid {
    /// Each Cluster's corner, beside the Finding's.
    pub(crate) clusters: Vec<StudioPosition>,
    /// Each Note: the Cluster it is drawn in, by its place in `clusters`, and
    /// its spot — from that Cluster's corner, or beside the Finding where it is
    /// in none.
    pub(crate) notes: Vec<(Option<usize>, StudioPosition)>,
    pub(crate) contradictions: Vec<StudioPosition>,
}

/// `read` with every Cluster that would hold fewer than two Notes taken out.
/// Its Notes still land, unclustered.
///
/// **Grouping by hand refuses a Cluster of fewer than two**, and a read-in is
/// held to the same: the owner, 2 Oct 2026, of a one-note frame. Repeated until
/// nothing changes, because a Note drawn in a dropped Cluster may be the second
/// a later one names.
pub(crate) fn without_thin_clusters(mut read: ReadIn) -> ReadIn {
    loop {
        let mut held: Vec<Vec<&str>> = vec![Vec::new(); read.clusters.len()];
        for note in &read.notes {
            let first = read
                .clusters
                .iter()
                .position(|cluster| cluster.of.iter().any(|named| named == &note.id));
            if let Some(cluster) = first {
                if !held[cluster].contains(&note.id.as_str()) {
                    held[cluster].push(&note.id);
                }
            }
        }
        let thick: Vec<bool> = held.iter().map(|notes| notes.len() >= 2).collect();
        if thick.iter().all(|&kept| kept) {
            return read;
        }
        let mut each = thick.into_iter();
        read.clusters.retain(|_| each.next().unwrap_or(false));
    }
}

/// Lay out `read` beside a Finding at `from`.
///
/// **A Note is in the first Cluster that names it, and in no other** — the
/// cost the owner took for a Cluster drawn as a frame: one Note in two frames
/// would be drawn twice. A Note no Cluster names, and every Contradiction,
/// are laid out down columns after the last Cluster.
pub(crate) fn laid_out_beside(from: StudioPosition, read: &ReadIn) -> Laid {
    let in_cluster = |handle: &str| {
        read.clusters
            .iter()
            .position(|cluster| cluster.of.iter().any(|named| named == handle))
    };
    let mut column = from.x + ACROSS;
    let clusters: Vec<StudioPosition> = read
        .clusters
        .iter()
        .map(|_| {
            let corner = StudioPosition {
                x: column,
                y: from.y,
            };
            column += ACROSS;
            corner
        })
        .collect();
    let loose_from = StudioPosition {
        x: column,
        y: from.y,
    };
    let mut held = vec![0i64; read.clusters.len()];
    let mut loose = 0i64;
    let notes = read
        .notes
        .iter()
        .map(|note| match in_cluster(&note.id) {
            Some(cluster) => {
                let at = StudioPosition {
                    x: INSET,
                    y: HEAD + held[cluster] * DOWN,
                };
                held[cluster] += 1;
                (Some(cluster), at)
            }
            None => {
                loose += 1;
                (None, laid_out(loose_from, loose - 1))
            }
        })
        .collect();
    let contradictions = read
        .contradictions
        .iter()
        .map(|_| {
            loose += 1;
            laid_out(loose_from, loose - 1)
        })
        .collect();
    Laid {
        clusters,
        notes,
        contradictions,
    }
}

#[cfg(test)]
mod tests {
    use ipc::{ReadInCluster, ReadInContradiction, ReadInNote};

    use super::*;

    fn note(id: &str) -> ReadInNote {
        ReadInNote {
            id: id.to_string(),
            said: id.to_string(),
        }
    }

    fn cluster(of: &[&str]) -> ReadInCluster {
        ReadInCluster {
            title: "Risks".to_string(),
            of: of.iter().map(|one| one.to_string()).collect(),
        }
    }

    #[test]
    fn each_cluster_frames_its_notes_and_a_note_is_in_the_first_that_names_it() {
        let read = ReadIn {
            notes: vec![note("n1"), note("n2"), note("n3")],
            clusters: vec![cluster(&["n1", "n3"]), cluster(&["n3", "n2"])],
            contradictions: vec![],
            relations: vec![],
        };
        let laid = laid_out_beside(StudioPosition { x: 24, y: 48 }, &read);
        assert_eq!(
            laid.clusters,
            vec![
                StudioPosition {
                    x: 24 + ACROSS,
                    y: 48
                },
                StudioPosition {
                    x: 24 + 2 * ACROSS,
                    y: 48
                },
            ]
        );
        assert_eq!(
            laid.notes[0],
            (Some(0), StudioPosition { x: INSET, y: HEAD })
        );
        assert_eq!(
            laid.notes[1],
            (Some(1), StudioPosition { x: INSET, y: HEAD })
        );
        assert_eq!(
            laid.notes[2],
            (
                Some(0),
                StudioPosition {
                    x: INSET,
                    y: HEAD + DOWN
                }
            ),
            "named by both, drawn in the first"
        );
    }

    #[test]
    fn what_no_cluster_names_goes_in_columns_after_the_last_cluster() {
        let read = ReadIn {
            notes: vec![note("n1"), note("loose")],
            clusters: vec![cluster(&["n1"])],
            contradictions: vec![ReadInContradiction {
                id: "c1".to_string(),
                first: "a".to_string(),
                second: "b".to_string(),
            }],
            relations: vec![],
        };
        let laid = laid_out_beside(StudioPosition { x: 0, y: 0 }, &read);
        let after = StudioPosition {
            x: 2 * ACROSS,
            y: 0,
        };
        assert_eq!(laid.notes[1], (None, after));
        assert_eq!(laid.contradictions[0], laid_out(after, 1));
    }
}
