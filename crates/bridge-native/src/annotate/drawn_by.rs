//! The files that draw each screen, so a note made on one knows its source: Fleet maps them to the features whose Jobs
//! the note goes to (`docs/concepts/feature-catalog.md`, *A note on a screen*). **The one place this is kept**: a screen
//! moved to another module moves its row here.

use gpui::App;

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
        Front::Jobs if place.job.is_some() => &["job_view"],
        Front::Jobs => &["jobs.rs", "jobs"],
        Front::Operator => &["operator"],
        Front::Cockpit => &["home", "operator", "jobs.rs", "jobs"],
        Front::Reviews => &["reviews"],
        Front::Kit => &["kit"],
        Front::Features => &["project_board", "features"],
        Front::Usage => &["usage_page"],
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
        (Some("Jobs"), Some(place)) => modules_of(&place),
        (screen, _) => screen.map_or(&[][..], modules_named),
    };
    modules.iter().map(|module| format!("{ROOT}/{module}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_over_a_place_is_drawn_by_settings_and_an_open_job_by_the_job_view() {
        let at = |front, job: Option<&str>, settings| Place { front, job: job.map(|job| job.to_string().into()), settings };
        assert_eq!(modules_of(&at(Front::Jobs, Some("s-1"), true)), ["settings"]);
        assert_eq!(modules_of(&at(Front::Jobs, Some("s-1"), false)), ["job_view"]);
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
