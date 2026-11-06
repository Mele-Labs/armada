//! The claim, written before the code: **a file an agent shows is kept on its Session's ledger as an
//! artifact of form `file` it showed, stamped with the time of the call, and a path that is missing, a
//! folder, over what Bridge's panel reads, or shown for no job is refused naming the path as resolved.**
//! `docs/concepts/session.md`, *Showing a file*.

use std::path::{Path, PathBuf};

use api::{Refusal, Sessions};
use ipc::{Attachment, SessionId, ShowFile};

use super::session_host::{rig_placing, Rig};

/// The most Bridge's panel reads: `MOST_BYTES` in `apps/desktop/src/main/session-file.ts`.
const MOST: u64 = 8 * 1024 * 1024;
/// The most a phone is sent of a text file.
const MOST_TEXT: u64 = 1024 * 1024;
/// The most a video may be to be shown.
const MOST_VIDEO: u64 = 256 * 1024 * 1024;

fn showing(id: &SessionId, path: &str) -> ShowFile {
    ShowFile { path: path.to_string(), session_id: Some(id.clone()) }
}

/// A file in the folder the job runs in, named for the test so none shares one.
fn planted(rig: &Rig, name: &str, text: &str) -> PathBuf {
    let path = Path::new(&rig.root).join(name);
    std::fs::write(&path, text).expect("a file in the job's folder");
    std::fs::canonicalize(path).expect("it exists")
}

fn shown(record: &ipc::SessionRecord) -> Vec<&Attachment> {
    record
        .attachments
        .iter()
        .filter(|one| one.kind == "artifact" && one.detail.get("by").map(String::as_str) == Some("agent"))
        .filter(|one| one.detail.get("form").map(String::as_str) == Some("file"))
        .collect()
}

async fn refused(rig: &Rig, show: ShowFile) -> ipc::WireError {
    match rig.fleet.show_file(None, show).await {
        Err(Refusal::Unacceptable(refused)) => refused,
        other => panic!("refused with 422, and nothing else: {other:?}"),
    }
}

#[tokio::test]
async fn a_relative_path_resolves_in_the_jobs_folder_and_is_kept_as_a_file_the_agent_showed() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = planted(&rig, "showing-file-relative.md", "# Notes\n");

    let record = rig.fleet.show_file(None, showing(&id, "showing-file-relative.md")).await.unwrap();
    std::fs::remove_file(&path).ok();

    let kept = shown(&record);
    assert_eq!(kept.len(), 1, "{:?}", record.attachments);
    assert_eq!((kept[0].kind.as_str(), kept[0].target.as_str()), ("artifact", path.to_str().unwrap()));
    assert_eq!(kept[0].detail["form"], "file");
    assert_eq!(kept[0].detail["by"], "agent");
    assert_eq!(kept[0].detail["title"], "showing-file-relative.md");
    let at = &kept[0].detail["shown"];
    assert!(at.len() == 24 && at.ends_with('Z') && at.contains('T'), "an RFC 3339 time, got `{at}`");
}

#[tokio::test]
async fn an_absolute_path_is_kept_as_it_resolves_and_is_read_back_from_the_store() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = planted(&rig, "showing-file-absolute.md", "x");

    rig.fleet.show_file(None, showing(&id, path.to_str().unwrap())).await.unwrap();
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    std::fs::remove_file(&path).ok();

    let record = listed.iter().find(|one| one.id == id).expect("the session is listed");
    assert_eq!(shown(record)[0].target, path.to_str().unwrap());
}

#[tokio::test]
async fn showing_the_same_file_again_is_a_new_show() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = planted(&rig, "showing-file-twice.md", "x");

    let first = rig.fleet.show_file(None, showing(&id, "showing-file-twice.md")).await.unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let second = rig.fleet.show_file(None, showing(&id, "showing-file-twice.md")).await.unwrap();
    std::fs::remove_file(&path).ok();

    assert_eq!(shown(&second).len(), 1, "one row for the path: {:?}", second.attachments);
    assert_ne!(shown(&first)[0].detail["shown"], shown(&second)[0].detail["shown"]);
}

#[tokio::test]
async fn a_path_that_does_not_exist_is_refused_naming_where_it_looked() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let said = refused(&rig, showing(&id, "showing-file-nothing-here.md")).await;
    assert_eq!(said.code, "fleet.file_missing");
    assert!(
        said.message.contains(&format!("{}/showing-file-nothing-here.md", rig.root)),
        "names the path as resolved: {}",
        said.message
    );
    assert!(said.message.contains("Allowed"), "says what is allowed: {}", said.message);
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    assert!(shown(listed.iter().find(|one| one.id == id).unwrap()).is_empty());
}

#[tokio::test]
async fn a_folder_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let said = refused(&rig, showing(&id, &rig.root)).await;
    assert_eq!(said.code, "fleet.file_not_a_file");
    assert!(said.message.contains("folder"), "{}", said.message);
}

#[tokio::test]
async fn a_file_over_what_the_panel_reads_is_refused_and_one_at_the_cap_is_kept() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = planted(&rig, "showing-file-large.bin", "");
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();

    file.set_len(MOST).unwrap();
    rig.fleet.show_file(None, showing(&id, "showing-file-large.bin")).await.expect("at the cap is kept");

    file.set_len(MOST + 1).unwrap();
    let said = refused(&rig, showing(&id, "showing-file-large.bin")).await;
    std::fs::remove_file(&path).ok();
    assert_eq!(said.code, "fleet.file_too_large");
    assert!(
        said.message.contains(path.to_str().unwrap()) && said.message.contains("8388608"),
        "names the path and the cap: {}",
        said.message
    );
}

#[tokio::test]
async fn a_call_that_places_no_job_is_refused() {
    let rig = rig_placing(None);
    let path = planted(&rig, "showing-file-nobody.md", "x");
    let said = refused(&rig, ShowFile { path: path.to_str().unwrap().to_string(), session_id: None }).await;
    assert_eq!(said.code, "fleet.file_unnamed");
    let said = refused(&rig, showing(&SessionId::carried("nobody"), path.to_str().unwrap())).await;
    std::fs::remove_file(&path).ok();
    assert_eq!(said.code, "fleet.file_unnamed");
}

#[tokio::test]
async fn a_file_the_agent_wrote_is_the_same_row_with_who_showed_it_and_when_added() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = planted(&rig, "showing-file-written.md", "x");
    rig.fleet.artifact_made(id.as_str(), path.to_str().unwrap().to_string(), "file").await;

    let record = rig.fleet.show_file(None, showing(&id, "showing-file-written.md")).await.unwrap();
    std::fs::remove_file(&path).ok();

    let rows: Vec<_> = record.attachments.iter().filter(|one| one.target == path.to_str().unwrap()).collect();
    assert_eq!(rows.len(), 1, "{:?}", record.attachments);
    assert_eq!(rows[0].detail["by"], "agent");
    assert!(rows[0].detail.contains_key("shown"));
    assert_eq!(rows[0].detail["written"], "true", "it was written, and showing it does not unwrite it");
}

#[tokio::test]
async fn a_file_written_stays_written_through_every_later_show() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = planted(&rig, "showing-file-written-twice.md", "x");
    rig.fleet.artifact_made(id.as_str(), path.to_str().unwrap().to_string(), "file").await;

    rig.fleet.show_file(None, showing(&id, "showing-file-written-twice.md")).await.unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let record = rig.fleet.show_file(None, showing(&id, "showing-file-written-twice.md")).await.unwrap();
    std::fs::remove_file(&path).ok();

    assert_eq!(shown(&record)[0].detail["written"], "true");
}

#[tokio::test]
async fn a_file_only_shown_was_not_written() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = planted(&rig, "showing-file-only-shown.md", "x");

    rig.fleet.show_file(None, showing(&id, "showing-file-only-shown.md")).await.unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let record = rig.fleet.show_file(None, showing(&id, "showing-file-only-shown.md")).await.unwrap();
    std::fs::remove_file(&path).ok();

    assert!(!shown(&record)[0].detail.contains_key("written"), "{:?}", shown(&record)[0].detail);
}

/// Pocket v2's claim for files: **the phone reads a file only where the job showed it**, as text, as a
/// picture's bytes, or refused by size.
#[tokio::test]
async fn a_shown_file_is_read_back_and_nothing_else_is() {
    use api::ShownFile;
    let rig = rig_placing(None);
    let id = rig.start().await;
    let text = planted(&rig, "shown-read.md", "# Shown\n");
    let picture = planted(&rig, "shown-read.PNG", "\u{89}PNG");
    let large = planted(&rig, "shown-read-large.txt", &"x".repeat(1024 * 1024 + 1));
    let unshown = planted(&rig, "shown-read-never.md", "not shown");
    for path in [&text, &picture, &large] {
        rig.fleet.show_file(None, showing(&id, path.to_str().unwrap())).await.unwrap();
    }
    let read = |path: &PathBuf| rig.fleet.get_shown_file(id.clone(), path.to_str().unwrap().to_string());

    match read(&text).await.unwrap() {
        ShownFile::Text(shown) => assert_eq!((shown.path.as_str(), shown.text.as_str()), (text.to_str().unwrap(), "# Shown\n")),
        other => panic!("text, got {other:?}"),
    }
    assert_eq!(
        read(&picture).await.unwrap(),
        ShownFile::Picture { media_type: "image/png".into(), bytes: "\u{89}PNG".as_bytes().to_vec() }
    );
    assert_eq!(read(&large).await.unwrap(), ShownFile::TooLarge { bytes: 1024 * 1024 + 1 });
    for refused in [read(&unshown).await, rig.fleet.get_shown_file(SessionId::carried("nobody"), text.to_str().unwrap().into()).await] {
        assert!(matches!(&refused, Err(Refusal::NotFound(why)) if why.code == "fleet.shown_file_not_shown"), "{refused:?}");
    }
    std::fs::remove_file(&text).unwrap();
    assert!(matches!(read(&text).await, Err(Refusal::NotFound(why)) if why.code == "fleet.shown_file_gone"));
    for path in [&picture, &large, &unshown] {
        std::fs::remove_file(path).ok();
    }
}

/// A file of `len` bytes that takes no disk: a hole, so a cap of 256 MiB writes nothing.
fn sparse(rig: &Rig, name: &str, len: u64) -> PathBuf {
    let path = planted(rig, name, "");
    std::fs::OpenOptions::new().write(true).open(&path).unwrap().set_len(len).unwrap();
    path
}

fn resized(path: &Path, len: u64) {
    std::fs::OpenOptions::new().write(true).open(path).unwrap().set_len(len).unwrap();
}

/// What a read came to, in words: a failed assertion on the file itself would print a megabyte.
fn briefly(read: &api::ShownFile) -> String {
    match read {
        api::ShownFile::Text(shown) => format!("text of {} bytes", shown.text.len()),
        api::ShownFile::Picture { media_type, bytes } => format!("{media_type} of {} bytes", bytes.len()),
        api::ShownFile::Video { media_type, bytes } => format!("{media_type} of {bytes} bytes, unread"),
        api::ShownFile::TooLarge { bytes } => format!("too large at {bytes} bytes"),
    }
}

/// Shows `path` and reads it back as the phone does.
async fn shown_then_read(rig: &Rig, id: &SessionId, path: &Path) -> api::ShownFile {
    let given = path.to_str().unwrap();
    rig.fleet.show_file(None, showing(id, given)).await.expect("shown");
    rig.fleet.get_shown_file(id.clone(), given.to_string()).await.expect("read back")
}

#[tokio::test]
async fn a_picture_over_one_mib_is_sent_whole() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = sparse(&rig, "shown-picture-over-text-cap.png", MOST_TEXT + 1);

    let read = shown_then_read(&rig, &id, &path).await;
    std::fs::remove_file(&path).ok();

    match read {
        api::ShownFile::Picture { media_type, bytes } => {
            assert_eq!((media_type.as_str(), bytes.len() as u64), ("image/png", MOST_TEXT + 1));
        }
        _ => panic!("a picture over the text cap is still sent, and not as TooLarge"),
    }
}

#[tokio::test]
async fn a_picture_over_eight_mib_is_too_large() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = sparse(&rig, "shown-picture-over-cap.png", MOST);

    assert!(matches!(shown_then_read(&rig, &id, &path).await, api::ShownFile::Picture { .. }), "at the cap it is sent");
    // It grew after it was shown: show_file would refuse it now, and the phone must be told so too.
    resized(&path, MOST + 1);
    let read = rig.fleet.get_shown_file(id.clone(), path.to_str().unwrap().to_string()).await.unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(briefly(&read), format!("too large at {} bytes", MOST + 1));
}

#[tokio::test]
async fn text_over_one_mib_is_too_large() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = sparse(&rig, "shown-text-over-cap.txt", MOST_TEXT);

    assert!(matches!(shown_then_read(&rig, &id, &path).await, api::ShownFile::Text(_)), "at the cap it is sent");
    resized(&path, MOST_TEXT + 1);
    let read = rig.fleet.get_shown_file(id.clone(), path.to_str().unwrap().to_string()).await.unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(briefly(&read), format!("too large at {} bytes", MOST_TEXT + 1));
}

#[tokio::test]
async fn an_svg_is_sent_as_a_picture() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let markup = r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#;
    let path = planted(&rig, "shown-drawing.SVG", markup);

    let read = shown_then_read(&rig, &id, &path).await;
    std::fs::remove_file(&path).ok();

    assert_eq!(read, api::ShownFile::Picture { media_type: "image/svg+xml".into(), bytes: markup.as_bytes().to_vec() });
}

/// A file the Mac will stat and not open, so any read of it fails where its size does not.
fn unreadable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert!(std::fs::read(path).is_err(), "this test needs a file its own process cannot open, and not root");
}

#[tokio::test]
async fn a_video_is_answered_by_its_size_and_never_read() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut paths = Vec::new();
    for (name, media_type) in [("never-read.mp4", "video/mp4"), ("never-read.MOV", "video/quicktime"), ("never-read.webm", "video/webm")] {
        let path = planted(&rig, name, "not really a video");
        let given = path.to_str().unwrap();
        rig.fleet.show_file(None, showing(&id, given)).await.unwrap();
        unreadable(&path);
        let read = rig.fleet.get_shown_file(id.clone(), given.to_string()).await;
        assert_eq!(read.unwrap(), api::ShownFile::Video { media_type: media_type.into(), bytes: 18 }, "{name}");
        paths.push(path);
    }
    for path in paths {
        std::fs::remove_file(path).ok();
    }
}

#[tokio::test]
async fn a_video_over_the_show_cap_is_still_answered_by_its_size() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = sparse(&rig, "shown-video-grew.mp4", MOST_VIDEO);
    let given = path.to_str().unwrap().to_string();
    rig.fleet.show_file(None, showing(&id, &given)).await.unwrap();

    // Grown after it was shown: the size is the file's now, and nothing refuses a video here.
    resized(&path, MOST_VIDEO + 1);
    let read = rig.fleet.get_shown_file(id.clone(), given).await.unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(read, api::ShownFile::Video { media_type: "video/mp4".into(), bytes: MOST_VIDEO + 1 });
}

#[tokio::test]
async fn show_file_takes_a_video_up_to_256_mib() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = sparse(&rig, "showing-video-at-cap.mp4", MOST_VIDEO);
    let record = rig.fleet.show_file(None, showing(&id, "showing-video-at-cap.mp4")).await.expect("at the cap is kept");
    std::fs::remove_file(&path).ok();
    assert_eq!(shown(&record)[0].target, path.to_str().unwrap());
}

#[tokio::test]
async fn show_file_refuses_a_video_over_256_mib() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = sparse(&rig, "showing-video-over-cap.webm", MOST_VIDEO + 1);
    let said = refused(&rig, showing(&id, "showing-video-over-cap.webm")).await;
    std::fs::remove_file(&path).ok();
    assert_eq!(said.code, "fleet.file_too_large");
    assert!(said.message.contains(&format!("{} bytes", MOST_VIDEO + 1)), "names its size: {}", said.message);
}

#[tokio::test]
async fn show_file_refuses_another_file_over_8_mib() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    // A picture, text, and a name that only contains a video's extension: none has the video's cap.
    for name in ["showing-over-8-mib.png", "showing-over-8-mib.txt", "showing-over-8-mib.mp4.txt", "showing-over-8-mib"] {
        let path = sparse(&rig, name, MOST + 1);
        let said = refused(&rig, showing(&id, name)).await;
        std::fs::remove_file(&path).ok();
        assert_eq!(said.code, "fleet.file_too_large", "{name}");
    }
}

#[tokio::test]
async fn the_refusal_names_both_caps() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let path = sparse(&rig, "showing-caps.mp4", MOST_VIDEO + 1);
    let said = refused(&rig, showing(&id, "showing-caps.mp4")).await;
    std::fs::remove_file(&path).ok();
    for cap in ["8388608", "268435456", "mp4, mov or webm"] {
        assert!(said.message.contains(cap), "names `{cap}`: {}", said.message);
    }
}
