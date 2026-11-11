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
