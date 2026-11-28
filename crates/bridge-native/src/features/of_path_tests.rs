use std::path::Path;

use serde_json::json;

use super::mock;
use super::of_path::{by_component, merged, mock_of_path, mock_resolve, place_of, relative, Place};

#[test]
fn a_path_inside_the_repository_is_made_relative_to_it_and_one_outside_has_none() {
    let root = Some(Path::new("/src/armada"));
    assert_eq!(relative("/src/armada/crates/fleet/src/start.rs", root).as_deref(), Some("crates/fleet/src/start.rs"));
    assert_eq!(relative("./apps/desktop/src/a.tsx", root).as_deref(), Some("apps/desktop/src/a.tsx"));
    assert_eq!(relative("apps/desktop/src/a.tsx", None).as_deref(), Some("apps/desktop/src/a.tsx"));
    assert_eq!(relative("/elsewhere/a.tsx", root), None);
    assert_eq!(relative("/src/armada/a.tsx", None), None, "an absolute path with no repository known");
    assert_eq!(relative("  ", root), None);
}

#[test]
fn a_notes_place_is_its_files_and_its_components_each_once() {
    let raw = json!({
        "source": { "file": "/src/armada/apps/desktop/src/jobs/JobRow.tsx", "line": 3 },
        "definition": { "component": "JobRow", "file": "apps/desktop/src/jobs/JobRow.tsx" },
        "component": "Title", "owners": ["JobRow", "JobList", "Board"],
    });
    let place = place_of(&raw, Some(Path::new("/src/armada")));
    assert_eq!(place.files, ["apps/desktop/src/jobs/JobRow.tsx"]);
    assert_eq!(place.components, ["Title", "JobRow", "JobList", "Board"]);
}

#[test]
fn the_features_a_screen_named_are_read_once_each() {
    let one = json!({ "project": "p1", "slug": "start-a-job", "title": "Start a Job", "area": "Running a Job" });
    let raw = json!({ "paths": ["crates/bridge-native/src/job_view"], "features": [one, one, { "project": "p1", "slug": "" }] });
    let place = place_of(&raw, None);
    assert_eq!(place.files, ["crates/bridge-native/src/job_view"]);
    assert_eq!(place.named.iter().map(|one| (one.slug.as_str(), one.area.as_str())).collect::<Vec<_>>(), [("start-a-job", "Running a Job")]);
}

#[test]
fn a_note_the_native_layer_wrote_names_neither_a_file_nor_a_component() {
    let raw = json!({ "component": null, "owners": [], "element": { "tag": "button", "text": "Send", "label": null } });
    assert_eq!(place_of(&raw, None), Place::default());
}

#[test]
fn features_from_several_files_are_listed_once_each_in_the_order_they_came() {
    let answers = [mock_of_path("apps/desktop/src/renderer/src/jobs/A.tsx"), mock_of_path("apps/desktop/src/renderer/src/dispatch/B.tsx"), mock_of_path("apps/desktop/src/renderer/src/jobs/C.tsx")];
    let slugs: Vec<String> = merged(&answers).into_iter().map(|one| one.slug).collect();
    assert_eq!(slugs, ["job-list-and-live-status", "pause-resume-stop-snooze", "start-a-job"]);
}

#[test]
fn a_component_maps_by_the_words_of_its_name_against_a_features_slug_and_title() {
    let library = mock::library();
    let slugs = |names: &[&str]| -> Vec<String> { by_component(&library, &names.iter().map(|name| name.to_string()).collect::<Vec<_>>()).into_iter().map(|one| one.slug).collect() };
    assert_eq!(slugs(&["ThreadComposer"]), ["thread-and-composer"]);
    assert_eq!(slugs(&["ThreadView"]), ["thread-and-composer"], "a view is what it is, not what it is about");
    assert_eq!(slugs(&["JobList"]), ["job-list-and-live-status"]);
    assert_eq!(slugs(&["pause_resume"]), ["pause-resume-stop-snooze"]);
    assert_eq!(slugs(&["Terminals"]), ["terminal-jobs"], "a plural is its singular");
    // What a component is, not what it draws, says nothing; so a name that is only that maps to nothing.
    assert!(slugs(&["Button"]).is_empty());
    assert!(slugs(&["PanelView"]).is_empty());
    assert!(slugs(&["Zebra"]).is_empty());
    // Every word must be there: Job and Zebra is no feature.
    assert!(slugs(&["JobZebra"]).is_empty());
    // The innermost name that maps counts, and the ones after it are not read.
    assert_eq!(slugs(&["Zebra", "Composer", "Terminals"]), ["thread-and-composer"]);
}

/// A stand-in Fleet that maps `crates/bridge-native/src/jobs.rs` to the Job list in the project `p1` alone, and
/// refuses a read that names no project, as Fleet does a caller outside any Job.
fn fleet_mapping_one_file() -> u16 {
    use std::io::{BufRead, BufReader, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("its address").port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().expect("the stream"));
            let mut first = String::new();
            if reader.read_line(&mut first).is_err() {
                continue;
            }
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                line.clear();
            }
            let route = first.split_whitespace().nth(1).unwrap_or_default().to_string();
            let (status, body) = if !route.contains("project=") {
                ("422 Unprocessable Entity", r#"{"code":"fleet.feature_project","message":"name a project, or call from a job that works in one","run_id":"r"}"#.to_string())
            } else if route.contains("project=p1") && route.contains("jobs.rs") {
                let job = r#"{"session_id":"s-1","title":"Fix the rail"}"#;
                ("200 OK", format!(r#"{{"path":"crates/bridge-native/src/jobs.rs","features":[{{"project":"p1","slug":"job-list-and-live-status","title":"Job list and live status","area":"Running a Job","how":"prefix","jobs":[{job}]}}]}}"#))
            } else {
                ("200 OK", r#"{"path":"x","features":[]}"#.to_string())
            };
            let mut stream = stream;
            let _ = write!(stream, "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
        }
    });
    port
}

/// The title bar shows every project, so no project is scoped: the note's files are read in each project Fleet lists.
#[gpui::test]
fn a_note_made_with_no_project_scoped_reads_its_files_in_each_project(cx: &mut gpui::TestAppContext) {
    let port = fleet_mapping_one_file();
    let wanted = super::of_path::Wanted { port, project: None, projects: vec!["p0".into(), "p1".into()] };
    let place = Place { files: vec!["crates/bridge-native/src/jobs.rs".into(), "crates/bridge-native/src/jobs".into()], ..Place::default() };
    let found = std::rc::Rc::new(std::cell::RefCell::new(None));
    let into = found.clone();
    cx.spawn(async move |cx| *into.borrow_mut() = Some(super::of_path::resolve(wanted, place, &cx).await)).detach();
    for _ in 0..100 {
        cx.run_until_parked();
        if found.borrow().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let found = found.borrow().clone().expect("the read ends");
    assert_eq!(found.iter().map(|one| one.slug.as_str()).collect::<Vec<_>>(), ["job-list-and-live-status"], "the Jobs list's file maps in p1");
    assert_eq!(found[0].jobs.iter().map(|job| job.title.as_str()).collect::<Vec<_>>(), ["Fix the rail"]);
}

#[test]
fn the_mock_reads_files_first_and_the_component_where_no_file_maps() {
    let by_file = Place { files: vec!["apps/desktop/src/renderer/src/jobs/JobRow.tsx".into()], components: vec!["Composer".into()], ..Place::default() };
    assert_eq!(mock_resolve(&by_file).len(), 2);
    let by_name = Place { files: vec!["somewhere/else.tsx".into()], components: vec!["Composer".into()], ..Place::default() };
    assert_eq!(mock_resolve(&by_name).iter().map(|one| one.slug.as_str()).collect::<Vec<_>>(), ["thread-and-composer"]);
    assert!(mock_resolve(&Place::default()).is_empty());
}
