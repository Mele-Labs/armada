//! What a binding draws when the data lacks what it names.

use serde_json::Value;

use super::draw;
use super::fixtures::{Fixtures, Scope};
use super::markup;
use crate::look::Look;

const JOBS: &str = r#"{"jobs":[{"title":"First","so_far":"Two files read"},{"title":"Second"},{"title":"Third","so_far":null}]}"#;

fn scene(body: &str) -> (markup::Scene, Fixtures, Vec<String>) {
    let scene = markup::parse(&format!("<scene title=\"t\"><fixture name=\"jobs\" type=\"json\"></fixture><state id=\"one\">{body}</state></scene>")).expect("the scene reads");
    let (fixtures, trouble) = Fixtures::read(&scene.fixtures, &[("jobs".to_string(), JOBS.to_string())]);
    (scene, fixtures, trouble)
}

fn bound(fixtures: &Fixtures, item: &Value, text: &str) -> String {
    let locals = [("job".to_string(), item)];
    let mut reads = Vec::new();
    Scope { fixtures, locals: &locals }.bind(text, &mut reads)
}

#[test]
fn a_field_missing_on_one_item_draws_empty_and_is_no_trouble() {
    let (scene, fixtures, _) = scene(r#"<list each="jobs.jobs" as="job"><row title="{job.title}" says="{job.so_far}"></row></list>"#);
    let without: Value = serde_json::from_str(r#"{"title":"Second"}"#).unwrap();
    assert_eq!(bound(&fixtures, &without, "{job.so_far}"), "");
    assert_eq!(bound(&fixtures, &without, "Now {job.so_far}."), "Now .");
    let trouble = draw::check(&Look::dark(), &scene, &fixtures).into_lines();
    assert!(trouble.is_empty(), "{trouble:?}");
}

#[test]
fn a_path_matching_no_item_goes_to_trouble() {
    let (scene, fixtures, _) = scene(r#"<list each="jobs.jobs" as="job"><row title="{job.title}" says="{job.so_farr}"></row></list><text>{jobs.jobs.0.titel}</text>"#);
    let trouble = draw::check(&Look::dark(), &scene, &fixtures).into_lines();
    assert_eq!(trouble, ["`{job.so_farr}` matches no field in jobs", "`{jobs.jobs.0.titel}` matches no field in jobs"]);
}
