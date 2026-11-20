//! What Who's on what says, worked out from Fleet's one answer in the three ways the page can lay it out: a grid with the features
//! down and the people across, a block for each feature, and a card for each person. Nothing here touches the screen or the wire.

use ipc::WhosOnWhat;

/// What the page says where no open pull request touches a feature.
pub const EMPTY: &str = "No open pull requests touch a feature.";

/// The three layouts of the one answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhoView {
    Grid,
    ByFeature,
    ByPerson,
}

impl WhoView {
    pub const ALL: [WhoView; 3] = [WhoView::Grid, WhoView::ByFeature, WhoView::ByPerson];

    /// The next layout, round to the first after the last.
    pub fn next(self) -> WhoView {
        match self {
            WhoView::Grid => WhoView::ByFeature,
            WhoView::ByFeature => WhoView::ByPerson,
            WhoView::ByPerson => WhoView::Grid,
        }
    }

    /// What the segment says and the file keeps.
    pub fn title(self) -> &'static str {
        match self {
            WhoView::Grid => "Grid",
            WhoView::ByFeature => "By feature",
            WhoView::ByPerson => "By person",
        }
    }

    pub fn of_title(title: &str) -> Option<WhoView> {
        WhoView::ALL.into_iter().find(|view| view.title() == title)
    }
}

/// A person on the page.
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    pub login: String,
    /// "You" for the owner, else the login with its first letter capital.
    pub name: String,
    pub initial: String,
    pub me: bool,
}

/// A pull request in a cell.
#[derive(Clone, Debug, PartialEq)]
pub struct Pull {
    pub number: u64,
    pub repository: String,
    pub title: String,
    pub draft: bool,
}

/// What a person has on a feature: a pull request, or a Job of the owner's with its title where the library names it.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    Pull(Pull),
    Job { id: String, title: Option<String> },
}

/// One person's cell on a feature.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// Where the person stands in `WhoModel::people`.
    pub person: usize,
    pub marks: Vec<Mark>,
}

/// A feature somebody is on.
#[derive(Clone, Debug, PartialEq)]
pub struct Feat {
    pub slug: String,
    pub title: String,
    /// Two or more people are on it.
    pub overlap: bool,
    /// Fleet's order: the owner's cell first, then the others by login.
    pub cells: Vec<Cell>,
}

/// What a person has on one feature, in a person's card.
#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    pub slug: String,
    pub title: String,
    pub marks: Vec<Mark>,
}

/// A person's card.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub person: usize,
    /// The person shares a feature with someone.
    pub overlap: bool,
    pub held: Vec<Held>,
}

/// A place the keys can stop on in a layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stop {
    Feature(String),
    Pull { repository: String, number: u64 },
    Job(String),
}

impl Stop {
    pub fn of(mark: &Mark) -> Stop {
        match mark {
            Mark::Pull(pull) => Stop::Pull { repository: pull.repository.clone(), number: pull.number },
            Mark::Job { id, .. } => Stop::Job(id.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WhoModel {
    /// The owner first, then the others in Fleet's order.
    pub people: Vec<Person>,
    /// In Fleet's order: most people first, then by title.
    pub features: Vec<Feat>,
    /// The others in Fleet's order, then the owner.
    pub cards: Vec<Card>,
}

fn name_of(login: &str, me: bool) -> String {
    if me {
        return "You".into();
    }
    let mut chars = login.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// `You and Priya`, `You, Priya and Dana`.
pub fn names_said(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_string(),
        [first @ .., last] => format!("{} and {last}", first.join(", ")),
    }
}

/// The page's model from Fleet's answer. `jobs` names the owner's Jobs by id, as the library holds them.
pub fn who(read: &WhosOnWhat, jobs: &[(String, String)]) -> WhoModel {
    let mut people: Vec<Person> = read
        .people
        .iter()
        .map(|one| Person { login: one.login.clone(), name: name_of(&one.login, one.me), initial: name_of(&one.login, one.me).chars().next().map(|first| first.to_string()).unwrap_or_default(), me: one.me })
        .collect();
    let mut features = Vec::new();
    for feature in &read.features {
        let mut cells = Vec::new();
        for cell in &feature.cells {
            let person = match people.iter().position(|one| one.login == cell.login) {
                Some(at) => at,
                None => {
                    let name = name_of(&cell.login, false);
                    people.push(Person { login: cell.login.clone(), initial: name.chars().next().map(|first| first.to_string()).unwrap_or_default(), name, me: false });
                    people.len() - 1
                }
            };
            let pulls = cell.pulls.iter().map(|pull| Mark::Pull(Pull { number: pull.number, repository: pull.repository.clone(), title: pull.title.clone(), draft: pull.draft }));
            let owned = cell.jobs.iter().map(|id| Mark::Job { id: id.as_str().to_string(), title: jobs.iter().find(|(held, _)| held == id.as_str()).map(|(_, title)| title.clone()) });
            cells.push(Cell { person, marks: pulls.chain(owned).collect() });
        }
        features.push(Feat { slug: feature.slug.clone(), title: feature.title.clone(), overlap: cells.len() >= 2, cells });
    }
    // Others in Fleet's order, then the owner last.
    let mut order: Vec<usize> = (0..people.len()).filter(|at| !people[*at].me).collect();
    order.extend((0..people.len()).filter(|at| people[*at].me));
    let cards = order
        .into_iter()
        .filter_map(|person| {
            let held: Vec<Held> = features
                .iter()
                .filter_map(|feature| feature.cells.iter().find(|cell| cell.person == person).map(|cell| (feature, cell)))
                .map(|(feature, cell)| Held { slug: feature.slug.clone(), title: feature.title.clone(), marks: cell.marks.clone() })
                .collect();
            let overlap = features.iter().any(|feature| feature.overlap && feature.cells.iter().any(|cell| cell.person == person));
            (!held.is_empty()).then_some(Card { person, overlap, held })
        })
        .collect();
    WhoModel { people, features, cards }
}

impl WhoModel {
    /// Who is on a feature, as its head says it where it is an overlap: `You and Priya`.
    pub fn overlap_said(&self, feature: &Feat) -> Option<String> {
        feature.overlap.then(|| names_said(&feature.cells.iter().map(|cell| self.people[cell.person].name.as_str()).collect::<Vec<_>>()))
    }

    /// The places the keys stop on in `view`, in the order the layout draws them: a feature's name, then its marks.
    pub fn stops(&self, view: WhoView) -> Vec<Stop> {
        let mut out = Vec::new();
        match view {
            WhoView::Grid => {
                for feature in &self.features {
                    out.push(Stop::Feature(feature.slug.clone()));
                    for person in 0..self.people.len() {
                        if let Some(cell) = feature.cells.iter().find(|cell| cell.person == person) {
                            out.extend(cell.marks.iter().map(Stop::of));
                        }
                    }
                }
            }
            WhoView::ByFeature => {
                for feature in &self.features {
                    out.push(Stop::Feature(feature.slug.clone()));
                    for cell in &feature.cells {
                        out.extend(cell.marks.iter().map(Stop::of));
                    }
                }
            }
            WhoView::ByPerson => {
                for card in &self.cards {
                    for held in &card.held {
                        out.push(Stop::Feature(held.slug.clone()));
                        out.extend(held.marks.iter().map(Stop::of));
                    }
                }
            }
        }
        out
    }
}
