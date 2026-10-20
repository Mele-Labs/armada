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
}
