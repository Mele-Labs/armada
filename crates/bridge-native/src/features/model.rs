//! What the Features list draws, worked out from the library Fleet read: the areas in their order, each
//! with its features, who is on each now and the merge a written section is stale since. Nothing here
//! touches the screen or the wire.

use gpui::{px, Pixels};
use ipc::{FeatureLibrary, FeatureRow};

/// Under this width of the screen the list column gives up its floor, so the page beside it keeps room to read.
pub const NARROW_BELOW: f32 = 1000.;

/// The list column's width on a narrow screen, or None where the column keeps its share and floor, which is from
/// `NARROW_BELOW` up. It takes what the page leaves it, `page_min` less than the screen, but never more than the
/// share and floor would give it and never less than `need`, what its fixed columns and a few letters of a title ask for.
/// Where `need` and `page_min` together are more than the screen, the list's need wins and the page is what gives.
pub fn narrow_list_width(screen: Pixels, floor: Pixels, need: Pixels, page_min: Pixels) -> Option<Pixels> {
    if screen >= px(NARROW_BELOW) {
        return None;
    }
    let old = (screen * 0.36).max(floor);
    Some((screen - page_min).min(old).max(need))
}

/// One feature as a row: Fleet's row, and the chips it wears.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    pub row: FeatureRow,
    /// The Jobs on it now, by title, in Fleet's order.
    pub on: Vec<String>,
    /// `stale since #N`, where a written section is stale.
    pub stale: Option<String>,
}

impl Shown {
    fn of(row: &FeatureRow) -> Shown {
        Shown {
            on: row.jobs_now.iter().map(|job| job.title.clone()).collect(),
            stale: row.stale_since.map(|since| format!("stale since #{since}")),
            row: row.clone(),
        }
    }

    /// Every Job on it, one to a line: what the column's tooltip says where the Jobs do not all fit.
    pub fn on_said(&self) -> String {
        self.on.join("\n")
    }
}

/// A line of the list: an area's head, or a feature by its place in [`Listed::shown`].
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    Head { slug: String, title: String },
    Feature(usize),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Listed {
    pub lines: Vec<Line>,
    /// The features in the order they are drawn, for the keys to move through.
    pub shown: Vec<Shown>,
}

impl Listed {
    /// The line a feature is drawn on.
    pub fn line_of(&self, at: usize) -> Option<usize> {
        self.lines.iter().position(|line| *line == Line::Feature(at))
    }

    pub fn find(&self, slug: &str) -> Option<usize> {
        self.shown.iter().position(|one| one.row.slug == slug)
    }
}

/// The areas as lines, each area's head before its features. A filter keeps the features whose title,
/// slug or area it names, and drops an area with none left; with no filter an area Fleet keeps stays.
pub fn listed(library: &FeatureLibrary, query: &str) -> Listed {
    let query = query.trim().to_lowercase();
    let mut out = Listed::default();
    for area in &library.areas {
        let area_matches = !query.is_empty() && area.title.to_lowercase().contains(&query);
        let kept: Vec<&FeatureRow> = area
            .features
            .iter()
            .filter(|row| query.is_empty() || area_matches || row.title.to_lowercase().contains(&query) || row.slug.to_lowercase().contains(&query))
            .collect();
        if !query.is_empty() && kept.is_empty() {
            continue;
        }
        out.lines.push(Line::Head { slug: area.slug.clone(), title: area.title.clone() });
        for row in kept {
            out.lines.push(Line::Feature(out.shown.len()));
            out.shown.push(Shown::of(row));
        }
    }
    out
}

/// Whether a library holds a feature, which is what makes it the one a window with no project picked shows.
pub fn holds_features(library: &FeatureLibrary) -> bool {
    library.areas.iter().any(|area| !area.features.is_empty())
}
