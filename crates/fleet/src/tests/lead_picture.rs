//! `get_lead_picture`: the picture a Job last showed, which the native done card leads with. The newest by
//! when it was shown, a sketch or window withdrawn skipped, and the same notion of "shown a picture" the Look
//! `all_done` check holds the Job to.

use std::collections::BTreeMap;

use api::Sessions;
use ipc::{LeadPicture, SessionId, ShowWindow, WithdrawWindow};
use store::{AttachmentState, Holder, KeptAttachment};

use super::session_host::{rig_placing, Rig};
use crate::session_host::picture_keeping::served_address;

/// An artifact the Job holds, with the detail given.
async fn held(rig: &Rig, id: &SessionId, target: &str, detail: &[(&str, &str)]) {
    let now = rig.fleet.now().as_str().to_string();
    let artifact = KeptAttachment {
        holder: Holder::session(id.as_str()),
        kind: "artifact".into(),
        manifest_id: rig.manifest.as_str().to_string(),
        target: target.into(),
        state: AttachmentState::Standing,
        detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect::<BTreeMap<_, _>>(),
        since: now.clone(),
        changed_at: now,
    };
    rig.fleet.store().lock().await.attach(&artifact, false).expect("kept");
}

/// A sketch drawn at `at`, an instant as a row id writes it.
async fn sketch(rig: &Rig, id: &SessionId, title: &str, at: &str) {
    let drawn = format!("{}-{at}-1", id.as_str());
    held(rig, id, &format!("sketch:{title}"), &[("form", "sketch"), ("by", "agent"), ("title", title), ("drawn", &drawn)]).await;
}

/// A file shown at `at`, an RFC3339 instant.
async fn file(rig: &Rig, id: &SessionId, path: &str, at: &str) {
    held(rig, id, path, &[("form", "file"), ("by", "agent"), ("shown", at)]).await;
}

async fn lead(rig: &Rig, id: &SessionId) -> Result<LeadPicture, (u16, String)> {
    rig.fleet.get_lead_picture(id.clone()).await.map_err(|why| (why.status(), why.error().code.clone()))
}

fn image(id: &SessionId, path: &str) -> LeadPicture {
    LeadPicture::Image { path: served_address(id.as_str(), path) }
}

#[tokio::test]
async fn a_job_that_showed_no_picture_is_a_404_with_a_named_code() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert_eq!(lead(&rig, &id).await, Err((404, "fleet.no_lead_picture".to_string())));

    file(&rig, &id, "/tmp/notes.md", "2026-11-13T10:00:00.000Z").await;
    assert_eq!(lead(&rig, &id).await, Err((404, "fleet.no_lead_picture".to_string())), "a text file is no picture");

    let unknown = lead(&rig, &SessionId::carried("nobody")).await;
    assert_eq!(unknown, Err((404, "fleet.no_such_session".to_string())));
}

#[tokio::test]
async fn a_sketch_then_an_image_leads_with_the_image() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    sketch(&rig, &id, "Header", "20261113T100000000Z").await;
    file(&rig, &id, "/tmp/after.png", "2026-11-13T10:05:00.000Z").await;
    assert_eq!(lead(&rig, &id).await, Ok(image(&id, "/tmp/after.png")));
    let LeadPicture::Image { path } = lead(&rig, &id).await.unwrap() else { unreachable!() };
    assert!(path.starts_with(&format!("/sessions/shown?session_id={}&path=%2Ftmp%2Fafter.png", id.as_str())), "{path}");
}

#[tokio::test]
async fn an_image_then_a_sketch_leads_with_the_sketch() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    file(&rig, &id, "/tmp/before.jpg", "2026-11-13T10:00:00.000Z").await;
    sketch(&rig, &id, "Header", "20261113T100500000Z").await;
    assert_eq!(lead(&rig, &id).await, Ok(LeadPicture::Sketch { title: "Header".into() }));
}

#[tokio::test]
async fn a_withdrawn_last_falls_back_to_the_one_before() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    file(&rig, &id, "/tmp/before.png", "2026-11-13T10:00:00.000Z").await;
    sketch(&rig, &id, "Header", "20261113T100500000Z").await;
    assert_eq!(lead(&rig, &id).await, Ok(LeadPicture::Sketch { title: "Header".into() }));

    rig.fleet.sketch_withdrawn_mark(id.as_str(), "sketch:Header").await;
    assert_eq!(lead(&rig, &id).await, Ok(image(&id, "/tmp/before.png")), "the sketch he took back is skipped");

    held(&rig, &id, "http://localhost:5173/", &[("form", "window"), ("shown", "2026-11-13T10:10:00.000Z")]).await;
    assert_eq!(lead(&rig, &id).await, Ok(LeadPicture::Window { url: "http://localhost:5173/".into() }));
    rig.fleet
        .window_withdrawn(None, WithdrawWindow { url: "http://localhost:5173/".into(), session_id: Some(id.clone()) })
        .await
        .expect("withdrawn");
    assert_eq!(lead(&rig, &id).await, Ok(image(&id, "/tmp/before.png")), "and so is the window");
}

#[tokio::test]
async fn every_picture_withdrawn_is_none_to_lead_with() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    sketch(&rig, &id, "Header", "20261113T100500000Z").await;
    rig.fleet.sketch_withdrawn_mark(id.as_str(), "sketch:Header").await;
    assert_eq!(lead(&rig, &id).await, Err((404, "fleet.no_lead_picture".to_string())));
}

#[tokio::test]
async fn a_window_shown_leads_with_its_url_and_a_page_shown_again_is_the_newest() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let show = |url: &str| ShowWindow { url: url.into(), title: Some("Findings".into()), session_id: Some(id.clone()), ..Default::default() };
    file(&rig, &id, "/tmp/early.png", "2000-01-01T00:00:00.000Z").await;
    rig.fleet.show_window(None, show("http://localhost:5173/")).await.expect("shown");
    assert_eq!(lead(&rig, &id).await, Ok(LeadPicture::Window { url: "http://localhost:5173/".into() }));
}
