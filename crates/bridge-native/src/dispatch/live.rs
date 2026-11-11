//! The sheet on a live Fleet: its repositories are the ones Fleet serves, narrowed to the project the
//! window is scoped to, and Fleet's answers reach it as data. What the sheet draws stays `face.rs`.

use gpui::SharedString;
use ipc::{ManifestId, RepositorySummary};

use super::face::{Chip, Face, Kind, Mark, Mentions, Offered, Reading, Tuning};

/// A repository Fleet serves: the name its chip carries and the id a Job there starts with.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Served {
    pub name: SharedString,
    pub id: ManifestId,
}

/// The served repositories that can start a Job: each by the name the title bar's picker gives it, with the
/// id it starts on (its Manifest's, else the one Fleet gives a repository with none). One with neither is left out.
pub(super) fn served(repositories: &[RepositorySummary]) -> Vec<Served> {
    let listed = crate::titlebar::repositories_of(repositories, &[]);
    repositories
        .iter()
        .zip(listed)
        .filter_map(|(one, listed)| {
            let name = listed.name;
            let id = one.manifest.as_ref().map(|manifest| manifest.id.clone()).or_else(|| one.implicit_id.clone())?;
            Some(Served { name, id })
        })
        .collect()
}

/// Chips for `names`, in order: the first nine carry a key, the rest go under More.
pub(super) fn chips(names: &[SharedString]) -> (Vec<Chip>, Vec<Chip>) {
    let chip = |at: usize, name: &SharedString| Chip { name: name.clone(), key: (at < 9).then(|| at as u8 + 1), mark: Mark::Plain };
    let shown = names.iter().take(9).enumerate().map(|(at, name)| chip(at, name)).collect();
    let more = names.iter().enumerate().skip(9).map(|(at, name)| chip(at, name)).collect();
    (shown, more)
}

/// The picker offers only `keep`, where the window is scoped to a project: its member repositories. One
/// member is the repository the sheet is for, as the rail's repository is; several leave the words to choose.
pub(super) fn narrowed(mut face: Face, keep: &[SharedString]) -> Face {
    let names: Vec<SharedString> = face.chips.iter().chain(&face.more).map(|chip| chip.name.clone()).filter(|name| keep.contains(name)).collect();
    let (shown, more) = chips(&names);
    face.chips = shown;
    face.more = more;
    face.reading = Reading::Idle;
    if let [only] = names.as_slice() {
        face = read(face, Some(only));
        face.reading = Reading::Scoped;
    }
    face
}

/// The face on a live Fleet. `keep` is the project's member repositories where the window is scoped to one.
pub(super) fn face_of(served: &[Served], keep: Option<&[SharedString]>, jobs: Vec<Offered>, connected: bool, tuning: Tuning) -> Face {
    let names: Vec<SharedString> = served.iter().map(|one| one.name.clone()).collect();
    let (chips, more) = chips(&names);
    let face = Face {
        connected,
        kinds: vec![Kind::Job],
        chips,
        more,
        reading: Reading::Idle,
        tuning,
        queue: false,
        mentions: Mentions { jobs, ..Mentions::default() },
        wait: None,
        read: None,
        refused: None,
    };
    match keep {
        Some(keep) => narrowed(face, keep),
        None => face,
    }
}

/// What Fleet read of the words: the repository they name is chosen, or none is.
pub(super) fn read(mut face: Face, named: Option<&SharedString>) -> Face {
    for chip in face.chips.iter_mut().chain(face.more.iter_mut()) {
        chip.mark = if Some(&chip.name) == named { Mark::Chosen } else { Mark::Plain };
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
pub(super) fn call(port: u16, path: String, body: Option<String>) -> Result<Vec<u8>, String> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
    let (status, bytes) = runtime.block_on(async {
        match body {
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
