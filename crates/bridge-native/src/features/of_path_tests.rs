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

#[test]
fn the_mock_reads_files_first_and_the_component_where_no_file_maps() {
    let by_file = Place { files: vec!["apps/desktop/src/renderer/src/jobs/JobRow.tsx".into()], components: vec!["Composer".into()] };
    assert_eq!(mock_resolve(&by_file).len(), 2);
    let by_name = Place { files: vec!["somewhere/else.tsx".into()], components: vec!["Composer".into()] };
    assert_eq!(mock_resolve(&by_name).iter().map(|one| one.slug.as_str()).collect::<Vec<_>>(), ["thread-and-composer"]);
    assert!(mock_resolve(&Place::default()).is_empty());
}
