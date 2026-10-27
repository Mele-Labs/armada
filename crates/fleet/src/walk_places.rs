//! Where a pull request's change runs: found from the pull request, then from what Fleet remembers
//! of the repository and the area of code, and the places kept. `docs/concepts/pull-review.md`,
//! *Walk*.
//!
//! **Finding reads and spends nothing**: the pull request's page and Fleet's own store. The order is
//! the person's, one source at a time and each only if the one before found nothing; asking him is
//! the third source and is Bridge's.

use std::sync::Arc;

use adapter_traits::{AgentHarness, CheckStanding, Delivery, PageFile, PullRequestPageFacts, Said, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ManifestId, RememberedPlace, RememberedPlaces, SaveWalkPlace, WalkHow, WalkPlace, WalkPlaceFound, WalkSource,
    WalkTried,
};
use store::KeptWalkPlace;

use crate::daemon::Fleet;
use crate::review_runs::UNREADABLE;

/// A place that cannot be kept or walked. A 422.
pub(crate) const INVALID: &str = "fleet.walk_place_invalid";

const COMMAND_AT_MOST: usize = 2_000;

fn is_web_address(text: &str) -> bool {
    let rest = text.strip_prefix("https://").or_else(|| text.strip_prefix("http://"));
    let host = rest.and_then(|rest| rest.split(['/', '?', '#']).next()).unwrap_or("");
    !host.is_empty()
        && !host.starts_with([':', '@'])
        && !text.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '\'' | '"' | '`' | '\\'))
}

/// Why a place cannot be walked, or `None` where it can. **At least one address**, each an http or
/// https one; `pull` is started by a command and `deployed` is already running, so it has none.
pub(crate) fn place_problem(place: &WalkPlace) -> Option<String> {
    if place.address.is_empty() && place.api.is_empty() {
        return Some("a place needs a web address or an API base, and this has neither".to_string());
    }
    for (what, text) in [("address", &place.address), ("API base", &place.api)] {
        if !text.is_empty() && !is_web_address(text) {
            return Some(format!("the {what} `{text}` is not an http or https address with a host"));
        }
    }
    let command = place.command.trim();
    match place.how {
        WalkHow::Pull if command.is_empty() => Some("a place pulled down needs the command that starts it".to_string()),
        WalkHow::Pull if command.chars().count() > COMMAND_AT_MOST || command.chars().any(char::is_control) => {
            Some(format!("the command is a single line of at most {COMMAND_AT_MOST} characters"))
        }
        WalkHow::Deployed if !command.is_empty() => Some("a place already deployed has no command to start".to_string()),
        _ => None,
    }
}

/// The area as it is kept: a directory of at most two normal parts, no `..`, no leading slash, and
/// a trailing slash dropped. Empty is the repository's root.
pub(crate) fn area_kept(area: &str) -> Result<String, String> {
    let area = area.trim();
    let area = area.strip_suffix('/').unwrap_or(area);
    let parts: Vec<&str> = if area.is_empty() { Vec::new() } else { area.split('/').collect() };
    let normal = |part: &&str| !part.is_empty() && *part != "." && *part != ".." && !part.chars().any(char::is_control);
    if parts.len() > 2 || !parts.iter().all(normal) {
        return Err(format!("`{area}` is not a directory of the repository, one or two parts deep"));
    }
    Ok(parts.join("/"))
}

/// The directory, two deep at most, holding the most changed files, and on a tie the most changed
/// lines. A file at the root is in the root, `""`.
pub(crate) fn area_of(files: &[PageFile]) -> String {
    let mut by: Vec<(String, u64, u64)> = Vec::new();
    for file in files {
        let parts: Vec<&str> = file.path.split('/').collect();
        let directories = &parts[..parts.len().saturating_sub(1)];
        let area = directories[..directories.len().min(2)].join("/");
        let lines = file.additions + file.deletions;
        match by.iter_mut().find(|(held, ..)| *held == area) {
            Some(one) => {
                one.1 += 1;
                one.2 += lines;
            }
            None => by.push((area, 1, lines)),
        }
    }
    by.into_iter()
        .max_by(|a, b| (a.1, a.2).cmp(&(b.1, b.2)).then_with(|| b.0.cmp(&a.0)))
        .map(|(area, ..)| area)
        .unwrap_or_default()
}

/// An area as it is said: a directory with its slash.
fn area_shown(area: &str) -> String {
    match area.is_empty() || area.ends_with('/') {
        true => area.to_string(),
        false => format!("{area}/"),
    }
}

/// Whether a place kept for `kept` covers a change whose area is `area`: the same directory or one
/// above it. The repository's root covers every area.
fn covers(kept: &str, area: &str) -> bool {
    area_shown(area).starts_with(&area_shown(kept))
}

fn is_a_bot(login: &str) -> bool {
    login.to_lowercase().ends_with("bot")
}

/// The address a line names after the word `preview` or `deployed`: the first http or https one
/// past the first such word, up to a space or a closing bracket or quote.
fn announced_in(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    // Lowercasing can change a line's byte length; an address is read from the original only where
    // the two agree on where it is.
    if lower.len() != line.len() {
        return None;
    }
    let word_ends = ["preview", "deployed"]
        .into_iter()
        .filter_map(|word| lower.find(word).map(|at| (at, at + word.len())))
        .min()?
        .1;
    let tail = &lower[word_ends..];
    let from = word_ends + ["https://", "http://"].into_iter().filter_map(|scheme| tail.find(scheme)).min()?;
    let end = line[from..]
        .find(|c: char| c.is_whitespace() || matches!(c, ')' | '>' | ']' | '"' | '\''))
        .map_or(line.len(), |len| from + len);
    Some(line[from..end].to_string())
}

/// What the pull request says about where it runs, and the address it names: a deploy bot's
/// comment naming a preview or a deployed address, else the details address of a passed check named
/// for a preview or a deploy.
pub(crate) fn from_pull_request(page: &PullRequestPageFacts) -> Option<(WalkTried, String)> {
    for said in &page.conversation {
        let Said::Comment { author, body, .. } = said else {
            continue;
        };
        if !is_a_bot(author) {
            continue;
        }
        if let Some((line, address)) = body.lines().find_map(|line| announced_in(line).filter(|found| is_web_address(found)).map(|found| (line, found))) {
            let tried = WalkTried {
                source: WalkSource::PullRequest,
                found: true,
                said: line.trim().to_string(),
                by: Some(author.clone()),
            };
            return Some((tried, address));
        }
    }
    page.checks.iter().find_map(|check| {
        let name = check.name.to_lowercase();
        let address = check.address.as_deref().map(str::trim)?;
        let deploys = name.contains("preview") || name.contains("deploy");
        (check.standing == CheckStanding::Passed && deploys && is_web_address(address)).then(|| {
            let tried = WalkTried {
                source: WalkSource::PullRequest,
                found: true,
                said: format!("{}: {address}", check.name),
                by: Some(check.name.clone()),
            };
            (tried, address.to_string())
        })
    })
}

/// A place in a sentence.
pub(crate) fn place_said(place: &WalkPlace) -> String {
    let api = (!place.api.is_empty()).then(|| format!("API at {}", place.api));
    let main = match place.how {
        WalkHow::Deployed => format!("Deployed at {}", place.address),
        WalkHow::Pull => {
            let at = (!place.address.is_empty()).then(|| format!("at {}", place.address));
            [Some(format!("Pull it down, run {}", place.command)), at].into_iter().flatten().collect::<Vec<_>>().join(" ")
        }
    };
    match (place.how, api) {
        (WalkHow::Deployed, Some(api)) if place.address.is_empty() => api,
        (_, Some(api)) => format!("{main}, {api}"),
        (_, None) => main,
    }
}

fn kept_place(row: &KeptWalkPlace) -> RememberedPlace {
    RememberedPlace {
        id: row.id.clone(),
        manifest_id: ManifestId::carried(row.manifest_id.clone()),
        repository: String::new(),
        area: row.area.clone(),
        place: WalkPlace {
            how: if row.how == "pull" { WalkHow::Pull } else { WalkHow::Deployed },
            address: row.address.clone(),
            command: row.command.clone(),
            api: row.api.clone(),
        },
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// A repository as the forge names it, else the id it is served under.
    pub(crate) async fn walk_slug(&self, root: &str, manifest_id: &ManifestId) -> String {
        let (reaching, at) = (Arc::clone(self.reaching()), root.to_string());
        let slug = tokio::task::spawn_blocking(move || reaching.slug(&at)).await.ok().flatten();
        slug.unwrap_or_else(|| manifest_id.as_str().to_string())
    }

    /// A refusal for a place or an area that cannot be kept.
    pub(crate) fn walk_place_refusal(&self, said: String) -> Refusal {
        Refusal::Unacceptable(self.pr_refusal(INVALID, said))
    }

    /// Where the pull request's change runs. **Nothing is started or spent.**
    pub(crate) async fn walk_place_found(&self, manifest_id: ManifestId, number: u64) -> Result<WalkPlaceFound, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let page = self
            .forge_asked(served.root(), move |vcs: &V, root: &str| vcs.pull_request_page(root, &number.to_string()))
            .await
            .ok_or_else(|| {
                Refusal::Fault(self.pr_refusal(
                    UNREADABLE,
                    format!("the forge would not say what is on pull request {number}"),
                ))
            })?;
        let area = area_of(&page.files);
        let mut tried = Vec::new();
        if let Some((found, address)) = from_pull_request(&page) {
            let place = WalkPlace { how: WalkHow::Deployed, address, command: String::new(), api: String::new() };
            tried.push(found);
            return Ok(WalkPlaceFound { area, tried, place: Some(place) });
        }
        tried.push(WalkTried {
            source: WalkSource::PullRequest,
            found: false,
            said: "No preview link in the comments or the checks".to_string(),
            by: None,
        });
        let slug = self.walk_slug(served.root(), &manifest_id).await;
        let kept = self
            .store()
            .lock()
            .await
            .walk_places_of(manifest_id.as_str())
            .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))?;
        let remembered = kept
            .iter()
            .filter(|row| covers(&row.area, &area))
            .max_by_key(|row| row.area.len())
            .map(kept_place);
        match remembered {
            Some(one) => {
                tried.push(WalkTried {
                    source: WalkSource::Memory,
                    found: true,
                    said: place_said(&one.place),
                    by: Some(area_shown(&one.area)),
                });
                Ok(WalkPlaceFound { area, tried, place: Some(one.place) })
            }
            None => {
                tried.push(WalkTried {
                    source: WalkSource::Memory,
                    found: false,
                    said: format!("Nothing remembered for {} in {slug}", area_shown(&area)),
                    by: None,
                });
                Ok(WalkPlaceFound { area, tried, place: None })
            }
        }
    }

    /// Every place kept, across the repositories Fleet serves, by repository then area.
    pub(crate) async fn walk_places_listed(&self) -> Result<RememberedPlaces, Refusal> {
        let mut places = Vec::new();
        for served in self.repositories().served() {
            let manifest_id = ManifestId::carried(served.manifest().id().as_str());
            let rows = self
                .store()
                .lock()
                .await
                .walk_places_of(manifest_id.as_str())
                .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))?;
            let slug = self.walk_slug(served.root(), &manifest_id).await;
            places.extend(rows.iter().map(|row| RememberedPlace { repository: slug.clone(), ..kept_place(row) }));
        }
        places.sort_by(|a, b| (&a.repository, &a.area).cmp(&(&b.repository, &b.area)));
        Ok(RememberedPlaces { places })
    }

    /// Remember where changes to an area run, replacing the place kept for it.
    pub(crate) async fn walk_place_saved(
        &self,
        manifest_id: ManifestId,
        save: SaveWalkPlace,
    ) -> Result<RememberedPlace, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        if let Some(problem) = place_problem(&save.place) {
            return Err(self.walk_place_refusal(problem));
        }
        let area = area_kept(&save.area).map_err(|said| self.walk_place_refusal(said))?;
        let row = KeptWalkPlace {
            id: self.mint().ulid().as_str().to_lowercase(),
            manifest_id: manifest_id.as_str().to_string(),
            area,
            how: match save.place.how {
                WalkHow::Pull => "pull",
                WalkHow::Deployed => "deployed",
            }
            .to_string(),
            address: save.place.address.clone(),
            command: save.place.command.trim().to_string(),
            api: save.place.api.clone(),
            saved_at: self.now().as_str().to_string(),
        };
        let kept = self
            .store()
            .lock()
            .await
            .keep_walk_place(&row)
            .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))?;
        let slug = self.walk_slug(served.root(), &manifest_id).await;
        Ok(RememberedPlace { repository: slug, ..kept_place(&kept) })
    }

    /// Stop remembering a place. Answers what is kept now.
    pub(crate) async fn walk_place_forgotten(
        &self,
        manifest_id: ManifestId,
        id: String,
    ) -> Result<RememberedPlaces, Refusal> {
        self.served_named(Some(&manifest_id))?;
        self.store()
            .lock()
            .await
            .forget_walk_place(manifest_id.as_str(), &id)
            .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))?;
        self.walk_places_listed().await
    }
}
