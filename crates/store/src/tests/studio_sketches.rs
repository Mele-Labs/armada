//! A Sketch is a drawing: V85. Decided with the owner, 1 Oct 2026.
//!
//! Tested from the version before it, `studio_files`' way: a V84 file written
//! through raw SQL, so what migrates is a Sketch the owner already has.

use core_model::{
    CaptureFrame, SketchBox, SketchDrawing, SketchJoin, SketchPicture, SketchPoint, SketchStroke,
    StudioAuthor, StudioId, StudioNode, StudioNodeContent, StudioNodeId, StudioPosition, Timestamp,
    Ulid,
};
use rusqlite::Connection;

use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{open, TempDir};
use crate::StudioError;

const AT: &str = "2026-10-01T09:00:00.000Z";

/// A V84 file holding one Studio and one Sketch written as text.
fn a_v84_file(dir: &TempDir) {
    let conn = Connection::open(dir.db()).expect("a file");
    for migration in &MIGRATIONS[..84] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute_batch(&format!(
        "INSERT INTO armada_meta (key, value) VALUES ('{SCHEMA_VERSION_KEY}', '84');
         INSERT INTO studios VALUES ('01OLD', 'armada', NULL, '{AT}', '{AT}', NULL);
         INSERT INTO studio_nodes VALUES ('01SKETCH', '01OLD', 'sketch', 'frozen',
             '{{\"body\":\"Legend on its own row\\nunder the step bar\"}}', 40, 80, '{AT}',
             'person');"
    ))
    .expect("a Studio as V84 wrote it");
}

fn old() -> StudioId {
    StudioId::carried(Ulid::carried("01OLD"))
}

fn at() -> Timestamp {
    Timestamp::from_rfc3339(AT)
}

/// **A Sketch written as text opens as one box holding it**, at its place and
/// with no state: `frozen` went with the text. Nothing a person wrote is lost.
#[test]
fn a_sketch_written_as_text_migrates_to_one_box_holding_it() {
    let dir = TempDir::new();
    a_v84_file(&dir);

    let store = open(&dir);
    let graph = store.studio(&old()).expect("migrates and reads");
    let sketch = &graph.nodes[0];
    assert_eq!(
        sketch.content(),
        &StudioNodeContent::Sketch {
            drawing: SketchDrawing::one_box(String::from(
                "Legend on its own row\nunder the step bar"
            )),
        }
    );
    assert_eq!(sketch.state(), None, "a Sketch holds no state");
    assert_eq!(sketch.position(), StudioPosition { x: 40, y: 80 });
}

fn a_drawing() -> SketchDrawing {
    SketchDrawing::drawn(
        vec![
            SketchBox {
                id: String::from("b1"),
                x: 0,
                y: 0,
                body: String::from("The rail"),
            },
            SketchBox {
                id: String::from("b2"),
                x: 300,
                y: 40,
                body: String::new(),
            },
        ],
        vec![SketchJoin {
            id: String::from("b1-p1"),
            from: String::from("b1"),
            to: String::from("p1"),
        }],
        vec![SketchStroke {
            id: String::from("s1"),
            points: vec![SketchPoint { x: 1, y: 2 }, SketchPoint { x: -3, y: 4 }],
        }],
        vec![SketchPicture {
            id: String::from("p1"),
            x: 10,
            y: 200,
            width: 320,
            height: 200,
            frame: CaptureFrame {
                filename: String::from("01NEW-01FRAME.png"),
                byte_size: 2048,
                width: 1280,
                height: 800,
            },
        }],
    )
    .expect("a drawing")
}

/// **Every part comes back as it went in**: boxes, the join, the hand's line
/// and the picture with the frame Fleet kept for it.
#[test]
fn a_drawing_round_trips_through_the_store() {
    let dir = TempDir::new();
    a_v84_file(&dir);
    let mut store = open(&dir);
    let content = StudioNodeContent::Sketch {
        drawing: a_drawing(),
    };
    let node = StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01NEW")),
        content.clone(),
        StudioPosition { x: 0, y: 0 },
        at(),
        StudioAuthor::Person,
    );
    store
        .add_studio_node(&old(), &node, None, &at())
        .expect("a Sketch writes");
    drop(store);

    let store = open(&dir);
    let read = store.studio(&old()).expect("reads");
    let sketch = read
        .nodes
        .iter()
        .find(|one| one.id().as_str() == "01NEW")
        .expect("the Sketch");
    assert_eq!(sketch.content(), &content);
}

/// **A stored drawing whose join reaches nothing does not read back**, by the
/// rule a request is held to, rather than reaching a person as a line into
/// nothing.
#[test]
fn a_malformed_drawing_in_the_record_is_refused_on_read() {
    let dir = TempDir::new();
    a_v84_file(&dir);
    let store = open(&dir);
    drop(store);
    let conn = Connection::open(dir.db()).expect("the same file");
    conn.execute_batch(
        r#"UPDATE studio_nodes SET content = '{"boxes":[{"id":"b1","x":0,"y":0,"body":""}],
            "joins":[{"id":"j","from":"b1","to":"b9"}],"strokes":[],"pictures":[]}'
           WHERE id = '01SKETCH';"#,
    )
    .expect("a broken row");
    let store = open(&dir);
    match store.studio(&old()) {
        Err(StudioError::Unreadable { .. }) => {}
        other => panic!("a join to nothing is refused, and this read {other:?}"),
    }
}
