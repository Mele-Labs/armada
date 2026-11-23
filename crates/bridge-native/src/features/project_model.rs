//! What the project's pages say, worked out from Fleet's read: the code map's tiles and the key to their hues, the busiest
//! features, the glossary, and the rows of Since you looked in the order Fleet served them. Nothing here touches the screen or the wire.

use chrono::{DateTime, Duration, Local, Utc};
use ipc::{LookedTag, MapRoom, ProjectOverview, SinceYouLooked};

use crate::look::{token, Token};

/// What the glossary card says while Fleet has no term to give it.
pub const GLOSSARY_EMPTY: &str = "Terms appear as Jobs add them at their end";

/// What the busiest card says while no feature has merged lately.
pub const BUSIEST_EMPTY: &str = "No feature has merged lately.";

/// What Who's on what says under its title.
pub const WHO_SUB: &str = "Open pull requests, mapped to features";

/// The pages of the project that sit above the areas in the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectPage {
    Overview,
    Since,
    Who,
}

impl ProjectPage {
    pub const ALL: [ProjectPage; 3] = [ProjectPage::Overview, ProjectPage::Since, ProjectPage::Who];

    /// Where the row sits among the places the keys step through: before the first feature, which is place 0.
    pub fn place(self) -> isize {
        match self {
            ProjectPage::Overview => -3,
            ProjectPage::Since => -2,
            ProjectPage::Who => -1,
        }
    }

    /// The page at `place`, where it is one of the rows over the features.
    pub fn at(place: isize) -> Option<ProjectPage> {
        ProjectPage::ALL.into_iter().find(|page| page.place() == place)
    }

    /// What the row's element id is made of.
    pub fn slug(self) -> &'static str {
        match self {
            ProjectPage::Overview => "overview",
            ProjectPage::Since => "since-you-looked",
            ProjectPage::Who => "whos-on-what",
        }
    }

    /// The row's words and the page's title.
    pub fn title(self) -> &'static str {
        match self {
            ProjectPage::Overview => "Overview",
            ProjectPage::Since => "Since you looked",
            ProjectPage::Who => "Who's on what",
        }
    }
}

/// An area of the library as the list draws it: the hue the list gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct AreaHue {
    pub slug: String,
    pub title: String,
    pub hue: Token,
}

/// One room of the code map.
#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    pub module: String,
    pub path: String,
    /// The hue of the area that owns most of its files, where one does.
    pub hue: Option<Token>,
    /// What a tooltip and a screen reader say of it.
    pub said: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Busy {
    pub slug: String,
    pub title: String,
    /// `#3862 #3863`, newest first as Fleet served them.
    pub pulls: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Glossary {
    /// The line the card carries while it has no term.
    Empty(&'static str),
    Terms(Vec<(String, String)>),
}

/// The Overview page.
#[derive(Clone, Debug, PartialEq)]
pub struct OverviewModel {
    /// The README's sentence and the file it came from. None where the repository has no README.
    pub lede: Option<(String, String)>,
    pub tiles: Vec<Tile>,
    /// The areas that own a room, in the list's order, each with its hue.
    pub key: Vec<AreaHue>,
    pub busiest: Vec<Busy>,
    pub glossary: Glossary,
}

/// `#3862 #3863`.
pub fn pulls_said(pulls: &[u64]) -> String {
    pulls.iter().map(|pull| format!("#{pull}")).collect::<Vec<_>>().join(" ")
}

fn tile(room: &MapRoom, areas: &[AreaHue]) -> Tile {
    let owner = room.area.as_deref().and_then(|slug| areas.iter().find(|area| area.slug == slug));
    let said = match owner {
        Some(area) => format!("{}, {}", room.module, area.title),
        None => room.module.clone(),
    };
    Tile { module: room.module.clone(), path: room.path.clone(), hue: owner.map(|area| area.hue), said }
}

pub fn overview(read: &ProjectOverview, areas: &[AreaHue]) -> OverviewModel {
    let tiles: Vec<Tile> = read.rooms.iter().map(|room| tile(room, areas)).collect();
    let key = areas.iter().filter(|area| read.rooms.iter().any(|room| room.area.as_deref() == Some(area.slug.as_str()))).cloned().collect();
    let glossary = if read.glossary.is_empty() { Glossary::Empty(GLOSSARY_EMPTY) } else { Glossary::Terms(read.glossary.iter().map(|term| (term.term.clone(), term.meaning.clone())).collect()) };
    OverviewModel {
        lede: read.what_it_is.as_ref().filter(|what| !what.sentence.trim().is_empty()).map(|what| (what.sentence.clone(), what.file.clone())),
        tiles,
        key,
        busiest: read.busiest.iter().map(|one| Busy { slug: one.slug.clone(), title: one.title.clone(), pulls: pulls_said(&one.pulls) }).collect(),
        glossary,
    }
}

/// The chip's words and its hue: New green, Changed blue, To confirm amber. A feature waits on him for its Confirm.
pub fn tag_mark(tag: LookedTag) -> (&'static str, Token) {
    match tag {
        LookedTag::New => ("New", token::STATUS_COMPLETED_SUCCESS),
        LookedTag::Changed => ("Changed", token::STATUS_RUNNING),
        LookedTag::NeedsYou => ("To confirm", token::STATUS_AWAITING_REVIEW),
    }
}

/// One row of Since you looked.
#[derive(Clone, Debug, PartialEq)]
pub struct Moved {
    pub tag: LookedTag,
    pub slug: String,
    pub title: String,
    pub what: String,
    pub pulls: String,
}

/// The Since you looked page: the paragraph where there is one, and the rows in Fleet's order (Needs you, Changed, New).
#[derive(Clone, Debug, PartialEq)]
pub struct SinceModel {
    pub paragraph: Option<String>,
    pub rows: Vec<Moved>,
}

pub fn since(read: &SinceYouLooked) -> SinceModel {
    SinceModel {
        paragraph: Some(read.paragraph.trim().to_string()).filter(|text| !text.is_empty()),
        rows: read.features.iter().map(|one| Moved { tag: one.tag, slug: one.slug.clone(), title: one.title.clone(), what: one.what.clone(), pulls: pulls_said(&one.pulls) }).collect(),
    }
}

/// How far back the first look reaches where nothing was kept and the librarian never ran.
const FIRST_LOOK_DAYS: i64 = 14;

/// The time to compare with: the one he marked read, else the librarian's last run, else fourteen days ago.
pub fn first_since(kept: Option<DateTime<Utc>>, ran: Option<DateTime<Utc>>, now: DateTime<Utc>) -> DateTime<Utc> {
    kept.or(ran).unwrap_or(now - Duration::days(FIRST_LOOK_DAYS))
}

/// `Tue 17 Nov, 18:40`, in his time.
pub fn when_said(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local).format("%a %-d %b, %H:%M").to_string()
}

pub fn nothing_since(at: DateTime<Utc>) -> String {
    format!("Nothing changed since {}", when_said(at))
}
