//! A feature's pictures, written before the code: **a Job keeps a picture of a screen it built under its feature, the page
//! carries every standing one newest first and the list its cover, one route serves the bytes, and his Not this feature takes
//! one off and keeps why.** `docs/concepts/feature-catalog.md`, *Pictures*.

use api::{Commands, Features};
use ipc::{AddFeaturePicture, FeatureRow, PictureBy, RemoveFeaturePicture, SessionId};

use super::agent_starts::{caller, rig, Rig};
use super::feature_library::VOTING;
use super::feature_reads::{a_job, page};

/// The bytes of a PNG `width` by `height`: its signature and header, which is all Fleet reads.
pub(super) fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(&13u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

/// `bytes` written in the rig's home as `name`, and where.
fn on_disk(rig: &Rig, name: &str, bytes: &[u8]) -> String {
    let path = rig.home.path().join(name);
    std::fs::write(&path, bytes).expect("written");
    path.to_string_lossy().into_owned()
}

fn add(id: &SessionId, path: &str, url: Option<&str>, screen: Option<&str>) -> AddFeaturePicture {
    AddFeaturePicture { feature: VOTING.to_string(), path: path.to_string(), url: url.map(str::to_string), screen: screen.map(str::to_string), project: None, session_id: Some(id.clone()) }
}

async fn row_of(rig: &Rig, project: &str, slug: &str) -> FeatureRow {
    let listed = rig.fleet.list_features(None, Some(project.to_string())).await.expect("listed");
    listed.areas.into_iter().flat_map(|area| area.features).find(|one| one.slug == slug).expect("the feature")
}

#[tokio::test]
async fn a_jobs_picture_is_kept_under_its_feature_with_where_it_came_from_and_the_page_and_the_list_carry_it() {
    let rig = rig(None);
    let (project, id) = a_job(&rig).await;
    let bytes = png(1280, 800);
    let path = on_disk(&rig, "shot.png", &bytes);

    let added = rig.fleet.add_feature_picture(caller(), add(&id, &path, Some("http://localhost:3000/votes"), None)).await.expect("kept");
    std::fs::remove_file(&path).expect("the Job's scratch goes");
    let picture = added.picture;
    assert_eq!(added.feature, VOTING);
    assert_eq!((picture.width, picture.height, picture.bytes), (1280, 800, bytes.len() as u64), "read off the PNG's header");
    assert_eq!((picture.url.as_deref(), picture.screen.as_deref(), picture.by, picture.job.as_deref()), (Some("http://localhost:3000/votes"), None, PictureBy::Job, Some(id.as_str())));

    let kept = crate::runtime::support_dir(&rig.fleet.host().home).join("feature-pictures").join(&project);
    let files: Vec<_> = walkdir(&kept);
    assert_eq!(files.len(), 1, "under the project and the feature: {files:?}");
    assert_eq!(rig.fleet.get_feature_picture(picture.id.clone()).await.expect("served"), bytes, "Fleet's copy, after the Job's went");

    let later = on_disk(&rig, "later.png", &png(900, 600));
    let second = rig.fleet.add_feature_picture(caller(), add(&id, &later, None, Some("features"))).await.expect("kept").picture;
    let voting = page(&rig, &project, VOTING).await;
    let ids: Vec<&str> = voting.pictures.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, [second.id.as_str(), picture.id.as_str()], "newest first");
    assert_eq!(voting.pictures[0].screen.as_deref(), Some("features"));
    assert_eq!(voting.feature.cover.as_deref(), Some(second.id.as_str()), "the cover is the newest");
    assert_eq!(row_of(&rig, &project, VOTING).await.cover.as_deref(), Some(second.id.as_str()));
    assert_eq!(row_of(&rig, &project, "page-actions").await.cover, None, "a feature with no picture has no cover");
}

#[tokio::test]
async fn a_picture_with_no_source_a_file_that_is_no_png_or_an_unknown_feature_is_refused_and_nothing_is_kept() {
    let rig = rig(None);
    let (project, id) = a_job(&rig).await;
    let good = on_disk(&rig, "shot.png", &png(10, 10));
    let text = on_disk(&rig, "notes.png", b"not a picture");

    let none = rig.fleet.add_feature_picture(caller(), add(&id, &good, Some("  "), None)).await.expect_err("no source");
    assert_eq!(none.error().code, "fleet.picture_source");
    for path in [text.as_str(), "/nowhere/shot.png"] {
        let unreadable = rig.fleet.add_feature_picture(caller(), add(&id, path, Some("http://localhost:3000/"), None)).await.expect_err("no PNG");
        assert_eq!(unreadable.error().code, "fleet.picture_unreadable", "{path}");
    }
    let unknown = AddFeaturePicture { feature: "no-such-feature".into(), ..add(&id, &good, Some("http://localhost:3000/"), None) };
    assert_eq!(rig.fleet.add_feature_picture(caller(), unknown).await.expect_err("unknown").error().code, "fleet.feature_unknown");

    assert!(page(&rig, &project, VOTING).await.pictures.is_empty());
    assert!(!crate::runtime::support_dir(&rig.fleet.host().home).join("feature-pictures").exists(), "no file was kept");
}

#[tokio::test]
async fn not_this_feature_takes_a_picture_off_the_page_and_the_cover_keeps_why_and_is_his_alone() {
    let rig = rig(None);
    let (project, id) = a_job(&rig).await;
    let first = rig.fleet.add_feature_picture(caller(), add(&id, &on_disk(&rig, "a.png", &png(10, 10)), Some("http://localhost:3000/votes"), None)).await.expect("kept").picture;
    let second = rig.fleet.add_feature_picture(caller(), add(&id, &on_disk(&rig, "b.png", &png(10, 10)), Some("http://localhost:3000/cart"), None)).await.expect("kept").picture;

    let from_a_job = rig.fleet.remove_feature_picture(caller(), RemoveFeaturePicture { picture: second.id.clone(), why: None }).await.expect_err("his alone");
    assert_eq!(from_a_job.error().code, "fleet.not_the_operator");

    let removed = rig.fleet.remove_feature_picture(None, RemoveFeaturePicture { picture: second.id.clone(), why: Some(" The cart, not voting ".into()) }).await.expect("removed");
    assert_eq!((removed.feature.as_str(), removed.cover.as_deref()), (VOTING, Some(first.id.as_str())), "the cover moves to the next");
    let rules = rig.fleet.list_catalog_rules(Some(project.clone())).await.expect("listed").rules;
    assert_eq!(rules.iter().map(|one| (one.id.as_str(), one.text.as_str(), one.act, one.features.clone())).collect::<Vec<_>>(), [(removed.rule.as_deref().expect("a rule"), "The cart, not voting", ipc::CatalogRuleAct::NotThisFeature, vec![VOTING.to_string()])], "his why is a rule of the catalog");
    let voting = page(&rig, &project, VOTING).await;
    assert_eq!(voting.pictures.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [first.id.as_str()]);
    assert_eq!(row_of(&rig, &project, VOTING).await.cover.as_deref(), Some(first.id.as_str()));

    let kept = rig.fleet.store().lock().await.feature_picture(&second.id).expect("read").expect("the row stays");
    assert_eq!((kept.removed_at.is_some(), kept.removed_why.as_str()), (true, "The cart, not voting"), "what he taught is kept");
    assert!(rig.fleet.get_feature_picture(second.id.clone()).await.is_ok(), "its file stays");

    let again = rig.fleet.remove_feature_picture(None, RemoveFeaturePicture { picture: second.id.clone(), why: Some("Again".into()) }).await.expect("answered as before");
    assert_eq!((again.cover.as_deref(), again.rule.as_deref()), (Some(first.id.as_str()), None));
    assert_eq!(rig.fleet.list_catalog_rules(Some(project.clone())).await.expect("listed").rules.len(), 1, "no second rule");
    assert_eq!(rig.fleet.store().lock().await.feature_picture(&second.id).expect("read").expect("held").removed_why, "The cart, not voting", "a second press changes nothing");

    let unsaid = rig.fleet.remove_feature_picture(None, RemoveFeaturePicture { picture: first.id.clone(), why: None }).await.expect("removed");
    assert_eq!((unsaid.cover, unsaid.rule), (None, None), "none left, and no words make no rule");
}

#[tokio::test]
async fn an_id_fleet_does_not_hold_is_not_found_to_both_the_bytes_and_the_removal() {
    let rig = rig(None);
    let _ = a_job(&rig).await;
    assert_eq!(rig.fleet.get_feature_picture("nope".into()).await.expect_err("unknown").error().code, "fleet.picture_unknown");
    let removal = rig.fleet.remove_feature_picture(None, RemoveFeaturePicture { picture: "nope".into(), why: None }).await.expect_err("unknown");
    assert_eq!(removal.error().code, "fleet.picture_unknown");
}

/// Every file under `dir`, at any depth.
fn walkdir(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walkdir(&path));
        } else {
            found.push(path);
        }
    }
    found
}

// The librarian's `pictures` part, with a capture that starts no browser.

/// The capture every rig gets: nothing is taken, so a test that wants a picture plants [`Taking`].
pub(crate) struct Unseen;

impl crate::picture_capture::Capture for Unseen {
    fn page(&self, _url: &str, _out: &std::path::Path) -> Result<(), String> {
        Err("a test takes no picture".to_string())
    }
    fn walk(&self, _app: &std::path::Path, _walk: &str, _out: &std::path::Path) -> Result<(), String> {
        Err("a test takes no picture".to_string())
    }
}

/// A capture that writes a 640 by 400 PNG for every page, and keeps what it was asked.
#[derive(Default)]
struct Taking {
    asked: std::sync::Mutex<Vec<String>>,
}

impl crate::picture_capture::Capture for Taking {
    fn page(&self, url: &str, out: &std::path::Path) -> Result<(), String> {
        self.asked.lock().unwrap().push(url.to_string());
        std::fs::write(out, png(640, 400)).map_err(|why| why.to_string())
    }
    fn walk(&self, _app: &std::path::Path, walk: &str, _out: &std::path::Path) -> Result<(), String> {
        self.asked.lock().unwrap().push(format!("walk {walk}"));
        Err("no native Bridge in a test".to_string())
    }
}

/// `web` serves, with its link at the server's root.
const SERVING: &str = r#"version: 1
id: 01FIXTUREMANIFEST
ports:
  web: {}
commands:
  web:
    serve: "/bin/sh -c 'sleep 60'"
    links:
      - url: http://localhost:${port.web}/
        name: The shop
"#;

fn rig_taking(taking: &std::sync::Arc<Taking>) -> Rig {
    let planted = std::sync::Arc::clone(taking);
    super::agent_starts::rigged_fitted(None, None, None, move |fittings| {
        fittings.starting().manifest = config::Manifest::parse(std::path::Path::new("armada.yml"), SERVING).expect("parses");
        fittings.port_range = super::servers::a_range_of_its_own();
        fittings.capture = planted;
    })
}

/// The `web` server started by hand and serving, and its port.
async fn serving(rig: &Rig) -> (String, u16) {
    let started = std::sync::Arc::clone(&rig.fleet).start_server(None, ipc::StartServer { name: "web".into(), checkout: None }, None).await.expect("it starts");
    let id = started.id.clone();
    super::session_host::eventually(|| async { rig.fleet.server_list().servers.iter().any(|one| one.id == id && one.phase == ipc::ServerPhase::Serving) }).await;
    (started.id, started.ports.first().expect("a port").port)
}

async fn pressed(rig: &Rig, project: &str) -> ipc::LibrarianRan {
    rig.fleet.run_librarian(None, ipc::RunLibrarian { project: project.to_string() }).await.expect("ran")
}

#[tokio::test]
async fn the_librarian_takes_the_screen_a_jobs_picture_showed_on_the_running_server_and_keeps_one_take_of_its_own() {
    let taking = std::sync::Arc::new(Taking::default());
    let rig = rig_taking(&taking);
    let (project, id) = a_job(&rig).await;
    let jobs = rig.fleet.add_feature_picture(caller(), add(&id, &on_disk(&rig, "votes.png", &png(1280, 800)), Some("http://localhost:3000/votes?tab=open"), None)).await.expect("kept").picture;
    let (server, port) = serving(&rig).await;
    let mut watching = rig.fleet.events().subscribe();

    let ran = pressed(&rig, &project).await;
    assert!(ran.ran.contains(&ipc::LibrarianPart::Pictures), "{:?}", ran.ran);
    assert_eq!(*taking.asked.lock().unwrap(), [format!("http://localhost:{port}/votes?tab=open")], "the Job's path, on the server running now; the other feature is tied to no screen");
    let told = super::librarian::steps_told(&mut watching, &project).await;
    assert!(told.contains(&super::librarian::step("take_pictures", ipc::LibrarianStepState::Done, "Took 1 picture")), "{told:?}");

    let voting = page(&rig, &project, VOTING).await;
    assert_eq!(voting.pictures.len(), 2, "the Job's stays beside the librarian's");
    let take = &voting.pictures[0];
    assert_eq!((take.by, take.job.as_deref(), take.url.clone(), take.width, take.height), (PictureBy::Librarian, None, Some(format!("http://localhost:{port}/votes?tab=open")), 640, 400));
    assert_eq!(voting.feature.cover.as_deref(), Some(take.id.as_str()));
    assert_eq!(rig.fleet.get_feature_picture(take.id.clone()).await.expect("served"), png(640, 400));

    pressed(&rig, &project).await;
    let again = page(&rig, &project, VOTING).await;
    assert_eq!(again.pictures.iter().filter(|one| one.by == PictureBy::Librarian).count(), 1, "a take again replaces its own, or is current");
    assert!(again.pictures.iter().any(|one| one.id == jobs.id), "never the Job's");
    rig.fleet.stopped_server(&server).await.expect("it stops");
}

#[tokio::test]
async fn a_screen_he_took_off_the_feature_is_never_taken_again_for_it_and_no_running_app_takes_nothing() {
    let taking = std::sync::Arc::new(Taking::default());
    let rig = rig_taking(&taking);
    let (project, id) = a_job(&rig).await;
    let jobs = rig.fleet.add_feature_picture(caller(), add(&id, &on_disk(&rig, "votes.png", &png(10, 10)), Some("http://localhost:3000/cart"), None)).await.expect("kept").picture;

    let mut watching = rig.fleet.events().subscribe();
    let ran = pressed(&rig, &project).await;
    assert!(!ran.ran.contains(&ipc::LibrarianPart::Pictures));
    assert!(taking.asked.lock().unwrap().is_empty(), "no app of the project runs");
    let told = super::librarian::steps_told(&mut watching, &project).await;
    assert!(told.contains(&super::librarian::step("take_pictures", ipc::LibrarianStepState::Skipped, "No app of the project is running")), "{told:?}");

    rig.fleet.remove_feature_picture(None, RemoveFeaturePicture { picture: jobs.id, why: Some("That is the cart".into()) }).await.expect("removed");
    let (server, _) = serving(&rig).await;
    let mut watching = rig.fleet.events().subscribe();
    pressed(&rig, &project).await;
    assert!(taking.asked.lock().unwrap().is_empty(), "the cart is not this feature");
    let told = super::librarian::steps_told(&mut watching, &project).await;
    assert!(told.contains(&super::librarian::step("take_pictures", ipc::LibrarianStepState::Skipped, "No feature is tied to a screen yet")), "{told:?}");
    rig.fleet.stopped_server(&server).await.expect("it stops");
}

#[test]
fn a_file_routes_to_the_page_its_framework_serves_it_as_and_a_filled_in_segment_to_none() {
    use crate::feature_library::route_of;
    for (path, route) in [
        ("apps/shop/app/page.tsx", Some("/")),
        ("src/app/(shop)/cart/page.tsx", Some("/cart")),
        ("app/settings/billing/page.jsx", Some("/settings/billing")),
        ("app/votes/[id]/page.tsx", None),
        ("app/votes/layout.tsx", None),
        ("pages/index.tsx", Some("/")),
        ("web/pages/votes.tsx", Some("/votes")),
        ("pages/_app.tsx", None),
        ("pages/api/votes.ts", None),
        ("src/routes/votes/+page.svelte", Some("/votes")),
        ("src/routes/votes/+layout.svelte", None),
        ("crates/pages/voting/ranks.rs", None),
    ] {
        assert_eq!(route_of(path).as_deref(), route, "{path}");
    }
}
