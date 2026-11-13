//! The claim, written before the code: **a picture an option carries is checked, kept and served by
//! Fleet. A file that is missing, is not the size the picture says, or has a part outside it or two
//! parts of one name is refused by name and keeps nothing; a good one is copied into the Job's own
//! files and `image` becomes the address `GET /sessions/shown` serves it at; a file the Job already
//! showed is served where it is.** `docs/practices/protocol/how-he-works-with-jobs-and-a-question-offered-as-pictures.md`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions, ShownFile};
use ipc::{
    OptionPicture, PicturePart, SessionId, SetWaitingFor, ShowFile, WaitingAct, WaitingActKind, WaitingInput,
    WaitingOption,
};

use super::session_host::{rig_placing, Rig};
use crate::session_host::picture_keeping::address_path;

/// The head of a PNG: its signature and an IHDR of this size. Fleet reads no more of one.
pub(super) fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    bytes.extend(b"IHDR");
    bytes.extend(width.to_be_bytes());
    bytes.extend(height.to_be_bytes());
    bytes.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

/// A JPEG of this size: a start, a baseline frame header of three components, an end.
fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0, 0xFF, 0xC0, 0, 17, 8];
    bytes.extend(height.to_be_bytes());
    bytes.extend(width.to_be_bytes());
    bytes.extend([3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1, 0xFF, 0xD9]);
    bytes
}

/// A file in the folder the job runs in.
pub(super) fn plant(rig: &Rig, name: &str, bytes: &[u8]) -> PathBuf {
    let path = Path::new(&rig.root).join(name);
    std::fs::write(&path, bytes).expect("a file in the job's folder");
    std::fs::canonicalize(path).expect("it exists")
}

fn part(name: &str, x: u32, y: u32, w: u32, h: u32) -> PicturePart {
    PicturePart { name: name.into(), label: name.to_uppercase(), x, y, w, h }
}

fn picture(image: &str, width: u32, height: u32, parts: Vec<PicturePart>) -> OptionPicture {
    OptionPicture { image: image.into(), width, height, parts }
}

fn asking(id: &SessionId, picture: OptionPicture) -> SetWaitingFor {
    SetWaitingFor {
        session_id: Some(id.clone()),
        items: vec![WaitingInput {
            approves: false,
            id: "header".into(),
            text: "Which header?".into(),
            act: Some(WaitingAct { kind: WaitingActKind::Answer, target: String::new() }),
            options: vec![
                WaitingOption { label: "Wide".into(), description: None, picture: Some(picture) },
                WaitingOption { label: "Words only".into(), description: None, picture: None },
            ],
            steps: Vec::new(),
        }],
    }
}

async fn refused(rig: &Rig, id: &SessionId, picture: OptionPicture) -> ipc::WireError {
    match rig.fleet.waiting_for(None, asking(id, picture)).await {
        Err(Refusal::Unacceptable(refused)) => {
            let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
            assert!(record.waiting_for.is_empty(), "a refused picture keeps nothing: {:?}", record.waiting_for);
            refused
        }
        other => panic!("refused with 422, and nothing else: {other:?}"),
    }
}

/// The picture the first option is served with, from the row as a client reads it.
async fn served(rig: &Rig, id: &SessionId) -> OptionPicture {
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    record.waiting_for[0].options[0].picture.clone().expect("the option keeps its picture")
}

#[tokio::test]
async fn a_good_picture_is_kept_in_the_jobs_files_and_served_at_the_address_stored() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let bytes = png(1200, 800);
    let original = plant(&rig, "picture-good.png", &bytes);
    let parts = vec![part("nav", 0, 0, 1200, 100), part("hero", 0, 100, 1200, 700)];

    rig.fleet.waiting_for(None, asking(&id, picture("picture-good.png", 1200, 800, parts.clone()))).await.unwrap();
    std::fs::remove_file(&original).unwrap();

    let kept = served(&rig, &id).await;
    assert_eq!((kept.width, kept.height, kept.parts.clone()), (1200, 800, parts), "the rest of the picture stands");
    assert!(kept.image.starts_with(&format!("/sessions/shown?session_id={}&path=%2F", id.as_str())), "{}", kept.image);
    let at = address_path(id.as_str(), &kept.image).expect("the address names a path");
    assert!(Path::new(&at).starts_with(rig.fleet.pictures_of(id.as_str())), "kept in the job's own files: {at}");
    let read = rig.fleet.get_shown_file(id.clone(), at).await.expect("the address serves it");
    assert_eq!(read, ShownFile::Picture { media_type: "image/png".into(), bytes }, "from the copy, the original being gone");

    // Stating the list again with the address Fleet gave keeps the picture as it is.
    rig.fleet.waiting_for(None, asking(&id, kept.clone())).await.unwrap();
    assert_eq!(served(&rig, &id).await, kept);
}

#[tokio::test]
async fn a_jpeg_is_read_by_its_frame_header_and_kept_under_its_own_type() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    plant(&rig, "picture-good.jpg", &jpeg(640, 480));

    rig.fleet.waiting_for(None, asking(&id, picture("picture-good.jpg", 640, 480, vec![part("all", 0, 0, 640, 480)]))).await.unwrap();

    let at = address_path(id.as_str(), &served(&rig, &id).await.image).unwrap();
    assert!(at.ends_with(".jpg"), "{at}");
    let read = rig.fleet.get_shown_file(id.clone(), at).await.unwrap();
    assert!(matches!(read, ShownFile::Picture { ref media_type, .. } if media_type == "image/jpeg"), "{read:?}");
}

#[tokio::test]
async fn a_picture_with_no_file_is_refused_naming_the_option_and_the_path() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let why = refused(&rig, &id, picture("nothing-here.png", 100, 100, Vec::new())).await;
    assert_eq!(why.code, "fleet.picture_missing");
    assert!(why.message.contains("option \"Wide\" of \"header\"") && why.message.contains("nothing-here.png"), "{}", why.message);
    assert!(why.message.contains("browser_screenshot"), "it says what to do: {}", why.message);
}

#[tokio::test]
async fn a_picture_that_is_not_the_size_it_says_is_refused_with_both_sizes() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    plant(&rig, "picture-size.png", &png(1200, 800));
    let why = refused(&rig, &id, picture("picture-size.png", 600, 400, Vec::new())).await;
    assert_eq!(why.code, "fleet.picture_size");
    assert!(why.message.contains("600 by 400") && why.message.contains("1200 by 800"), "{}", why.message);
}

#[tokio::test]
async fn a_part_outside_the_picture_is_refused_by_name() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    plant(&rig, "picture-outside.png", &png(1200, 800));
    for far in [part("hero", 0, 700, 1200, 200), part("hero", 1100, 0, 200, 100)] {
        let why = refused(&rig, &id, picture("picture-outside.png", 1200, 800, vec![part("nav", 0, 0, 1200, 100), far])).await;
        assert_eq!(why.code, "fleet.picture_part_outside");
        assert!(why.message.contains("part \"hero\"") && why.message.contains("1200 by 800"), "{}", why.message);
    }
}

#[tokio::test]
async fn two_parts_of_one_name_are_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    plant(&rig, "picture-twice.png", &png(1200, 800));
    let why = refused(&rig, &id, picture("picture-twice.png", 1200, 800, vec![part("nav", 0, 0, 10, 10), part("nav", 0, 100, 10, 10)])).await;
    assert_eq!(why.code, "fleet.picture_part_repeated");
    assert!(why.message.contains("\"nav\""), "{}", why.message);
}

#[tokio::test]
async fn a_file_that_is_no_png_or_jpeg_or_is_over_eight_mebibytes_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    plant(&rig, "picture-text.png", b"not a picture at all, only words");
    let why = refused(&rig, &id, picture("picture-text.png", 10, 10, Vec::new())).await;
    assert_eq!(why.code, "fleet.picture_not_an_image");

    let mut large = png(100, 100);
    large.resize(8 * 1024 * 1024 + 1, 0);
    plant(&rig, "picture-large.png", &large);
    let why = refused(&rig, &id, picture("picture-large.png", 100, 100, Vec::new())).await;
    assert_eq!(why.code, "fleet.picture_too_large");
}

#[tokio::test]
async fn a_file_the_job_showed_is_accepted_as_it_is_and_copied_nowhere() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let shown = plant(&rig, "picture-shown.png", &png(300, 200));
    rig.fleet.show_file(None, ShowFile { path: shown.to_string_lossy().into(), session_id: Some(id.clone()) }).await.unwrap();

    rig.fleet.waiting_for(None, asking(&id, picture("picture-shown.png", 300, 200, vec![part("all", 0, 0, 300, 200)]))).await.unwrap();

    let at = address_path(id.as_str(), &served(&rig, &id).await.image).unwrap();
    assert_eq!(at, shown.to_string_lossy(), "served from where it was shown");
    assert!(!rig.fleet.pictures_of(id.as_str()).exists(), "nothing was copied");
    assert!(matches!(rig.fleet.get_shown_file(id.clone(), at).await, Ok(ShownFile::Picture { .. })));
}

#[tokio::test]
async fn a_path_nobody_showed_and_nothing_kept_is_not_served() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let loose = plant(&rig, "picture-loose.png", &png(10, 10));
    let climbing = rig.fleet.pictures_of(id.as_str()).join("..").join("..").join("picture-loose.png");
    for path in [loose, climbing] {
        let read = rig.fleet.get_shown_file(id.clone(), path.to_string_lossy().into()).await;
        assert!(matches!(read, Err(Refusal::NotFound(_))), "{path:?}: {read:?}");
    }
}
