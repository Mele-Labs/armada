//! What a project crosses the wire as: the answer with its members, place and the files set aside, and
//! the bodies that make one and change one, each as Bridge's mirror in `packages/protocol` spells it.

use serde_json::json;

use crate::{EditProject, NewProject, ProjectLeftOut, Projects, RunCloud, RunPlace};

#[test]
fn the_answer_carries_members_place_and_what_would_not_read() {
    let answer = json!({
        "projects": [
            {
                "id": "shop", "name": "Shop", "rows": [],
                "members": [{"manifest_id": "web"}, {"manifest_id": "mono", "folder": "apps/api"}],
                "place": {"place": "synced", "synced_folder": "/Users/him/Sync"}
            },
            {
                "id": "lost", "name": "Lost", "rows": [], "members": [],
                "place": {"place": "cloud", "cloud": {"bucket": "s3://b/p"}},
                "unread": "the object is not there"
            }
        ],
        "loose": [],
        "left_out": [{"place": "this_mac", "path": "/kit/projects/broken.yml", "why": "not a project"}]
    });
    let read: Projects = serde_json::from_value(answer.clone()).unwrap();
    assert_eq!(read.projects[0].members[1].folder.as_deref(), Some("apps/api"));
    assert_eq!(read.projects[1].unread.as_deref(), Some("the object is not there"));
    assert_eq!(read.projects[1].place.cloud, Some(RunCloud::Bucket("s3://b/p".to_string())));
    assert_eq!(read.left_out, [ProjectLeftOut { place: RunPlace::ThisMac, path: "/kit/projects/broken.yml".into(), why: "not a project".into() }]);
    assert_eq!(serde_json::to_value(&read).unwrap(), answer, "nothing is added or dropped on the way back");
}

#[test]
fn a_project_whose_file_read_says_nothing_of_unread() {
    let one = json!({"id": "a", "name": "A", "rows": [], "members": [], "place": {"place": "this_mac"}});
    let read: crate::Project = serde_json::from_value(one.clone()).unwrap();
    assert_eq!(read.unread, None);
    assert_eq!(serde_json::to_value(&read).unwrap(), one);
}

#[test]
fn a_new_project_needs_only_a_name_and_an_edit_only_what_changes() {
    let bare: NewProject = crate::decode("a project to add", br#"{"name":"Shop"}"#).unwrap();
    assert!((bare.members.is_none(), bare.place.is_none()) == (true, true));
    assert_eq!(serde_json::to_value(&bare).unwrap(), json!({"name": "Shop"}));

    let whole: NewProject = crate::decode(
        "a project to add",
        br#"{"name":"Shop","members":[{"manifest_id":"web"}],"place":{"place":"cloud","cloud":{"store":{}}}}"#,
    )
    .unwrap();
    assert_eq!(whole.members.as_ref().map(Vec::len), Some(1));
    assert_eq!(whole.place.as_ref().and_then(|one| one.cloud.clone()), Some(RunCloud::Store {}));
    assert!(crate::decode::<NewProject>("a project to add", br#"{"members":[]}"#).is_err(), "a name is required");

    let nothing: EditProject = crate::decode("an edit", b"{}").unwrap();
    assert_eq!(nothing, EditProject::default());
    assert_eq!(serde_json::to_value(&nothing).unwrap(), json!({}), "an absent field is not written");
    let cleared: EditProject = crate::decode("an edit", br#"{"members":[]}"#).unwrap();
    assert_eq!(cleared.members, Some(Vec::new()), "present and empty is every member taken away");
    assert_eq!(serde_json::to_value(&cleared).unwrap(), json!({"members": []}));
}

#[test]
fn a_project_carries_only_the_values_it_sets() {
    let with = json!({
        "id": "a", "name": "A", "rows": [], "members": [], "place": {"place": "this_mac"},
        "values": {"pr_mode": "draft", "auto_merge": "checks-pass", "look": {"density": "roomy"}}
    });
    let read: crate::Project = serde_json::from_value(with.clone()).unwrap();
    let values = read.values.clone().expect("the values it sets");
    assert_eq!((values.pr_mode, values.auto_merge.as_deref(), values.review_model.as_deref()), (Some(crate::PrMode::from(core_model::PrMode::Draft)), Some("checks-pass"), None));
    assert_eq!(values.look.and_then(|look| look.density).as_deref(), Some("roomy"));
    assert_eq!(serde_json::to_value(&read).unwrap(), with, "nothing is added or dropped on the way back");
}

#[test]
fn a_new_project_and_an_edit_carry_values_and_an_empty_block_is_not_an_absent_one() {
    let new: NewProject = crate::decode("a project to add", br#"{"name":"Shop","values":{"review_model":"big"}}"#).unwrap();
    assert_eq!(new.values.and_then(|values| values.review_model).as_deref(), Some("big"));
    let absent: EditProject = crate::decode("an edit", br#"{"name":"Shop"}"#).unwrap();
    assert_eq!(absent.values, None, "absent leaves the values as they are");
    let cleared: EditProject = crate::decode("an edit", br#"{"values":{}}"#).unwrap();
    assert_eq!(cleared.values, Some(crate::ValuesSet::default()), "an empty block takes every value back");
    assert_eq!(serde_json::to_value(&cleared).unwrap(), json!({"values": {}}));
}

#[test]
fn a_word_that_is_not_a_pull_request_mode_is_refused_by_the_decoder() {
    assert!(crate::decode::<EditProject>("an edit", br#"{"values":{"pr_mode":"soon"}}"#).is_err());
}

#[test]
fn resolved_values_name_the_layer_of_each_key_and_leave_an_empty_one_without_a_value() {
    let answer = json!({
        "pr_mode": {"value": "draft", "from": "project"},
        "auto_merge": {"value": "never", "from": "default"},
        "review_model": {"from": "default"},
        "layout": {"from": "default"},
        "theme": {"value": "paper", "from": "machine"},
        "density": {"value": "roomy", "from": "kit"},
        "autonomy": [{"kind": "forge_write", "value": "asks", "from": "repository"}]
    });
    let read: crate::ResolvedValues = serde_json::from_value(answer.clone()).unwrap();
    assert_eq!(read.pr_mode.from, crate::ValueLayer::Project);
    assert_eq!(read.review_model.value, None);
    assert_eq!(read.autonomy[0].from, crate::ValueLayer::Repository);
    assert_eq!(serde_json::to_value(&read).unwrap(), answer);
    let layers = ["default", "machine", "kit", "repository", "project", "job"];
    for layer in layers {
        let one: crate::ValueLayer = serde_json::from_value(json!(layer)).unwrap();
        assert_eq!(serde_json::to_value(one).unwrap(), json!(layer));
    }
}

#[test]
fn a_projects_tracker_is_the_manifest_block_as_a_map_and_a_resolved_one_names_its_layer() {
    let edit: EditProject = crate::decode("an edit", br#"{"values":{"tracker":{"provider":"tickets","site":"acme"}}}"#).unwrap();
    let block = edit.values.as_ref().and_then(|values| values.tracker.as_ref()).expect("a tracker");
    assert_eq!((block.get("provider").map(String::as_str), block.get("site").map(String::as_str)), (Some("tickets"), Some("acme")));
    assert_eq!(serde_json::to_value(&edit).unwrap(), json!({"values": {"tracker": {"provider": "tickets", "site": "acme"}}}));
    assert!(crate::decode::<EditProject>("an edit", br#"{"values":{"tracker":"tickets"}}"#).is_err(), "a tracker is a map of text");

    let answer = json!({"value": {"provider": "tickets"}, "from": "project"});
    let resolved: crate::ResolvedTracker = serde_json::from_value(answer.clone()).unwrap();
    assert_eq!(resolved.from, crate::ValueLayer::Project);
    assert_eq!(serde_json::to_value(&resolved).unwrap(), answer);
    let none: crate::ResolvedTracker = serde_json::from_value(json!({"from": "default"})).unwrap();
    assert_eq!(none.value, None, "no layer names one: the forge's own");
}

#[test]
fn a_project_issue_list_is_an_issue_list_named_for_its_project() {
    let answer = json!({"project": "shop", "fetched_at": "2026-11-10T12:00:00Z", "issues": [], "stale": "the tracker would not answer"});
    let read: crate::ProjectIssueList = serde_json::from_value(answer.clone()).unwrap();
    assert_eq!((read.project.as_str(), read.stale.as_deref()), ("shop", Some("the tracker would not answer")));
    assert_eq!(serde_json::to_value(&read).unwrap(), answer);
    let fresh: crate::ProjectIssueList = serde_json::from_value(json!({"project": "shop", "fetched_at": "2026-11-10T12:00:00Z", "issues": []})).unwrap();
    assert_eq!(fresh.stale, None);
}

#[test]
fn kit_values_are_saved_whole() {
    let body: crate::SaveKitValues = crate::decode("Kit's values", br#"{"values":{"auto_merge":"always"}}"#).unwrap();
    assert_eq!(body.values.auto_merge.as_deref(), Some("always"));
    assert!(crate::decode::<crate::SaveKitValues>("Kit's values", b"{}").is_err(), "the block is required, an empty one taking every value back");
}
