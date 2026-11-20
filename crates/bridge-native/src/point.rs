//! Mod and Report on every menu opened at a point, after the menu's own entries, as Electron's
//! `point-acts.tsx` adds them. What was pressed is the labelled element under the press, read as
//! the menu opens: Mod starts a Mod Job told what it is, Report begins a note on it.

use gpui::{App, Pixels, Point};

use crate::dropdown::Entry;
use crate::labels::Drawn;

pub const MOD: &str = "point-mod";
pub const REPORT: &str = "point-report";

/// The two acts, set apart by a rule. Report is in every build, as the annotation layer is.
pub fn entries() -> Vec<Entry> {
    vec![
        Entry::Rule,
        Entry::Act { id: MOD, label: "Mod".into(), icon: Some("puzzle"), danger: false },
        Entry::Act { id: REPORT, label: "Report".into(), icon: Some("flag"), danger: false },
    ]
}

/// Run a point act on what was pressed at `at`.
pub fn run(id: &str, at: Point<Pixels>, pressed: Option<Drawn>, cx: &mut App) {
    match id {
        MOD => {
            let Some(pressed) = pressed else { return };
            crate::dispatch::screen::start_mod(&pressed, cx);
        }
        REPORT => crate::dispatch::screen::report_at(at, cx),
        _ => {}
    }
}

/// The name a Mod Job takes: the element's words when they are short (`mod-job.ts` `nameOf`).
pub fn name_of(pressed: &Drawn) -> String {
    const NAME_WIDTH: usize = 40;
    let said = pressed.name.trim();
    if !said.is_empty() && said.chars().count() <= NAME_WIDTH {
        return said.to_string();
    }
    crate::capture::role_word(pressed.role).unwrap_or("element").to_string()
}
