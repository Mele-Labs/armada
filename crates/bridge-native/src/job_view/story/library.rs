//! The names a Story tab takes from its project's feature library, read once for every tab of the project. `list_features`
//! reads the whole library, far more than a story needs, so the story draws as soon as it is read and a Filed under row
//! names its feature by its slug until the titles land.
//!
//! **Read again** when the connection comes back on another port, when the project's librarian moves a step, and once
//! for a slug a story names that the names lack (a feature filed since they were read). A read asked while one is out is
//! made once, after it.

use std::collections::{BTreeSet, HashMap};

use gpui::{App, Global, Task};

use super::model::Features;
use crate::settings::wire::Refused;

/// Each feature's title by slug, and the slugs of the areas in the library's order, which give a feature its hue.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Names {
    pub(crate) features: Features,
    pub(crate) areas: Vec<String>,
}

impl Names {
    pub(crate) fn of(library: &ipc::FeatureLibrary) -> Names {
        Names {
            features: library.areas.iter().flat_map(|area| &area.features).map(|row| (row.slug.clone(), row.title.clone())).collect(),
            areas: library.areas.iter().map(|area| area.slug.clone()).collect(),
        }
    }
}

#[derive(Default)]
struct Held {
    /// The port the names were last asked on.
    port: Option<u16>,
    names: Option<Names>,
    /// Counts the reads that landed, so a tab knows whether it holds the latest.
    landed: u64,
    reading: bool,
    /// A read was asked while one was out.
    again: bool,
    /// The slugs a story named that the names lacked, each asked for once.
    missed: BTreeSet<String>,
    work: Option<Task<()>>,
}

/// The names of each project read, by Fleet's id of the project.
#[derive(Default)]
pub(crate) struct Libraries(HashMap<String, Held>);

impl Global for Libraries {}

/// Whether a read of `held` is due, asked on `port` by a story that names `named`. A read out marks one to follow it
/// where one would be due.
fn due<'a>(held: &mut Held, port: u16, named: impl IntoIterator<Item = &'a str>) -> bool {
    let lacking: Vec<String> = match &held.names {
        Some(names) => named.into_iter().filter(|slug| !names.features.contains_key(*slug) && !held.missed.contains(*slug)).map(str::to_string).collect(),
        None => Vec::new(),
    };
    held.missed.extend(lacking.iter().cloned());
    let wanted = held.port != Some(port) || held.names.is_none() || !lacking.is_empty();
    if held.reading {
        held.again |= wanted;
        return false;
    }
    wanted
}

/// The names of `project` as last read, with the count of reads that landed.
pub(crate) fn names<'a>(project: &str, cx: &'a App) -> Option<(u64, &'a Names)> {
    let held = cx.try_global::<Libraries>()?.0.get(project)?;
    held.names.as_ref().map(|names| (held.landed, names))
}

/// A Story tab of `project` on `port` wants its names, for a story that names `named`: they are read where they are due.
pub(crate) fn want<'a>(project: &str, port: u16, named: impl IntoIterator<Item = &'a str>, cx: &mut App) {
    let held = cx.default_global::<Libraries>().0.entry(project.to_string()).or_default();
    if due(held, port, named) {
        read(project, port, cx);
    }
}

/// The project's librarian moved a step: its names are read again, once, after a read that is out.
pub(crate) fn librarian_moved(project: &str, port: u16, cx: &mut App) {
    let Some(held) = cx.try_global::<Libraries>().and_then(|all| all.0.get(project)) else { return };
    if held.reading {
        cx.default_global::<Libraries>().0.entry(project.to_string()).or_default().again = true;
    } else {
        read(project, port, cx);
    }
}

fn read(project: &str, port: u16, cx: &mut App) {
    let asked = crate::features::wire::library(port, project, cx);
    let named = project.to_string();
    let work = cx.spawn(async move |cx| {
        let answer = asked.await;
        cx.update(|cx| landed(&named, port, answer, cx));
    });
    let held = cx.default_global::<Libraries>().0.entry(project.to_string()).or_default();
    held.port = Some(port);
    held.reading = true;
    held.again = false;
    held.work = Some(work);
}

/// A read of `project` answered. A failed one keeps the names read before; a read asked meanwhile goes out now.
fn landed(project: &str, port: u16, answer: Result<ipc::FeatureLibrary, Refused>, cx: &mut App) {
    let held = cx.default_global::<Libraries>().0.entry(project.to_string()).or_default();
    held.reading = false;
    if let Ok(library) = answer {
        held.names = Some(Names::of(&library));
        held.landed += 1;
    }
    if std::mem::take(&mut held.again) {
        read(project, port, cx);
    }
}

#[cfg(test)]
pub(crate) fn landed_in_tests(project: &str, port: u16, library: ipc::FeatureLibrary, cx: &mut App) {
    landed(project, port, Ok(library), cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(slugs: &[&str]) -> Names {
        Names { features: slugs.iter().map(|slug| (slug.to_string(), slug.to_uppercase())).collect(), areas: Vec::new() }
    }

    #[test]
    fn a_library_is_read_once_for_every_tab_on_the_same_port_and_again_on_another() {
        let mut held = Held::default();
        assert!(due(&mut held, 1, []), "nothing read yet");
        held.port = Some(1);
        held.reading = true;
        assert!(!due(&mut held, 1, []), "a second tab waits on the read that is out");
        held.reading = false;
        held.names = Some(names(&["story"]));
        assert!(!due(&mut held, 1, ["story"]), "read already");
        assert!(due(&mut held, 2, ["story"]), "the connection came back on another port");
    }

    #[test]
    fn a_slug_the_names_lack_is_read_for_once() {
        let mut held = Held { port: Some(1), names: Some(names(&["story"])), ..Held::default() };
        assert!(due(&mut held, 1, ["story", "new-one"]));
        held.names = Some(names(&["story"]));
        assert!(!due(&mut held, 1, ["new-one"]), "a slug still missing after its read is not read for again");
    }

    #[test]
    fn a_read_wanted_while_one_is_out_is_marked_to_follow_it() {
        let mut held = Held { port: Some(1), names: Some(names(&["story"])), reading: true, ..Held::default() };
        assert!(!due(&mut held, 1, ["story"]));
        assert!(!held.again, "nothing new was wanted");
        assert!(!due(&mut held, 1, ["new-one"]));
        assert!(held.again, "the slug is read for after the read that is out");
    }
}
