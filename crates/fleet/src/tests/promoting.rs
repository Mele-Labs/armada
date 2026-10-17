//! Promotion through Fleet: two Notes clustered, the Cluster written up, the
//! draft edited and dispatched, and a Contradiction ended four ways. `#1291`.
//!
//! **What is proved here is the record, not the rendering.** Every rung
//! answers with the Studio whole, so each assertion reads the nodes and edges
//! that came back rather than a return value of its own.
//!
//! **Nothing here reaches a forge**, and nothing could: no seam in this crate
//! offers one to a Studio, which is the point — `docs/concepts/studio.md`
//! makes filing a person's own act.
//!
//! **Over 500 lines, and staying one file**, `crate::tests::proposing`'s
//! reason: the rungs share a Studio and two of them end a Contradiction as
//! they go, so a change to one has to be checked against the others it runs
//! beside. Split by rung, the four-outcomes test would have nowhere to live.

use api::{Redirector, Refusal, Studios};
use core_model::StudioNodeKind;
use ipc::{
    AddStudioNode, ContradictionSettled, CreateStudio, DeferOnStudio, GroupStudioNodes, SettleContradiction, Studio, StudioNode, StudioNodeContent,
    StudioNodeId, StudioPosition, WriteUpStudioNode,
};
use testkit::FakeWorkProduct;

use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Fixture = crate::daemon::Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

const FIRST_NOTE: &str = "The chip keeps its count after the filter is cleared";
const SECOND_NOTE: &str = "Overview still says three waiting after I answered one";
const DRAFT_TITLE: &str = "Counts go stale after the thing they count changes";
const DRAFT_BODY: &str = "Two places show a count that does not move.";

fn a_fleet(home: &TempDir) -> Fixture {
    crate::daemon::Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[])))
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

fn at(x: i64, y: i64) -> StudioPosition {
    StudioPosition { x, y }
}

async fn a_studio(fleet: &Fixture) -> Studio {
    fleet
        .create_studio(
            CreateStudio {
                name: Some("Stale counts".to_string()),
            },
            None,
        )
        .await
        .expect("a Studio in the repository Fleet starts in")
}

/// A node no route mints, written straight through the store.
///
/// **A Contradiction is the one kind this test needs and nothing can add**:
/// reading in is what makes one (`#1293`), and a person adds a Note, a Link or
/// a Sketch by hand and nothing else (`#1364`). So the four outcomes are
/// exercised against a Contradiction the record holds, rather than through a
/// door that would have to be opened for a test.
async fn seeded(fleet: &Fixture, studio: &Studio, content: StudioNodeContent) -> StudioNodeId {
    let id = studio.id.to_domain();
    let at = fleet.now();
    let node = core_model::StudioNode::added(
        core_model::StudioNodeId::carried(fleet.mint().ulid()),
        content.to_domain(),
        core_model::StudioPosition { x: 0, y: 0 },
        at.clone(),
        core_model::StudioAuthor::Person,
    );
    let mut store = fleet.store().lock().await;
    store.add_studio_node(&id, &node, None, &at).expect("added");
    StudioNodeId::from(node.id())
}

async fn added(fleet: &Fixture, studio: &Studio, content: StudioNodeContent) -> StudioNodeId {
    let studio = fleet
        .add_studio_node(
            studio.id.clone(),
            AddStudioNode {
                content: content.try_into().expect("not a picture"),
                position: at(0, 0),
                produced_by: None,
                within: None,
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a person's node");
    studio.nodes.last().expect("the node just added").id.clone()
}

/// The two Notes the bench's Studio is worked out from.
async fn two_notes(fleet: &Fixture, studio: &Studio) -> (StudioNodeId, StudioNodeId) {
    let note = |said: &str| StudioNodeContent::Note {
        said: said.to_string(),
        capture: None,
    };
    (
        added(fleet, studio, note(FIRST_NOTE)).await,
        added(fleet, studio, note(SECOND_NOTE)).await,
    )
}

fn of_kind<'a>(studio: &'a Studio, kind: StudioNodeKind) -> Vec<&'a StudioNode> {
    studio
        .nodes
        .iter()
        .filter(|node| node.content.to_domain().kind() == kind)
        .collect()
}

/// Every `produced` edge into `node`, as the ids that made it.
fn made_by<'a>(studio: &'a Studio, node: &StudioNodeId) -> Vec<&'a StudioNodeId> {
    studio
        .edges
        .iter()
        .filter(|edge| &edge.to == node && edge.kind.as_wire() == "produced")
        .map(|edge| &edge.from)
        .collect()
}

/// **Only a person defers, and the Deferral says what it holds up.** The
/// `blocks` edge lands accepted, since the person drawing it is the one who
/// would have accepted it.
#[tokio::test]
async fn a_deferral_is_produced_by_what_raised_it_and_blocks_what_it_holds_up() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let (first, second) = two_notes(&fleet, &studio).await;

    let deferred = fleet
        .defer_on_studio(
            studio.id.clone(),
            DeferOnStudio {
                what: "Whether a count is the Board's or the row's".to_string(),
                raised_on: first.clone(),
                blocks: Some(second.clone()),
                position: at(0, 240),
            },
            None,
        )
        .await
        .expect("a person's Deferral");
    let [deferral] = of_kind(&deferred, StudioNodeKind::Deferral)[..] else {
        panic!("one Deferral");
    };
    assert_eq!(deferral.state.map(|state| state.as_wire()), Some("open"));
    assert_eq!(made_by(&deferred, &deferral.id), [&first]);
    let blocks = deferred
        .edges
        .iter()
        .find(|edge| edge.kind.as_wire() == "blocks")
        .expect("what it holds up");
    assert_eq!((&blocks.from, &blocks.to), (&deferral.id, &second));
    assert_eq!(
        blocks.standing.as_wire(),
        "accepted",
        "a person drawing a relation is the person who accepts one"
    );
}

/// A Contradiction ends one of four ways, and each of the four leaves the node
/// saying which. **The two that make a node are the rungs that make it** — so
/// there is one way to write up and one way to defer, not two.
#[tokio::test]
async fn a_contradiction_ends_one_of_four_ways_and_never_twice() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let disagreement = || StudioNodeContent::Contradiction {
        first: "The chip reads the Board".to_string(),
        second: "The chip reads its own row".to_string(),
        answer: None,
    };
    let state = |studio: &Studio, node: &StudioNodeId| {
        studio
            .nodes
            .iter()
            .find(|one| &one.id == node)
            .and_then(|one| one.state)
            .map(|state| state.as_wire())
    };

    // Written up: the Contradiction ends as `issue_draft`, and the draft is a
    // node on the far end of a Produced edge.
    let written_up = seeded(&fleet, &studio, disagreement()).await;
    let after = fleet
        .write_up_studio_node(
            studio.id.clone(),
            WriteUpStudioNode {
                node_id: written_up.clone(),
                title: DRAFT_TITLE.to_string(),
                body: DRAFT_BODY.to_string(),
                position: at(240, 0),
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a Contradiction written up");
    assert_eq!(state(&after, &written_up), Some("issue_draft"));

    // Deferred: it ends as `deferral`.
    let put_off = seeded(&fleet, &studio, disagreement()).await;
    let after = fleet
        .defer_on_studio(
            studio.id.clone(),
            DeferOnStudio {
                what: "Which of the two the chip should read".to_string(),
                raised_on: put_off.clone(),
                blocks: None,
                position: at(0, 480),
            },
            None,
        )
        .await
        .expect("a Contradiction deferred");
    assert_eq!(state(&after, &put_off), Some("deferral"));

    // Not a problem, and Resolved here, which keeps the answer on the node.
    let held = seeded(&fleet, &studio, disagreement()).await;
    let after = fleet
        .settle_contradiction(
            studio.id.clone(),
            SettleContradiction {
                node_id: held.clone(),
                outcome: ContradictionSettled::NotAProblem,
            },
            None,
        )
        .await
        .expect("both statements holding");
    assert_eq!(state(&after, &held), Some("not_a_problem"));

    let settled = seeded(&fleet, &studio, disagreement()).await;
    let after = fleet
        .settle_contradiction(
            studio.id.clone(),
            SettleContradiction {
                node_id: settled.clone(),
                outcome: ContradictionSettled::ResolvedHere {
                    answer: "The chip reads the Board, and the row is stale".to_string(),
                },
            },
            None,
        )
        .await
        .expect("a person settling it");
    assert_eq!(state(&after, &settled), Some("resolved_here"));
    let node = after
        .nodes
        .iter()
        .find(|one| one.id == settled)
        .expect("the Contradiction");
    let StudioNodeContent::Contradiction { answer, .. } = &node.content else {
        panic!("a Contradiction");
    };
    assert_eq!(
        answer.as_deref(),
        Some("The chip reads the Board, and the row is stale"),
        "Resolved here is only a record if the answer is on the node"
    );

    // A Contradiction ends once: the node keeps what was acted on.
    let again = fleet
        .settle_contradiction(
            studio.id.clone(),
            SettleContradiction {
                node_id: settled,
                outcome: ContradictionSettled::NotAProblem,
            },
            None,
        )
        .await
        .expect_err("one that has already ended");
    assert_eq!(code(&again), "fleet.studio_contradiction_settled");
}

/// **A Cluster is drawn round its Notes**, inside the Zone they share, and a
/// Note is in one Cluster at a time — the owner, 2 Oct 2026, `#1620`. A Zone
/// is put on the board by hand, and moving one carries what it holds.
#[tokio::test]
async fn a_cluster_frames_its_notes_inside_their_zone_and_a_note_is_in_one() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let zone = added(&fleet, &studio, StudioNodeContent::Zone).await;
    let (first, second) = two_notes(&fleet, &studio).await;
    for (note, y) in [(&first, 60), (&second, 260)] {
        fleet
            .move_studio_node(
                studio.id.clone(),
                ipc::MoveStudioNode {
                    node_id: note.clone(),
                    within: Some(zone.clone()),
                    position: at(40, y),
                },
                None,
            )
            .await
            .expect("put in the Zone");
    }

    let clustered = fleet
        .group_studio_nodes(
            studio.id.clone(),
            GroupStudioNodes {
                content: StudioNodeContent::Cluster {
                    title: "Counts go stale".to_string(),
                },
                from: vec![first.clone(), second.clone()],
                position: at(5000, 5000),
            },
            None,
        )
        .await
        .expect("clustered");
    let [cluster] = of_kind(&clustered, StudioNodeKind::Cluster)[..] else {
        panic!("one Cluster");
    };
    assert_eq!(cluster.within.as_ref(), Some(&zone), "inside the Zone");
    assert_eq!(
        cluster.position,
        at(16, 12),
        "round its Notes, not where it was asked"
    );
    for note in [&first, &second] {
        let held = clustered.nodes.iter().find(|n| &n.id == note).unwrap();
        assert_eq!(held.within.as_ref(), Some(&cluster.id));
    }

    let third = added(
        &fleet,
        &studio,
        StudioNodeContent::Note {
            said: "A third".to_string(),
            capture: None,
        },
    )
    .await;
    let refused = fleet
        .group_studio_nodes(
            studio.id.clone(),
            GroupStudioNodes {
                content: StudioNodeContent::Cluster {
                    title: "Again".to_string(),
                },
                from: vec![first.clone(), third],
                position: at(0, 0),
            },
            None,
        )
        .await
        .expect_err("already in a Cluster");
    assert_eq!(code(&refused), "fleet.studio_note_in_a_cluster");

    let moved = fleet
        .move_studio_node(
            studio.id.clone(),
            ipc::MoveStudioNode {
                node_id: zone.clone(),
                within: None,
                position: at(900, 900),
            },
            None,
        )
        .await
        .expect("the Zone moved");
    let held = moved.nodes.iter().find(|n| n.id == first).unwrap();
    assert_eq!(
        (held.within.as_ref(), held.position),
        (Some(&cluster.id), at(24, 48)),
        "it came with the Zone, untouched"
    );
}

/// **A node pressed inside a Zone is added in it, in one write**, measured
/// from its corner — the owner, 2 Oct 2026. What a move would refuse, an add
/// refuses too, and writes nothing: a Zone in a Zone, and anything in a
/// Cluster.
#[tokio::test]
async fn a_node_is_added_inside_a_zone_and_never_a_zone_in_one_or_into_a_cluster() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let zone = added(&fleet, &studio, StudioNodeContent::Zone).await;
    let in_frame = |content: StudioNodeContent, frame: &StudioNodeId| AddStudioNode {
        content: content.try_into().expect("not a picture"),
        position: at(40, 60),
        produced_by: None,
        within: Some(frame.clone()),
    };
    let note = |said: &str| StudioNodeContent::Note {
        said: said.to_string(),
        capture: None,
    };

    let inside = fleet
        .add_studio_node(
            studio.id.clone(),
            in_frame(note("Pressed in the Zone"), &zone),
            Redirector::Person,
            None,
        )
        .await
        .expect("added in the Zone");
    let made = inside.nodes.last().expect("the node just added");
    assert_eq!(
        (made.within.as_ref(), made.position),
        (Some(&zone), at(40, 60)),
        "in the Zone, from its corner"
    );

    let (first, second) = two_notes(&fleet, &studio).await;
    let clustered = fleet
        .group_studio_nodes(
            studio.id.clone(),
            GroupStudioNodes {
                content: StudioNodeContent::Cluster {
                    title: "Counts go stale".to_string(),
                },
                from: vec![first, second],
                position: at(0, 0),
            },
            None,
        )
        .await
        .expect("clustered");
    let cluster = of_kind(&clustered, StudioNodeKind::Cluster)[0].id.clone();

    for (content, frame) in [
        (StudioNodeContent::Zone, &zone),
        (note("Pressed in the Cluster"), &cluster),
    ] {
        let refused = fleet
            .add_studio_node(
                studio.id.clone(),
                in_frame(content, frame),
                Redirector::Person,
                None,
            )
            .await
            .expect_err("a move there is refused");
        assert_eq!(code(&refused), "fleet.studio_frame_cannot_hold");
    }
    let after = fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("read");
    assert_eq!(after.nodes.len(), clustered.nodes.len(), "nothing written");
}
