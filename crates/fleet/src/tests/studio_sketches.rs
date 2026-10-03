//! A Sketch is the pad's drawing, kept on a Studio: its pictures are files
//! Fleet names, served by picture, carried over a redraw by id, and gone with
//! the node. Decided with the owner, 1 Oct 2026.

use std::path::Path;

use api::{Redirector, Refusal, Studios};
use ipc::{
    AddStudioNode, CreateStudio, EditStudioSketch, RemoveStudioNodes, SketchDrawn, Studio,
    StudioNodeAdded, StudioNodeContent, StudioNodeId, StudioPosition,
};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn a_fleet(home: &TempDir) -> Fixture {
    Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[])))
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

async fn a_studio(fleet: &Fixture) -> Studio {
    fleet
        .create_studio(CreateStudio { name: None }, None)
        .await
        .expect("created in the repository Fleet starts in")
}

/// A PNG Bridge's main staged, of `bytes`, as the picture JSON names it.
fn staged(home: &TempDir, name: &str, bytes: &[u8]) -> String {
    let path = home.path().join(name);
    std::fs::write(&path, bytes).expect("a picture to stage");
    format!(
        r#"{{"staged_path":{},"width":1280,"height":800}}"#,
        ipc::encode(&path.to_string_lossy().to_string()).expect("a path")
    )
}

/// A drawing of two joined boxes and the pictures given, each `(id, staged)`
/// where `staged` is the JSON of a staged file or `None` for one already kept.
fn drawn(pictures: &[(&str, Option<String>)]) -> SketchDrawn {
    let pictures: Vec<String> = pictures
        .iter()
        .map(|(id, staged)| match staged {
            Some(staged) => format!(
                r#"{{"id":"{id}","x":0,"y":300,"width":320,"height":200,"staged":{staged}}}"#
            ),
            None => format!(r#"{{"id":"{id}","x":0,"y":300,"width":320,"height":200}}"#),
        })
        .collect();
    let body = format!(
        r#"{{"boxes":[{{"id":"b1","x":0,"y":0,"body":"The rail"}},{{"id":"b2","x":300,"y":0,"body":"The board"}}],"joins":[{{"id":"b1-b2","from":"b1","to":"b2"}}],"strokes":[],"pictures":[{}]}}"#,
        pictures.join(",")
    );
    ipc::decode("a drawing", body.as_bytes()).expect("a drawing")
}

fn a_sketch(drawing: SketchDrawn) -> AddStudioNode {
    AddStudioNode {
        content: StudioNodeAdded::Sketch { drawing },
        position: StudioPosition { x: 40, y: 80 },
        produced_by: None,
        within: None,
    }
}

/// Every file the Studio keeps beside its records, sorted.
fn kept_files(fleet: &Fixture, studio: &Studio) -> Vec<String> {
    let dir = Path::new(&fleet.host().studio_frames_dir).join(studio.id.as_str());
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    names.sort();
    names
}

fn the_sketch(studio: &Studio) -> (StudioNodeId, ipc::SketchDrawing) {
    let node = studio.nodes.first().expect("the Sketch");
    let StudioNodeContent::Sketch { drawing } = &node.content else {
        panic!("a Sketch, and it read {:?}", node.content);
    };
    (node.id.clone(), drawing.clone())
}

/// **A Sketch placed with a picture keeps the picture as a file Fleet named**,
/// reads back whole after Fleet is assembled again, and serves that picture
/// by its id on the drawing.
#[tokio::test]
async fn a_sketch_keeps_its_picture_and_serves_it_by_id() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let png = staged(&home, "pasted.png", &[5u8; 300]);

    fleet
        .add_studio_node(
            studio.id.clone(),
            a_sketch(drawn(&[("p1", Some(png))])),
            Redirector::Person,
            None,
        )
        .await
        .expect("a person's own kind");
    drop(fleet);

    let fleet = a_fleet(&home);
    let read = fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("still there");
    let (id, drawing) = the_sketch(&read);
    let domain = drawing.to_domain();
    assert_eq!(domain.boxes().len(), 2);
    assert_eq!(domain.joins().len(), 1);
    let kept = &drawing.pictures()[0].frame.filename;
    assert!(
        kept.starts_with(&format!("{}-", id.as_str())),
        "named by Fleet: {kept}"
    );
    assert_eq!(kept_files(&fleet, &studio), vec![kept.clone()]);

    let (name, bytes) = fleet
        .get_studio_frame(studio.id.clone(), id.clone(), Some("p1".into()), None)
        .await
        .expect("the picture that was kept");
    assert_eq!(&name, kept);
    assert_eq!(bytes, [5u8; 300], "the file itself");
    let refused = fleet
        .get_studio_frame(studio.id.clone(), id, Some("p9".into()), None)
        .await
        .expect_err("no picture by that id");
    assert_eq!(code(&refused), "fleet.studio_frame_not_kept");
}

/// **A redraw keeps a picture named by its id, keeps a new one, and deletes
/// the one it dropped** — once the record no longer names it. A picture with
/// nothing staged under an id the Sketch never held is refused, and the new
/// picture staged beside it is taken back.
#[tokio::test]
async fn a_redraw_carries_kept_pictures_over_and_lets_dropped_ones_go() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let first = staged(&home, "first.png", &[1u8; 100]);
    let added = fleet
        .add_studio_node(
            studio.id.clone(),
            a_sketch(drawn(&[("p1", Some(first))])),
            Redirector::Person,
            None,
        )
        .await
        .expect("placed");
    let (id, before) = the_sketch(&added);
    let p1 = before.pictures()[0].frame.filename.clone();

    let second = staged(&home, "second.png", &[2u8; 200]);
    let redrawn = fleet
        .edit_studio_sketch(
            studio.id.clone(),
            EditStudioSketch {
                node_id: id.clone(),
                drawing: drawn(&[("p1", None), ("p2", Some(second))]),
            },
            None,
        )
        .await
        .expect("redrawn");
    let (_, after) = the_sketch(&redrawn);
    assert_eq!(
        after.pictures()[0].frame.filename,
        p1,
        "p1 is the file it was"
    );
    let p2 = after.pictures()[1].frame.filename.clone();
    assert_eq!(kept_files(&fleet, &studio).len(), 2);

    let third = staged(&home, "third.png", &[3u8; 30]);
    let refused = fleet
        .edit_studio_sketch(
            studio.id.clone(),
            EditStudioSketch {
                node_id: id.clone(),
                drawing: drawn(&[("p3", Some(third)), ("p9", None)]),
            },
            None,
        )
        .await
        .expect_err("p9 was never kept");
    assert_eq!(code(&refused), "fleet.studio_sketch_picture_not_kept");
    assert_eq!(
        kept_files(&fleet, &studio).len(),
        2,
        "p3's file was taken back"
    );

    fleet
        .edit_studio_sketch(
            studio.id.clone(),
            EditStudioSketch {
                node_id: id,
                drawing: drawn(&[("p2", None)]),
            },
            None,
        )
        .await
        .expect("p1 dropped");
    assert_eq!(
        kept_files(&fleet, &studio),
        vec![p2],
        "p1's file went with it"
    );
}

/// **Deleting a Sketch deletes every picture it kept**, as a Picture's goes.
#[tokio::test]
async fn deleting_a_sketch_deletes_every_picture_it_kept() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let one = staged(&home, "one.png", &[1u8; 10]);
    let two = staged(&home, "two.png", &[2u8; 10]);
    let added = fleet
        .add_studio_node(
            studio.id.clone(),
            a_sketch(drawn(&[("p1", Some(one)), ("p2", Some(two))])),
            Redirector::Person,
            None,
        )
        .await
        .expect("placed");
    assert_eq!(kept_files(&fleet, &studio).len(), 2);
    let (id, _) = the_sketch(&added);

    fleet
        .remove_studio_nodes(
            studio.id.clone(),
            RemoveStudioNodes { node_ids: vec![id] },
            None,
        )
        .await
        .expect("deleted");
    assert!(
        kept_files(&fleet, &studio).is_empty(),
        "nothing is left behind"
    );
}

/// **Nothing drawn is a blank Sketch**, refused on the way in and on a redraw,
/// and a drawing is refused on any node that is not a Sketch.
#[tokio::test]
async fn a_blank_drawing_and_a_redraw_of_another_kind_are_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let nothing: SketchDrawn = ipc::decode(
        "a drawing",
        br#"{"boxes":[],"joins":[],"strokes":[],"pictures":[]}"#,
    )
    .expect("an empty drawing decodes");
    let refused = fleet
        .add_studio_node(
            studio.id.clone(),
            a_sketch(nothing),
            Redirector::Person,
            None,
        )
        .await
        .expect_err("nothing drawn");
    assert_eq!(code(&refused), "fleet.studio_node_blank");

    let note: StudioNodeContent =
        ipc::decode("a Note", br#"{"kind":"note","said":"pointed"}"#).expect("a Note");
    let added = fleet
        .add_studio_node(
            studio.id.clone(),
            AddStudioNode {
                content: note.try_into().expect("a write"),
                position: StudioPosition { x: 0, y: 0 },
                produced_by: None,
                within: None,
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a Note");
    let refused = fleet
        .edit_studio_sketch(
            studio.id.clone(),
            EditStudioSketch {
                node_id: added.nodes[0].id.clone(),
                drawing: drawn(&[]),
            },
            None,
        )
        .await
        .expect_err("a Note is not drawn on");
    assert_eq!(code(&refused), "fleet.studio_not_a_sketch");
}
