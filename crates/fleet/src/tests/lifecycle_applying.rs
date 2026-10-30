//! Which lifecycles a Job's repository has, and what is refused where a workflow ends a step on one
//! it lacks. **A repository nobody serves has the four Armada carries**, so a Job outside one holds
//! what it always could; one that is served has what its pieces say.

use std::collections::BTreeMap;

use core_model::Lifecycle;
use testkit::FakeWorkProduct;

use super::daemon::a_fleet;
use super::lifecycle_waking::{resolved, reviewing};
use super::tmp::TempDir;
use crate::repositories::{Catalogued, WorkflowNotSaved};

const TICKETS: &str = "version: 1\nlifecycle_id: ticket\nname: Ticket\nstates:\n  \
    - { id: in_review, label: In review, read: { tracker: [Review] } }\n  \
    - { id: closed, label: Closed, read: { tracker: [Done] } }\nends: [closed]\n";

fn ids(lifecycles: &[Lifecycle]) -> Vec<&str> {
    let mut ids: Vec<&str> = lifecycles.iter().map(|one| one.id().as_str()).collect();
    ids.sort_unstable();
    ids
}

fn states_of(lifecycles: &[Lifecycle], id: &str) -> usize {
    lifecycles.iter().find(|one| one.id().as_str() == id).expect("a lifecycle").states().len()
}

#[tokio::test]
async fn a_served_repository_has_what_its_pieces_say_and_one_nobody_serves_has_the_four_armada_carries() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();

    let plain = fleet.lifecycles_that_apply(&manifest);
    assert_eq!(ids(&plain), ["deploy", "job", "pr", "ticket"]);
    assert_eq!(states_of(&plain, "ticket"), 4, "Armada's own ticket");

    let kit = home.path().join("kit").join("lifecycles");
    std::fs::create_dir_all(&kit).expect("a folder");
    std::fs::write(kit.join("tickets.yml"), TICKETS).expect("a piece");
    let yours = fleet.lifecycles_that_apply(&manifest);
    assert_eq!(states_of(&yours, "ticket"), 2, "yours in Kit runs over Armada's");

    let outside = fleet.lifecycles_that_apply("01NOBODYSERVESTHIS");
    assert_eq!((ids(&outside), states_of(&outside, "ticket")), (vec!["deploy", "job", "pr", "ticket"], 4), "the built-ins, not a failure");
}

#[tokio::test]
async fn a_workflow_that_ends_a_step_on_a_lifecycle_the_repository_lacks_is_set_aside_with_a_sentence_naming_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
    let good = resolved(&reviewing("pr: approved"));
    let lacking = resolved(&reviewing("pull: approved").replace("workflow_id: rail", "workflow_id: lacking"));
    let mut workflows = BTreeMap::new();
    workflows.insert(good.id().clone(), good);
    workflows.insert(lacking.id().clone(), lacking);

    let read = fleet.lifecycle_ends_checked(&manifest, Catalogued { workflows, ..Catalogued::default() });
    assert_eq!(read.workflows.keys().map(|id| id.as_str().to_string()).collect::<Vec<_>>(), ["rail"], "the one that fits stands");
    let [left] = read.left_out.as_slice() else { panic!("{:?}", read.left_out) };
    assert!(left.said.starts_with("`lacking` was left out, because "), "{}", left.said);
    assert!(left.said.contains("no lifecycle here is called `pull`"), "{}", left.said);
}

#[tokio::test]
async fn a_definition_about_to_be_saved_is_refused_for_a_lifecycle_or_state_the_repository_lacks() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
    let why = |ends: &str| match fleet.lifecycle_ends_unnamed(&manifest, &reviewing(ends)) {
        Err(WorkflowNotSaved::Unfit { why }) => Some(why),
        Err(_) => panic!("refused another way"),
        Ok(()) => None,
    };
    assert_eq!(why("pr: approved"), None);
    assert_eq!(why("any ticket: closed"), None);
    assert!(why("pull: approved").is_some_and(|said| said.contains("no lifecycle here is called `pull`")));
    assert!(why("pr: accepted").is_some_and(|said| said.contains("the pr lifecycle has no state `accepted`")));
    assert!(fleet.lifecycle_ends_unnamed(&manifest, "steps: [").is_ok(), "one that will not parse is the loader's to refuse");
}
