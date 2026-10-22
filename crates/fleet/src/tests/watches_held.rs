//! Watches as this build holds them: a start checked against its recipe and the limits and kept
//! for the Job, a weighing held to what was reported, and a stop. Nothing runs a command yet.

use std::path::PathBuf;
use std::sync::Arc;

use api::{Refusal, Watches};
use ipc::{SessionId, StartWatch, StopWatch, WatchStarts, WatchState, WatchVerdict, WeighWatch};

use super::repositories::Planted;
use super::session_host::{rig_fitted, rig_placing, Rig};

fn code(refusal: &Refusal) -> (&str, u16) {
    (refusal.error().code.as_str(), refusal.status())
}

fn own(job: &SessionId, every: &str, gives_up_after: Option<&str>) -> StartWatch {
    StartWatch {
        session_id: Some(job.clone()),
        name: "QA deploy".to_string(),
        starts: WatchStarts::Command {
            command: vec!["scripts/deploy-state".to_string(), "abc123".to_string()],
            every: every.to_string(),
            gives_up_after: gives_up_after.map(str::to_string),
        },
        follows: Some(42),
        set_aside: Vec::new(),
    }
}

fn recipe(job: &SessionId, named: &str, values: &[(&str, &str)]) -> StartWatch {
    StartWatch {
        session_id: Some(job.clone()),
        name: "QA deploy".to_string(),
        starts: WatchStarts::Recipe {
            recipe: named.to_string(),
            values: values.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect(),
        },
        follows: None,
        set_aside: Vec::new(),
    }
}

#[tokio::test]
async fn a_start_is_kept_for_its_job_with_the_machines_give_up() {
    let rig = rig_placing(None);
    let job = rig.start().await;
    let started = rig.fleet.start_watch(None, own(&job, "60s", None)).await.expect("started");
    assert_eq!((started.state, started.live), (WatchState::Waiting, true));
    assert_eq!(started.gives_up_after_seconds, 120 * 60, "timeouts.watchGiveUpMinutes ships at 120");
    assert_eq!(started.follows, Some(42));

    let listed = rig.fleet.list_watches(None, Some(job.clone())).await.expect("listed");
    assert_eq!(listed.watches, vec![started]);
    let other = rig.fleet.list_watches(None, Some(SessionId::carried("another"))).await.expect("listed");
    assert!(other.watches.is_empty());
    let unplaced = rig.fleet.list_watches(None, None).await.expect_err("nothing places a Job");
    assert_eq!(code(&unplaced), ("fleet.watches_unplaced", 422));
}

#[tokio::test]
async fn the_limits_refuse_a_start_and_say_which() {
    let rig = rig_placing(None);
    let job = rig.start().await;
    let refused = |every: &'static str, gives_up: Option<&'static str>| own(&job, every, gives_up);
    let too_often = rig.fleet.start_watch(None, refused("10s", None)).await.expect_err("under the floor");
    assert_eq!(code(&too_often), ("fleet.watch_too_often", 422));
    let too_late = rig.fleet.start_watch(None, refused("60s", Some("25h"))).await.expect_err("past a day");
    assert_eq!(code(&too_late), ("fleet.watch_gives_up_too_late", 422));
    let not_a_span = rig.fleet.start_watch(None, refused("often", None)).await.expect_err("no span");
    assert_eq!(code(&not_a_span), ("fleet.watch_refused", 422));

    for _ in 0..5 {
        rig.fleet.start_watch(None, own(&job, "60s", Some("24h"))).await.expect("within the limit");
    }
    let sixth = rig.fleet.start_watch(None, own(&job, "60s", None)).await.expect_err("past watchesPerJob");
    assert_eq!(code(&sixth), ("fleet.watch_limit_reached", 409));
}

fn rig_offering(recipes: Vec<config::WatchRecipeWritten>) -> Rig {
    let planted = Arc::new(Planted::nothing());
    planted.will_offer_recipes(recipes);
    rig_fitted(move |fittings| fittings.locating = planted)
}

#[tokio::test]
async fn a_recipe_fills_whole_arguments_and_the_machines_copy_wins() {
    let rig = rig_offering(vec![
        config::WatchRecipeWritten::in_repository(
            PathBuf::from(".armada/watches/qa.yml"),
            "name: qa-deploy\ncommand: [scripts/deploy-state, \"{sha}\"]\nevery: 60s\n".to_string(),
        ),
        config::WatchRecipeWritten::on_machine(
            PathBuf::from("/home/me/.armada/machine/watches/qa.yml"),
            "name: qa-deploy\ncommand: [/home/me/bin/deploy-state, \"{sha}\"]\nevery: 2m\ngives_up_after: 1h\n".to_string(),
        ),
    ]);
    let job = rig.start().await;
    let started = rig
        .fleet
        .start_watch(None, recipe(&job, "qa-deploy", &[("sha", "abc; rm -rf /")]))
        .await
        .expect("started from the recipe");
    assert_eq!(started.command, ["/home/me/bin/deploy-state", "abc; rm -rf /"]);
    assert_eq!((started.every_seconds, started.gives_up_after_seconds), (120, 3600));
    assert_eq!(started.recipe.as_deref(), Some("qa-deploy"));

    let unknown = rig.fleet.start_watch(None, recipe(&job, "ci", &[])).await.expect_err("no such recipe");
    assert_eq!(code(&unknown), ("fleet.watch_recipe_unknown", 422));
    assert!(unknown.error().message.contains("qa-deploy"), "{}", unknown.error().message);
    let missing = rig.fleet.start_watch(None, recipe(&job, "qa-deploy", &[])).await.expect_err("no sha");
    assert_eq!(code(&missing), ("fleet.watch_value_refused", 422));
}

#[tokio::test]
async fn a_weighing_and_a_stop_act_on_the_jobs_own_watch() {
    let rig = rig_placing(None);
    let job = rig.start().await;
    let id = rig.fleet.start_watch(None, own(&job, "60s", None)).await.expect("started").id;
    let weigh = |watch: &str, verdict: WatchVerdict| WeighWatch { session_id: Some(job.clone()), watch: watch.to_string(), verdict };

    let ours = rig.fleet.weigh_watch(None, weigh(&id, WatchVerdict::Ours)).await.expect_err("nothing failed");
    assert_eq!(code(&ours), ("fleet.watch_not_failed", 409));
    let not_ours = WatchVerdict::NotOurs { sentence: "Another service.".to_string(), parts: vec!["unit".to_string()] };
    let unreported = rig.fleet.weigh_watch(None, weigh(&id, not_ours)).await.expect_err("no reading carried it");
    assert_eq!(code(&unreported), ("fleet.watch_part_unreported", 422));

    let elsewhere = StopWatch { session_id: Some(SessionId::carried("another")), watch: id.clone() };
    let refused = rig.fleet.stop_watch(None, elsewhere).await.expect_err("another Job's watch");
    assert_eq!(code(&refused), ("fleet.no_such_watch", 404));

    let stopped = rig.fleet.stop_watch(None, StopWatch { session_id: None, watch: id.clone() }).await.expect("Bridge stops any");
    assert_eq!((stopped.state, stopped.live), (WatchState::Stopped, false));
    let again = rig.fleet.stop_watch(None, StopWatch { session_id: None, watch: id.clone() }).await.expect("a second Stop");
    assert_eq!(again.state, WatchState::Stopped);
    let ended = rig.fleet.weigh_watch(None, weigh(&id, WatchVerdict::Ours)).await.expect_err("it ended");
    assert_eq!(code(&ended), ("fleet.watch_ended", 409));
}
