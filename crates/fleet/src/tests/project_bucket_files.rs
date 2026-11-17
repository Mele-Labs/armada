//! A project file kept in a bucket is taken out of it when the project is deleted or kept somewhere
//! else. A bucket that refuses the delete does not stop the act: its answer says the file was left.

use std::sync::Arc;

use api::Projects;
use ipc::{EditProject, NewProject, ProjectMember, ProjectPlace, RunCloud, RunPlace};
use testkit::FakeBucket;

use super::red_claim::Planted;
use super::session_host::{rig_fitted, Rig};
use crate::run_memory::places;
use crate::tests::tmp::TempDir;

const BUCKET: &str = "s3://shop-runs/armada";
const OTHER: &str = "s3://other-runs/armada";

fn bucketed(bucket: &Arc<FakeBucket>) -> Rig {
    let bucket = Arc::clone(bucket);
    rig_fitted(move |fittings| fittings.reaching = Arc::new(Planted::with_bucket(bucket)))
}

fn in_bucket(address: &str) -> ProjectPlace {
    ProjectPlace { place: RunPlace::Cloud, synced_folder: None, cloud: Some(RunCloud::Bucket(address.into())) }
}

/// A project of one member, kept in the bucket, and the object its file is.
async fn kept_in_the_bucket(rig: &Rig) -> (String, String) {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: "Shop".into(), members, place: Some(in_bucket(BUCKET)), values: None };
    let id = rig.fleet.add_project(new).await.expect("added").projects.into_iter().find(|one| one.name == "Shop").expect("shop").id;
    let object = places::project_object(&id);
    (id, object)
}

fn moved_to(place: ProjectPlace) -> EditProject {
    EditProject { place: Some(place), ..Default::default() }
}

#[tokio::test]
async fn deleting_a_project_removes_its_file_from_the_bucket() {
    let bucket = Arc::new(FakeBucket::new());
    let rig = bucketed(&bucket);
    let (id, object) = kept_in_the_bucket(&rig).await;
    assert!(bucket.object(&object).is_some(), "its file is in the bucket");

    let answered = rig.fleet.delete_project(id).await.expect("deleted");

    assert_eq!(bucket.object(&object), None, "taken out of the bucket");
    assert!(answered.projects.is_empty() && answered.left_behind.is_empty(), "{answered:?}");
}

#[tokio::test]
async fn a_bucket_that_refuses_the_delete_leaves_the_file_and_the_answer_says_so_in_its_words() {
    let bucket = Arc::new(FakeBucket::new());
    let rig = bucketed(&bucket);
    let (id, object) = kept_in_the_bucket(&rig).await;
    bucket.set_no_delete(true);

    let answered = rig.fleet.delete_project(id).await.expect("deleted all the same");

    assert!(answered.projects.is_empty(), "the project is deleted");
    assert!(bucket.object(&object).is_some(), "its file stayed");
    let [left] = answered.left_behind.as_slice() else { panic!("{:?}", answered.left_behind) };
    assert_eq!((left.place, left.path.as_str()), (RunPlace::Cloud, object.as_str()));
    assert!(left.why.contains("may not delete here"), "the bucket's words: {}", left.why);
}

#[tokio::test]
async fn moving_a_project_off_a_bucket_removes_the_old_file_and_moving_it_within_one_does_not() {
    let bucket = Arc::new(FakeBucket::new());
    let rig = bucketed(&bucket);
    let (id, object) = kept_in_the_bucket(&rig).await;

    let same = rig.fleet.edit_project(id.clone(), moved_to(in_bucket(BUCKET))).await.expect("kept there");
    assert!(bucket.object(&object).is_some(), "the file it was just written to is not deleted");
    assert!(same.left_behind.is_empty());

    let synced = TempDir::new();
    let there = ProjectPlace { place: RunPlace::Synced, synced_folder: Some(synced.path().to_string_lossy().into_owned()), cloud: None };
    let moved = rig.fleet.edit_project(id.clone(), moved_to(there)).await.expect("moved");

    assert_eq!(bucket.object(&object), None, "the old file is taken out of the bucket");
    assert!(moved.left_behind.is_empty(), "{moved:?}");
    assert!(synced.path().join("projects").join(format!("{id}.yml")).is_file(), "and written in its new place");
}

#[tokio::test]
async fn moving_a_project_off_a_synced_folder_removes_the_old_file() {
    let bucket = Arc::new(FakeBucket::new());
    let rig = bucketed(&bucket);
    let synced = TempDir::new();
    let there = ProjectPlace { place: RunPlace::Synced, synced_folder: Some(synced.path().to_string_lossy().into_owned()), cloud: None };
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let id = rig.fleet.add_project(NewProject { name: "Shop".into(), members, place: Some(there), values: None }).await.expect("added").projects[0].id.clone();
    let old = synced.path().join("projects").join(format!("{id}.yml"));
    assert!(old.is_file());

    rig.fleet.edit_project(id.clone(), moved_to(in_bucket(BUCKET))).await.expect("moved");

    assert!(!old.exists(), "the synced folder's file is taken away");
    assert!(bucket.object(&places::project_object(&id)).is_some(), "and the bucket holds it");
}

#[tokio::test]
async fn a_move_the_old_bucket_refuses_to_clean_up_still_moves_and_says_the_file_was_left() {
    let bucket = Arc::new(FakeBucket::new());
    let rig = bucketed(&bucket);
    let (id, object) = kept_in_the_bucket(&rig).await;
    bucket.set_no_delete(true);

    let moved = rig.fleet.edit_project(id.clone(), moved_to(in_bucket(OTHER))).await.expect("moved all the same");

    let project = moved.projects.iter().find(|one| one.id == id).expect("still a project");
    assert_eq!(project.place.cloud, Some(RunCloud::Bucket(OTHER.into())), "it is kept at the new place");
    let [left] = moved.left_behind.as_slice() else { panic!("{:?}", moved.left_behind) };
    assert_eq!((left.place, left.path.as_str()), (RunPlace::Cloud, object.as_str()));
    assert!(left.why.contains("may not delete here"), "{}", left.why);
}
