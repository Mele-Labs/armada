//! What each live holder has written, which pairs are waiting to be checked, and what the last
//! check of each found. **Memory only, and no I/O**: every method answers from the table.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::time::Duration;

use core_model::Timestamp;

use crate::clock::elapsed;

/// The least time between a write that puts two holders on one file and the check of them, and
/// between two checks of one pair.
pub(crate) const DEBOUNCE: Duration = Duration::from_secs(30);

/// Something that writes in a repository: a Session.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Who {
    Session(String),
}

/// What a holder's changes are set against.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Side {
    Holder(Who),
    Main,
}

/// One clash, or the check of one: a holder against another holder or main. **Two holders are in
/// id order**, so either side finds the same key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Opposed(pub Who, pub Side);

impl Opposed {
    pub(crate) fn pair(one: &Who, other: &Who) -> Opposed {
        let (first, second) = if one <= other { (one, other) } else { (other, one) };
        Opposed(first.clone(), Side::Holder(second.clone()))
    }

    pub(crate) fn involves(&self, who: &Who) -> bool {
        self.0 == *who || self.1 == Side::Holder(who.clone())
    }
}

/// What the last check found, kept while it conflicts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reading {
    pub paths: BTreeSet<String>,
    /// When this pair first conflicted. A change in the files keeps it.
    pub since: Timestamp,
    /// For a clash with main, the commit there that changed the first file.
    pub landed: Option<String>,
    /// Who has been told this set of files. Cleared when the set changes.
    pub told: BTreeSet<Who>,
}

/// What a check's verdict did to the reading.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Changed {
    Nothing,
    /// A new set of files, to be told.
    Set(BTreeSet<String>),
    /// It conflicts no longer.
    Cleared,
}

struct Held {
    manifest: String,
    paths: BTreeSet<String>,
    /// Worktrees it was credited a write in that its standing may not name: a terminal Job at the
    /// repository's root writes in a slot or worktree it never stood in.
    worktrees: BTreeSet<String>,
}

#[derive(Default)]
struct State {
    by_path: BTreeMap<String, BTreeMap<String, BTreeSet<Who>>>,
    by_holder: BTreeMap<Who, Held>,
    due: BTreeMap<Opposed, Timestamp>,
    readings: BTreeMap<Opposed, Reading>,
    /// Where main stood when last looked at, and where it has moved to since, by repository.
    main_tip: BTreeMap<String, String>,
    main_moved: BTreeMap<String, String>,
    /// Whether the table was filled from the worktrees once since Fleet started.
    seeded: bool,
}

/// The table. Shared with the taps that read a Drone's stream, which may not await.
#[derive(Default)]
pub(crate) struct Writes(Mutex<State>);

impl Writes {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().expect("held across no panic")
    }

    /// `who` wrote these paths. Every other holder that has written one of the files `who` has
    /// written is due a check against it, and so is `who` against main if it conflicts there.
    pub(crate) fn wrote(
        &self,
        who: &Who,
        manifest: &str,
        paths: impl IntoIterator<Item = String>,
        now: &Timestamp,
    ) {
        let mut state = self.state();
        let state = &mut *state;
        let held = state.by_holder.entry(who.clone()).or_insert_with(|| Held {
            manifest: manifest.to_string(),
            paths: BTreeSet::new(),
            worktrees: BTreeSet::new(),
        });
        let in_manifest = state.by_path.entry(manifest.to_string()).or_default();
        for path in paths {
            held.paths.insert(path.clone());
            in_manifest.entry(path).or_default().insert(who.clone());
        }
        let mut others = BTreeSet::new();
        for path in &held.paths {
            others.extend(in_manifest[path].iter().filter(|other| *other != who).cloned());
        }
        for other in others {
            state.due.entry(Opposed::pair(who, &other)).or_insert_with(|| now.clone());
        }
        let against_main = Opposed(who.clone(), Side::Main);
        if state.readings.contains_key(&against_main) {
            state.due.entry(against_main).or_insert_with(|| now.clone());
        }
    }

    /// `who` wrote these paths, each named from the root of the worktree at `worktree`.
    pub(crate) fn wrote_in(&self, who: &Who, manifest: &str, worktree: &str, paths: Vec<String>, now: &Timestamp) {
        self.wrote(who, manifest, paths, now);
        if let Some(held) = self.state().by_holder.get_mut(who) {
            held.worktrees.insert(worktree.to_string());
        }
    }

    /// The worktrees `who` was credited a write in by [`Writes::wrote_in`].
    pub(crate) fn worktrees_of(&self, who: &Who) -> Vec<String> {
        self.state().by_holder.get(who).map(|held| held.worktrees.iter().cloned().collect()).unwrap_or_default()
    }

    /// `who` is over: gone from the table, and every clash it was in with it.
    pub(crate) fn left(&self, who: &Who) {
        let mut state = self.state();
        if let Some(held) = state.by_holder.remove(who) {
            if let Some(in_manifest) = state.by_path.get_mut(&held.manifest) {
                for path in &held.paths {
                    if let Some(holders) = in_manifest.get_mut(path) {
                        holders.remove(who);
                        if holders.is_empty() {
                            in_manifest.remove(path);
                        }
                    }
                }
            }
        }
        state.due.retain(|opposed, _| !opposed.involves(who));
        state.readings.retain(|opposed, _| !opposed.involves(who));
    }

    /// The checks that have waited out [`DEBOUNCE`].
    pub(crate) fn due(&self, now: &Timestamp) -> Vec<Opposed> {
        self.state()
            .due
            .iter()
            .filter(|(_, since)| elapsed(since, now) >= DEBOUNCE)
            .map(|(opposed, _)| opposed.clone())
            .collect()
    }

    /// This check is being run; a write from here on schedules the next.
    pub(crate) fn taken(&self, opposed: &Opposed) {
        self.state().due.remove(opposed);
    }

    /// What a check found. `None` is a pair that does not conflict.
    pub(crate) fn found(
        &self,
        opposed: &Opposed,
        paths: Option<BTreeSet<String>>,
        landed: Option<String>,
        now: &Timestamp,
    ) -> Changed {
        let mut state = self.state();
        let paths = paths.filter(|paths| !paths.is_empty());
        let kept = state.readings.get(opposed).map(|reading| reading.paths.clone());
        match (paths, kept) {
            (None, None) => Changed::Nothing,
            (None, Some(_)) => {
                state.readings.remove(opposed);
                Changed::Cleared
            }
            (Some(paths), Some(kept)) if paths == kept => Changed::Nothing,
            (Some(paths), Some(_)) => {
                if let Some(reading) = state.readings.get_mut(opposed) {
                    reading.paths = paths.clone();
                    reading.landed = landed;
                    reading.told.clear();
                }
                Changed::Set(paths)
            }
            (Some(paths), None) => {
                let reading = Reading {
                    paths: paths.clone(),
                    since: now.clone(),
                    landed,
                    told: BTreeSet::new(),
                };
                state.readings.insert(opposed.clone(), reading);
                Changed::Set(paths)
            }
        }
    }

    /// `who` has been told the files of this clash as they stand.
    pub(crate) fn told(&self, opposed: &Opposed, who: &Who) {
        if let Some(reading) = self.state().readings.get_mut(opposed) {
            reading.told.insert(who.clone());
        }
    }

    /// Every clash and who in it has not heard its files yet.
    pub(crate) fn untold(&self) -> Vec<(Opposed, Reading)> {
        self.state()
            .readings
            .iter()
            .filter(|(opposed, reading)| {
                let mut sides = vec![opposed.0.clone()];
                if let Side::Holder(other) = &opposed.1 {
                    sides.push(other.clone());
                }
                sides.iter().any(|who| !reading.told.contains(who))
            })
            .map(|(opposed, reading)| (opposed.clone(), reading.clone()))
            .collect()
    }

    pub(crate) fn reading(&self, opposed: &Opposed) -> Option<Reading> {
        self.state().readings.get(opposed).cloned()
    }

    /// Every standing clash of one repository's holders.
    pub(crate) fn readings_in(&self, manifest: &str) -> Vec<(Opposed, Reading)> {
        let state = self.state();
        state
            .readings
            .iter()
            .filter(|(opposed, _)| {
                state.by_holder.get(&opposed.0).is_some_and(|held| held.manifest == manifest)
            })
            .map(|(opposed, reading)| (opposed.clone(), reading.clone()))
            .collect()
    }

    /// Every holder, with its repository and what it has written.
    pub(crate) fn holders(&self) -> Vec<(Who, String, BTreeSet<String>)> {
        self.state()
            .by_holder
            .iter()
            .map(|(who, held)| (who.clone(), held.manifest.clone(), held.paths.clone()))
            .collect()
    }

    /// What `who` has written, repository-relative. Empty for a holder the table does not hold.
    pub(crate) fn written_by(&self, who: &Who) -> BTreeSet<String> {
        self.state().by_holder.get(who).map(|held| held.paths.clone()).unwrap_or_default()
    }

    /// Which repository `who` writes in.
    pub(crate) fn manifest_of(&self, who: &Who) -> Option<String> {
        self.state().by_holder.get(who).map(|held| held.manifest.clone())
    }

    /// Nothing is held, nothing moved and nothing waits to be sent.
    pub(crate) fn idle(&self) -> bool {
        let state = self.state();
        state.by_holder.is_empty() && state.main_moved.is_empty()
    }

    /// Main's tip in this repository is `tip`. **True where it moved** from one seen before; the
    /// first sight only sets what later moves are measured from.
    pub(crate) fn main_seen(&self, manifest: &str, tip: &str) -> bool {
        let mut state = self.state();
        let before = state.main_tip.insert(manifest.to_string(), tip.to_string());
        let moved = before.is_some_and(|before| before != tip);
        if moved {
            state.main_moved.insert(manifest.to_string(), tip.to_string());
        }
        moved
    }

    /// Moves of main nothing has acted on yet.
    pub(crate) fn main_moves(&self) -> Vec<(String, String)> {
        std::mem::take(&mut self.state().main_moved).into_iter().collect()
    }

    /// Main's tip as last seen.
    pub(crate) fn main_tip(&self, manifest: &str) -> Option<String> {
        self.state().main_tip.get(manifest).cloned()
    }

    /// True the first time it is asked, when the table is to be filled from what git holds.
    pub(crate) fn first_pass(&self) -> bool {
        !std::mem::replace(&mut self.state().seeded, true)
    }
}
