//! The files that draw each screen, so a note made on one knows its source: Fleet maps them to the features whose Jobs
//! the note goes to (`docs/concepts/feature-catalog.md`, *A note on a screen*). **The one place this is kept**: a screen
//! moved to another module moves its row here.
//!
//! A whole screen's files name the screen, and many of them no feature owns: the open Job's view is drawn by files of a
//! dozen features. So what the screen shows is read too: the Job open on it and the features that Job writes to, or the
//! feature whose page is open.

use gpui::App;
use serde_json::{json, Value};

use crate::history::Place;
use crate::operator::Front;

const ROOT: &str = "crates/bridge-native/src";

/// The modules under `crates/bridge-native/src` that draw the place he is on: Settings over everything, else the Job
/// open on the Jobs screen, else the place in front.
fn modules_of(place: &Place) -> &'static [&'static str] {
    if place.settings {
        return &["settings"];
    }
    match place.front {
        Front::Jobs | Front::Cockpit if place.job.is_some() => &["job_view"],
        Front::Jobs => &["jobs.rs", "jobs"],
        Front::Operator => &["operator"],
        Front::Cockpit => &["home", "operator", "jobs.rs", "jobs"],
        Front::Reviews => &["reviews"],
        Front::Kit => &["kit"],
        Front::Features => &["project_board", "features"],
        Front::Usage => &["usage_page"],
        // A screen added since names no modules until it is listed here.
        #[allow(unreachable_patterns)]
        _ => &[],
    }
}

/// The modules that draw a screen known by its name alone, where no window keeps a history: a walk's, or a test's.
fn modules_named(screen: &str) -> &'static [&'static str] {
    match screen {
        "Jobs" => &["jobs.rs", "jobs"],
        "Prototype" => &["prototype"],
        _ => &[],
    }
}

/// The files that draw where he is, from the repository root; `screen` is the layer's own name for it. Only the main
/// window's layer, on the Jobs screen, reads the place its window's history keeps.
pub fn paths_now(screen: Option<&str>, cx: &App) -> Vec<String> {
    let modules = match (screen, crate::history::now(cx)) {
        (Some("Jobs"), Some(mut place)) => {
            // The history keeps a Job only on the Jobs screen; one open over the Cockpit's page is drawn as it is there.
            if place.front == Front::Cockpit && !place.settings {
                place.job = crate::history::screen(cx).and_then(|screen| screen.read(cx).job_shown(cx));
            }
            modules_of(&place)
        }
        (screen, _) => screen.map_or(&[][..], modules_named),
    };
    modules.iter().map(|module| format!("{ROOT}/{module}")).collect()
}

/// What the main window shows, for a note made on it: the Job open there, and the features it names outright, the open
/// Job's or the open feature page's, each as `{project, slug, title, area}`. Nothing under Settings, or on another layer.
pub fn shown_now(screen: Option<&str>, cx: &App) -> (Option<String>, Vec<Value>) {
    let Some(at) = (screen == Some("Jobs")).then(|| crate::history::screen(cx)).flatten() else { return (None, Vec::new()) };
    if crate::history::now(cx).is_some_and(|place| place.settings) {
        return (None, Vec::new());
    }
    let at = at.read(cx);
    let named = |project: &str, slug: &str, title: &str, area: &str| json!({ "project": project, "slug": slug, "title": title, "area": area });
    if let Some(job) = at.job_shown(cx) {
        let list = at.list(cx);
        let record = list.read(cx).records().iter().find(|record| record.id.as_str() == job.as_ref());
        let features = record.into_iter().flat_map(|record| &record.features).map(|one| named(&one.project, &one.slug, &one.title, &one.area_title)).collect();
        return (Some(job.to_string()), features);
    }
    let features = at.feature_shown(cx).map(|one| named(&one.project, &one.slug, &one.title, &one.area)).into_iter().collect();
    (None, features)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_over_a_place_is_drawn_by_settings_and_an_open_job_by_the_job_view() {
        let at = |front, job: Option<&str>, settings| Place { front, job: job.map(|job| job.to_string().into()), settings };
        assert_eq!(modules_of(&at(Front::Jobs, Some("s-1"), true)), ["settings"]);
        assert_eq!(modules_of(&at(Front::Jobs, Some("s-1"), false)), ["job_view"]);
        assert_eq!(modules_of(&at(Front::Cockpit, Some("s-1"), false)), ["job_view"], "a Job open over the Cockpit's page");
        assert_eq!(modules_of(&at(Front::Jobs, None, false)), ["jobs.rs", "jobs"]);
        assert_eq!(modules_of(&at(Front::Features, None, false)), ["project_board", "features"]);
    }

    #[test]
    fn every_module_named_is_in_the_tree() {
        let fronts = [Front::Jobs, Front::Operator, Front::Cockpit, Front::Reviews, Front::Kit, Front::Features, Front::Usage];
        let places = fronts.iter().map(|front| Place { front: *front, job: None, settings: false }).chain([
            Place { front: Front::Jobs, job: Some("s".into()), settings: false },
            Place { front: Front::Jobs, job: None, settings: true },
        ]);
        let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for module in places.flat_map(|place| modules_of(&place).to_vec()).chain(modules_named("Prototype").iter().copied()) {
            assert!(here.join(module).exists(), "{module} is not under crates/bridge-native/src");
        }
    }
}
