//! A Picture a person pastes onto a Studio: kept as a Note's frame is, served
//! by its node, and gone with it. Decided with the owner, 28 Sep 2026.

use std::path::{Path, PathBuf};

use api::{Redirector, Refusal, Studios};
use ipc::{
    AddStudioNode, CreateStudio, RemoveStudioNodes, StagedFrame, Studio, StudioNodeAdded,
    StudioNodeContent, StudioPosition,
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

/// A PNG Bridge's main staged, `bytes` of it.
fn staged(home: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = home.path().join(name);
    std::fs::write(&path, bytes).expect("a frame to stage");
    path
}

fn a_picture(path: &Path) -> AddStudioNode {
    AddStudioNode {
        content: StudioNodeAdded::Picture {
            staged: StagedFrame {
                staged_path: path.to_string_lossy().to_string(),
                width: 1280,
                height: 800,
            },
        },
        position: StudioPosition { x: 40, y: 80 },
        produced_by: None,
    }
}

/// Every file the Studio keeps beside its records.
fn kept_files(fleet: &Fixture, studio: &Studio) -> Vec<String> {
    let dir = Path::new(&fleet.host().studio_frames_dir).join(studio.id.as_str());
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect()
}

/// **A pasted image lands as a Picture whose frame is a file Fleet kept**, and
/// reads back — after Fleet is assembled again — as `kind: picture` naming
/// that file, which `get_studio_frame` serves byte for byte, as a Note's is.
#[tokio::test]
async fn a_person_pastes_a_picture_and_its_frame_is_kept_and_served() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let png = staged(&home, "pasted.png", &[9u8; 700]);

    let added = fleet
        .add_studio_node(studio.id.clone(), a_picture(&png), Redirector::Person, None)
        .await
        .expect("a person's own kind");
    let node = added.nodes.first().expect("the Picture");
    let id = node.id.clone();
    drop(fleet);

    let fleet = a_fleet(&home);
    let read = fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("still there");
    let node = read.nodes.first().expect("the Picture");
    assert_eq!(node.id, id);
    assert_eq!(node.added_by.map(|by| by.as_wire()), Some("person"));
    assert_eq!(node.state, None, "a Picture holds no state");
    assert_eq!(
        ipc::encode(&node.content).expect("plain data"),
        format!(
            r#"{{"kind":"picture","frame":{{"filename":"{}.png","byte_size":700,"width":1280,"height":800}}}}"#,
            id.as_str()
        )
    );

    let (name, bytes) = fleet
        .get_studio_frame(studio.id.clone(), id.clone(), None)
        .await
        .expect("the frame that was kept");
    assert_eq!(name, format!("{}.png", id.as_str()));
    assert_eq!(bytes, [9u8; 700], "the file itself");
}

/// **Over the cap is refused, and nothing is left behind**: no node, and no
/// file under the Studio. Unreadable is refused the same way. A Picture from
/// Helm is refused before its file is touched, since Helm adds only what
/// starts proposed.
#[tokio::test]
async fn a_picture_that_cannot_be_kept_or_is_not_a_persons_writes_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;

    let heavy = staged(&home, "heavy.png", &vec![0u8; 4 * 1024 * 1024 + 1]);
    let refused = fleet
        .add_studio_node(
            studio.id.clone(),
            a_picture(&heavy),
            Redirector::Person,
            None,
        )
        .await
        .expect_err("a frame over the cap");
    assert_eq!(code(&refused), "fleet.studio_frame_too_large");

    let missing = home.path().join("never-written.png");
    let refused = fleet
        .add_studio_node(
            studio.id.clone(),
            a_picture(&missing),
            Redirector::Person,
            None,
        )
        .await
        .expect_err("a staged frame that is not there");
    assert_eq!(code(&refused), "fleet.studio_frame_unreadable");

    let light = staged(&home, "light.png", &[1u8; 64]);
    let refused = fleet
        .add_studio_node(studio.id.clone(), a_picture(&light), Redirector::Helm, None)
        .await
        .expect_err("Helm adds only what starts proposed");
    assert_eq!(code(&refused), "fleet.studio_node_not_helms");

    // A `produced_by` naming no node of this Studio is refused by the store,
    // after the file was kept — and the file goes back out with it.
    let mut orphan = a_picture(&light);
    orphan.produced_by = Some(ipc::StudioNodeId::carried("01NOSUCHNODE"));
    fleet
        .add_studio_node(studio.id.clone(), orphan, Redirector::Person, None)
        .await
        .expect_err("made from a node that is not there");

    let read = fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("read back");
    assert!(read.nodes.is_empty(), "nothing refused was written");
    assert_eq!(
        kept_files(&fleet, &studio),
        Vec::<String>::new(),
        "and no file kept"
    );
}

/// **Deleting a Picture takes its file**, as deleting a captured Note does —
/// `#1411`. A Picture left standing keeps its own.
#[tokio::test]
async fn deleting_a_picture_deletes_its_frame() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let png = staged(&home, "pasted.png", &[3u8; 128]);
    for _ in 0..2 {
        fleet
            .add_studio_node(studio.id.clone(), a_picture(&png), Redirector::Person, None)
            .await
            .expect("a person's own kind");
    }
    let read = fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("read");
    let (going, staying) = (read.nodes[0].id.clone(), read.nodes[1].id.clone());
    assert_eq!(kept_files(&fleet, &studio).len(), 2, "a file each");

    fleet
        .remove_studio_nodes(
            studio.id.clone(),
            RemoveStudioNodes {
                node_ids: vec![going.clone()],
            },
            None,
        )
        .await
        .expect("a person's delete");
    assert_eq!(
        kept_files(&fleet, &studio),
        vec![format!("{}.png", staying.as_str())],
        "the deleted Picture's file goes, and the other's stays"
    );
}

/// **A write never names a kept frame.** A Picture as `get_studio` reads it
/// cannot be made into an `add_studio_node`, so the one way in is the staged
/// file — and the decoder refuses a body that names `frame`, every shape of
/// which `ipc`'s own tests hold.
#[test]
fn a_picture_as_read_is_no_write() {
    let claimed = StudioNodeContent::Picture {
        frame: ipc::CaptureFrame {
            filename: "../../elsewhere.png".to_string(),
            byte_size: 1,
            width: 1,
            height: 1,
        },
    };
    StudioNodeAdded::try_from(claimed).expect_err("its frame is Fleet's to name");
    let body = br#"{"kind":"picture","frame":{"filename":"../../elsewhere.png","byte_size":1,"width":1,"height":1},"position":{"x":0,"y":0}}"#;
    ipc::decode::<AddStudioNode>("a node", body).expect_err("a frame named on a write");
}
