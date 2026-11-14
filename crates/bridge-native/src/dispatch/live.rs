//! The sheet on a live Fleet: its picker lists his projects with their repositories, then the repositories
//! Fleet serves in no project, narrowed to what the window is scoped to, and Fleet's answers reach it as
//! data. What the sheet draws stays `face.rs`.

use gpui::SharedString;
use ipc::{ManifestId, RepositorySummary, StartAfter, StartBase, StartCondition, WaitReading};

use super::face::{Chip, Cycle, Face, InProject, Mark, Mentions, Offered, Part, Reading, Tuning, Wait};

/// A repository Fleet serves: the name its chip carries and the id a Job there starts with.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Served {
    pub name: SharedString,
    pub id: ManifestId,
    /// Where it is checked out: what a queued idea keeps, as Electron's does.
    pub root: String,
}

/// The served repositories that can start a Job: each by the name the title bar's picker gives it, with the
/// id it starts on (its Manifest's, else the one Fleet gives a repository with none). One with neither is left out.
pub(crate) fn served(repositories: &[RepositorySummary]) -> Vec<Served> {
    let listed = crate::titlebar::repositories_of(repositories, &[]);
    repositories
        .iter()
        .zip(listed)
        .filter_map(|(one, listed)| {
            let name = listed.name;
            let id = one.manifest.as_ref().map(|manifest| manifest.id.clone()).or_else(|| one.implicit_id.clone())?;
            Some(Served { name, id, root: one.root.clone() })
        })
        .collect()
}

/// A project and its repositories by the names the picker gives them, in its members' order.
pub(crate) type ProjectRepositories = (InProject, Vec<SharedString>);

/// His projects as the title bar holds them, each with the repositories of its members once, in their order.
pub(crate) fn projects_of(projects: &[crate::titlebar::Project], repositories: &[crate::titlebar::Repository]) -> Vec<ProjectRepositories> {
    projects
        .iter()
        .map(|project| {
            let mut names: Vec<SharedString> = Vec::new();
            for member in &project.members {
                if let Some(one) = repositories.get(member.repository).filter(|one| !names.contains(&one.name)) {
                    names.push(one.name.clone());
                }
            }
            (InProject { id: project.id.clone(), name: project.name.clone() }, names)
        })
        .collect()
}

/// The picker's rows in order: each project with each repository of it that Fleet serves, then every served
/// repository in no project, which is also where any folder Fleet serves stands.
pub(super) fn rows_of(served: &[Served], projects: &[ProjectRepositories]) -> Vec<(Option<InProject>, SharedString)> {
    let serves = |name: &SharedString| served.iter().any(|one| one.name == *name);
    let mut rows: Vec<(Option<InProject>, SharedString)> = Vec::new();
    for (project, names) in projects {
        rows.extend(names.iter().filter(|name| serves(name)).map(|name| (Some(project.clone()), name.clone())));
    }
    rows.extend(served.iter().map(|one| (None, one.name.clone())));
    rows
}

/// Chips for `rows`, in order: the first nine carry a key, the rest go under More.
pub(super) fn chips(rows: &[(Option<InProject>, SharedString)]) -> (Vec<Chip>, Vec<Chip>) {
    let chip = |at: usize, (project, name): &(Option<InProject>, SharedString)| Chip { name: name.clone(), project: project.clone(), key: (at < 9).then(|| at as u8 + 1), mark: Mark::Plain };
    let shown = rows.iter().take(9).enumerate().map(|(at, row)| chip(at, row)).collect();
    let more = rows.iter().enumerate().skip(9).map(|(at, row)| chip(at, row)).collect();
    (shown, more)
}

/// The picker offers only what the window is scoped to: the rows of `project` among `keep`, or with no
/// project, the repository in `keep` on its own. One row is the one the sheet is for, as the rail's
/// repository is; several leave the words to choose.
pub(super) fn narrowed(mut face: Face, project: Option<&SharedString>, keep: &[SharedString]) -> Face {
    let rows: Vec<(Option<InProject>, SharedString)> = face
        .chips
        .iter()
        .chain(&face.more)
        .filter(|chip| keep.contains(&chip.name) && chip.project.as_ref().map(|one| &one.id) == project)
        .map(|chip| (chip.project.clone(), chip.name.clone()))
        .collect();
    let (shown, more) = chips(&rows);
    face.chips = shown;
    face.more = more;
    face.reading = Reading::Idle;
    if let [(_, only)] = rows.as_slice() {
        face = read(face, Some(only));
        face.reading = Reading::Scoped;
    }
    face
}

/// The face on a live Fleet. `keep` is what the window is scoped to: a project by its id with its
/// repositories, or one repository in no project.
pub(super) fn face_of(served: &[Served], projects: &[ProjectRepositories], keep: Option<(Option<&SharedString>, &[SharedString])>, jobs: Vec<Offered>, connected: bool, tuning: Tuning) -> Face {
    let (chips, more) = chips(&rows_of(served, projects));
    let face = Face {
        connected,
        chips,
        more,
        reading: Reading::Idle,
        tuning,
        // Queue for later keeps the idea on this machine (`crate::queued`): nothing to ask of Fleet.
        queue: true,
        mentions: Mentions { jobs, ..Mentions::default() },
        wait: None,
        refused: None,
    };
    match keep {
        Some((project, keep)) => narrowed(face, project, keep),
        None => face,
    }
}

/// What Fleet read of the words: the first row of the repository they name is chosen, a project's
/// before none, or no row is.
pub(super) fn read(mut face: Face, named: Option<&SharedString>) -> Face {
    let mut found = false;
    for chip in face.chips.iter_mut().chain(face.more.iter_mut()) {
        let first = !found && Some(&chip.name) == named;
        found |= first;
        chip.mark = if first { Mark::Chosen } else { Mark::Plain };
    }
    face.reading = if named.is_some() { Reading::Detected } else { Reading::Unresolved };
    face
}

/// The default the sheet's efforts and models start from, until Fleet has said its models.
pub(super) fn tuning(models: Option<&ipc::ModelChoices>) -> Tuning {
    Tuning {
        default_model: models.map(|read| read.session.clone().into()),
        models: models.map(|read| read.models.iter().map(|one| one.clone().into()).collect()).unwrap_or_default(),
        efforts: ["low", "medium", "high", "xhigh", "max"].iter().map(|one| one.to_string().into()).collect(),
    }
}

/// Ask Fleet: a POST where there is a body, else a GET. Blocks, so it runs on the background executor. A refusal
/// is Fleet's own sentence.
pub(crate) fn call(port: u16, path: String, body: Option<String>) -> Result<Vec<u8>, String> {
    answered(port, path, body, false)
}

/// A POST to a route that asks a model inside the request: the proposal and the wait read wait as long
/// as Electron's do.
pub(super) fn call_model(port: u16, path: String, body: String) -> Result<Vec<u8>, String> {
    answered(port, path, Some(body), true)
}

fn answered(port: u16, path: String, body: Option<String>, model: bool) -> Result<Vec<u8>, String> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
    let (status, bytes) = runtime.block_on(async {
        match body {
            Some(body) if model => crate::transport::post_to_model(port, &path, body).await,
            Some(body) => crate::transport::post(port, &path, body).await,
            None => crate::transport::get(port, &path).await,
        }
    })?;
    if (200..300).contains(&status) {
        Ok(bytes)
    } else {
        Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
    }
}

// -- waits ---------------------------------------------------------------------------------------

/// A condition by the key its part of the sentence cycles it by. A part of the plan is named by its id: the
/// plan of the Job waited on is not held here, so its title is not.
pub(super) fn when_key(condition: &StartCondition) -> String {
    match condition {
        StartCondition::Ends => "ends".into(),
        StartCondition::GroupDone { group } => format!("group:{group}"),
        StartCondition::TaskDone { task } => format!("task:{task}"),
        StartCondition::PrPushed => "pushed".into(),
        StartCondition::PrLands => "lands".into(),
    }
}

fn condition_of(key: &str) -> StartCondition {
    match key {
        "pushed" => StartCondition::PrPushed,
        "lands" => StartCondition::PrLands,
        _ => match key.split_once(':') {
            Some(("group", group)) => StartCondition::GroupDone { group: group.to_string() },
            Some(("task", task)) => StartCondition::TaskDone { task: task.to_string() },
            _ => StartCondition::Ends,
        },
    }
}

fn base_key(base: StartBase) -> &'static str {
    match base {
        StartBase::Main => "main",
        StartBase::Branch => "branch",
    }
}

fn options(pairs: &[(&str, &str)]) -> Vec<(SharedString, SharedString)> {
    pairs.iter().map(|(value, label)| (value.to_string().into(), label.to_string().into())).collect()
}

/// What a line of needs offers: all of it, and each thing where it asks for several (`needsOf`, `waits.ts`).
fn needs_options(needs: &str) -> Vec<(SharedString, SharedString)> {
    let words: Vec<&str> = needs.split_whitespace().collect();
    let mut each: Vec<String> = vec![String::new()];
    for word in words {
        if word.eq_ignore_ascii_case("and") {
            each.push(String::new());
        } else if let Some(last) = each.last_mut() {
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(word);
        }
    }
    let each: Vec<String> = each.into_iter().filter(|one| !one.is_empty()).collect();
    std::iter::once(needs.to_string()).chain(if each.len() > 1 { each } else { Vec::new() }).map(|one| (one.clone().into(), one.into())).collect()
}

/// The sentence for a reading of the brief that tags `on`. A part a hand set stands while the reading of that
/// part is what it was when he set it (`hand` is the sentence and the reading it was drawn from); a correction
/// in the words that changes the reading takes the part back (`chosen`, `waits.ts`).
pub(super) fn wait_of(on: Offered, others: Vec<Offered>, read: &WaitReading, hand: Option<(&Wait, &WaitReading)>) -> Wait {
    let when_key = when_key(&read.condition);
    let mut when_options = vec![("ends", "ends")];
    let part = match &read.condition {
        StartCondition::GroupDone { .. } => Some("a group of its plan is done"),
        StartCondition::TaskDone { .. } => Some("a task of its plan is done"),
        _ => None,
    };
    if let Some(label) = part {
        when_options.push((when_key.as_str(), label));
    }
    when_options.extend([("pushed", "PR is pushed"), ("lands", "PR lands")]);
    let cycle = |value: &str, options: Vec<(SharedString, SharedString)>| {
        let label = options.iter().find(|(one, _)| one == value).map(|(_, label)| label.clone()).unwrap_or_else(|| value.to_string().into());
        Cycle { value: value.to_string().into(), label, options }
    };
    let mut wait = Wait {
        on,
        when: cycle(&when_key, options(&when_options)),
        base: cycle(base_key(read.base), options(&[("main", "main"), ("branch", "its PR branch")])),
        needs: (!read.needs.is_empty()).then(|| cycle(&read.needs, needs_options(&read.needs))),
        his: Vec::new(),
        others,
    };
    if let Some((held, was)) = hand {
        if held.his(Part::When) && self::when_key(&was.condition) == when_key {
            wait.when = held.when.clone();
            wait.his.push(Part::When);
        }
        if held.his(Part::Base) && was.base == read.base {
            wait.base = held.base.clone();
            wait.his.push(Part::Base);
        }
        if held.his(Part::Needs) && was.needs == read.needs {
            wait.needs = held.needs.clone();
            wait.his.push(Part::Needs);
        }
    }
    wait
}

/// The reading where the proposer could not be had: the Job ends, from main, asking nothing (`waitReading`).
pub(super) fn defaults() -> WaitReading {
    WaitReading { condition: StartCondition::Ends, base: StartBase::Main, needs: String::new(), plain: false }
}

/// The wire's `after` for the sentence as it stands.
pub(super) fn after_of(wait: &Wait) -> StartAfter {
    StartAfter {
        session: ipc::SessionId::carried(wait.on.id.to_string()),
        condition: condition_of(&wait.when.value),
        base: if wait.base.value == "branch" { StartBase::Branch } else { StartBase::Main },
        needs: wait.needs.as_ref().map(|needs| needs.value.to_string()).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mock Mac's picker at rest: his projects as the walks' fixture has them, then every repository.
    fn rest() -> Face {
        crate::dispatch::scenes::rest(&[])
    }

    fn rows(face: &Face) -> Vec<(Option<&str>, &str)> {
        face.chips.iter().chain(&face.more).map(|chip| (chip.project.as_ref().map(|one| one.name.as_ref()), chip.name.as_ref())).collect()
    }

    #[test]
    fn the_picker_lists_each_project_with_its_repositories_then_every_repository_in_none() {
        let face = rest();
        assert_eq!(
            rows(&face),
            [
                (Some("Armada"), "armada"),
                (Some("Storefront"), "sample-web"),
                (Some("Storefront"), "sample-api"),
                (None, "armada"),
                (None, "sample-web"),
                (None, "sample-api"),
                (None, "sample-docs"),
            ],
            "a project whose file did not read is left out, and a folder Fleet serves stands in no project"
        );
        assert_eq!(face.chips.iter().map(|chip| chip.key).collect::<Vec<_>>(), (1..=7).map(Some).collect::<Vec<_>>());
    }

    #[test]
    fn a_window_on_a_project_offers_its_rows_alone() {
        let face = narrowed(rest(), Some(&"storefront".into()), &["sample-web".into(), "sample-api".into()]);
        assert_eq!(rows(&face), [(Some("Storefront"), "sample-web"), (Some("Storefront"), "sample-api")]);
        assert_eq!(face.reading, Reading::Idle, "two rows leave the words to choose");
        assert_eq!(face.chips.iter().map(|chip| chip.key).collect::<Vec<_>>(), [Some(1), Some(2)]);
    }

    #[test]
    fn a_window_on_a_repository_in_no_project_starts_there() {
        let face = narrowed(rest(), None, &["armada".into()]);
        assert_eq!(rows(&face), [(None, "armada")]);
        assert_eq!(face.reading, Reading::Scoped);
        assert!(face.chosen().is_some_and(|chip| chip.project.is_none()));
    }

    #[test]
    fn the_words_choose_the_first_row_of_their_repository() {
        let face = read(rest(), Some(&"sample-api".into()));
        let chosen = face.chosen().expect("a row is chosen");
        assert_eq!((chosen.project.as_ref().map(|one| one.id.as_ref()), chosen.name.as_ref()), (Some("storefront"), "sample-api"));
        assert_eq!(face.chips.iter().chain(&face.more).filter(|chip| chip.mark == Mark::Chosen).count(), 1, "one row stands, not each of the repository's");
    }

    #[test]
    fn a_pick_by_hand_is_the_row_and_not_its_repository() {
        let mut face = rest();
        let loose = face.chips.iter().find(|chip| chip.project.is_none() && chip.name == "armada").expect("armada in no project").id();
        face.pick(&loose);
        let chosen = face.chosen().expect("a row is chosen");
        assert!(chosen.project.is_none(), "armada in no project, not in Armada");
        assert_eq!(chosen.label().as_ref(), "armada");
    }
}
