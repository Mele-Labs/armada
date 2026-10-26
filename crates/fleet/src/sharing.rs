//! Share: **pieces a person keeps for a repository, made into one package, read, and added to a
//! colleague's own Kit.** `docs/concepts/kit.md`, *Share*.
//!
//! Fleet makes the text and reads it, and writes nothing to make it. Bridge writes the file and
//! the clipboard. What is added becomes the colleague's own, in the places their own pieces live,
//! and runs like it.
//!
//! **A piece lands on the one repository whose remote is its own.** A remote that names none of
//! the repositories Fleet serves, or several, keeps the piece unbound in [`unbound`] until a
//! person chooses one, as `docs/concepts/account.md` keeps a repository no profile owns.
//!
//! **Only the pieces a package holds are written.** An id Kit holds, a tile that breaks a rule and
//! a workflow `save_workflow` would refuse are all found before the first write, and leave Kit as
//! it was. A file that cannot be written is found only by writing it, and leaves the pieces
//! written before it written: workflows, then review tools, then manifests, then requests, then one
//! repository's tiles at a time, then the unbound pieces.

mod package;
mod unbound;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal};
use config::{KitCopy, KitRead, WorkflowSource};
use ipc::{
    AddPackage, BindUnboundPiece, KitManifestSaved, SaveKitManifest, FlightControl, FlightControls, FlightControlsRead, Landed, PackageAdded,
    PackagePiece, PackageRead, PackageText, PieceAdded, PieceToCommit, PieceKind, ReviewToolScope,
    SaveRepositoryFlightControls, SaveWorkflow, SharePieces, SharedPackage, TileList, UnboundPiece,
    UnboundPieces, WireError, WorkflowScope,
};

use crate::daemon::Fleet;
use crate::flight_controls::repository::check_repository;
use crate::kit_manifests::KitManifest;
use package::{Piece, Shown};

/// No pieces chosen, or a piece the person does not keep. A 422.
const NOT_YOURS: &str = "fleet.share_not_yours";
/// Text that is not a package. A 422.
const MALFORMED: &str = "fleet.share_malformed";
/// Kit already holds the id and `replace` was not asked for. A 422.
const EXISTS: &str = "fleet.share_exists";
/// A repository's tile file will not read, or the tiles together break a rule. A 422.
const REFUSED: &str = "fleet.share_refused";
/// Nothing is kept unbound under the id. A 404.
const NO_UNBOUND: &str = "fleet.share_no_unbound_piece";
/// A file could not be written. A 500.
const UNWRITABLE: &str = "fleet.share_unwritable";
/// Text that is not a person's `armada.yml`. A 422.
const KIT_MANIFEST_REFUSED: &str = "fleet.kit_manifest_refused";
/// The place Kit keeps the copy in would not answer. A 422.
const KIT_MANIFEST_UNPLACED: &str = "fleet.kit_manifest_unplaced";
/// An agent asked for a repository other than its own Job's. A 422.
const KIT_MANIFEST_ELSEWHERE: &str = "fleet.kit_manifest_elsewhere";
/// The copy could not be written. A 500.
const KIT_MANIFEST_UNWRITABLE: &str = "fleet.kit_manifest_unwritable";

/// Write `text` to `<path>.writing` and rename it over `path`.
fn write_over(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut staged = path.as_os_str().to_owned();
    staged.push(".writing");
    let staged = PathBuf::from(staged);
    std::fs::write(&staged, text).and_then(|()| std::fs::rename(&staged, path))
}

/// The one root whose slug is `remote`. None, or several, name no repository.
fn matching(slugs: &[(String, Option<String>)], remote: Option<&str>) -> Option<String> {
    let remote = remote?;
    let mut named = slugs.iter().filter(|(_, slug)| slug.as_deref().is_some_and(|slug| slug.eq_ignore_ascii_case(remote)));
    match (named.next(), named.next()) {
        (Some((root, _)), None) => Some(root.clone()),
        _ => None,
    }
}

/// Put `tile` into `list`: in place of one of the same id, else at the end.
fn put(list: &mut Vec<FlightControl>, tile: FlightControl) {
    match list.iter().position(|held| held.id == tile.id) {
        Some(at) => list[at] = tile,
        None => list.push(tile),
    }
}

fn added_of(shown: Shown, repository: Option<String>, landed: Landed, unbound_id: Option<String>) -> PieceAdded {
    PieceAdded { kind: shown.kind, id: shown.id, name: shown.name, list: shown.list, remote: shown.remote, repository, landed, unbound_id }
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
    fn share_refusal(&self, code: &str, said: String) -> Refusal {
        let raised = WireError::raised(code, said, self.run_id());
        match code {
            NO_UNBOUND => Refusal::NotFound(raised),
            UNWRITABLE => Refusal::Fault(raised),
            _ => Refusal::Unacceptable(raised),
        }
    }

    fn kit(&self) -> PathBuf {
        PathBuf::from(&self.host().kit_home)
    }

    /// Each repository Fleet serves, and the remote it pushes to. Read off this thread.
    async fn served_slugs(&self) -> Vec<(String, Option<String>)> {
        let roots: Vec<String> = self.repositories().served().iter().map(|one| one.root().to_string()).collect();
        let reaching = std::sync::Arc::clone(self.reaching());
        tokio::task::spawn_blocking(move || roots.into_iter().map(|root| { let slug = reaching.slug(&root); (root, slug) }).collect())
            .await
            .unwrap_or_default()
    }

    /// `share_package`: the chosen pieces as one package's text.
    pub(crate) async fn package_made(&self, asked: SharePieces) -> Result<SharedPackage, Refusal> {
        if asked.pieces.is_empty() {
            return Err(self.share_refusal(NOT_YOURS, String::from("No pieces are chosen.")));
        }
        let slugs = self.served_slugs().await;
        let mut made = Vec::new();
        for chosen in asked.pieces {
            let root = self.run_repository(&chosen.repository)?;
            let remote = slugs.iter().find(|(one, _)| *one == root).and_then(|(_, slug)| slug.clone());
            match chosen.piece {
                PieceToCommit::FlightControls { job, deck } => made.extend(self.tiles_kept(&root, &remote, &job, &deck).await?),
                PieceToCommit::Workflow { workflow_id } => made.push(self.workflow_kept(&root, remote, workflow_id.as_str())?),
                PieceToCommit::ReviewTool { id } => made.push(self.review_tool_kept(&root, remote, &id)?),
                PieceToCommit::Manifest => made.push(self.manifest_kept(&root, remote).await?),
                PieceToCommit::Requests { collections, environments } => {
                    made.push(self.requests_shared(&root, remote, &collections, &environments).await?)
                }
            }
        }
        let text = package::written(made.clone()).map_err(|why| self.share_refusal(UNWRITABLE, format!("The package could not be made: {why}")))?;
        Ok(SharedPackage { file_name: package::file_name(&made), text })
    }

    /// The chosen tiles from the person's own for `root`, each a piece.
    async fn tiles_kept(&self, root: &str, remote: &Option<String>, job: &[String], deck: &[String]) -> Result<Vec<Piece>, Refusal> {
        if job.is_empty() && deck.is_empty() {
            return Err(self.share_refusal(NOT_YOURS, String::from("No tiles are chosen.")));
        }
        let yours = match self.repository_flight_controls_read(root.to_string(), None).await?.read {
            FlightControlsRead::Read { controls } => controls,
            FlightControlsRead::NotRead { why } => return Err(self.share_refusal(NOT_YOURS, format!("Your tiles for this repository did not read: {why}"))),
        };
        let mut pieces = Vec::new();
        for (list, chosen, from) in [(TileList::Job, job, &yours.job), (TileList::Deck, deck, &yours.deck)] {
            for id in chosen {
                let Some(tile) = from.iter().find(|tile| &tile.id == id) else {
                    return Err(self.share_refusal(NOT_YOURS, format!("Your tiles for this repository have no `{id}` in {}.", list_word(list))));
                };
                pieces.push(Piece::Tile { list, remote: remote.clone(), tile: tile.clone() });
            }
        }
        Ok(pieces)
    }

    /// The chosen collections and environments from the person's own for `root`, as one piece.
    async fn requests_shared(&self, root: &str, remote: Option<String>, collections: &[String], environments: &[String]) -> Result<Piece, Refusal> {
        let yours = self
            .own_requests_file(root)
            .await
            .map_err(|why| self.share_refusal(NOT_YOURS, format!("Your requests for this repository did not read: {why}")))?;
        let chosen = crate::requesting::copied(ipc::RequestsFile::default(), &yours, collections, environments, false).map_err(|why| match why {
            crate::requesting::NotCopied::NotYours(why) | crate::requesting::NotCopied::Exists(why) => self.share_refusal(NOT_YOURS, why),
        })?;
        Ok(Piece::Requests { remote, collections: chosen.collections, environments: chosen.environments })
    }

    /// A workflow from Kit, as its file is written.
    fn workflow_kept(&self, root: &str, remote: Option<String>, id: &str) -> Result<Piece, Refusal> {
        let not_yours = || self.share_refusal(NOT_YOURS, format!("Your Kit holds no workflow `{id}` this repository runs."));
        let served = self.repositories().served().into_iter().find(|one| one.root() == root).ok_or_else(not_yours)?;
        let files = served.files();
        let kit = files.iter().find(|file| file.source() == WorkflowSource::Kit && file.id().as_str() == id).ok_or_else(not_yours)?;
        Ok(Piece::Workflow { remote, text: kit.text().to_string() })
    }

    /// A review tool from Kit, as its file is written.
    fn review_tool_kept(&self, root: &str, remote: Option<String>, id: &str) -> Result<Piece, Refusal> {
        let folder = self.review_tool_folder(ReviewToolScope::Kit, root);
        let held = crate::review_tools::review_tool_defined(&folder, ReviewToolScope::Kit, id);
        match held {
            Some((_, text)) => Ok(Piece::ReviewTool { remote, id: id.to_string(), text }),
            None => Err(self.share_refusal(NOT_YOURS, format!("Your Kit holds no review tool `{id}`."))),
        }
    }

    /// The place the person's Kit keeps the pieces of the repository at `root`.
    pub(crate) async fn kit_place_of(&self, root: &str) -> Result<ipc::RunPlace, Refusal> {
        let held = self.kept_place(root).await?;
        Ok(crate::run_memory::places::where_kept(&self.kit(), held.as_ref()).0)
    }

    /// Kit's copy of the `armada.yml` for `root`, at the place their Kit keeps it, to lay over a file.
    pub(crate) async fn kit_copy_of(&self, root: &str) -> Result<std::sync::Arc<dyn config::KitCopy>, Refusal> {
        let held = self.kept_place(root).await?;
        Ok(std::sync::Arc::new(KitManifest::at(&self.kit(), root, held.as_ref(), std::sync::Arc::clone(self.reaching()))))
    }

    /// The person's `armada.yml` for `root`, read from the place their Kit keeps it, off this thread.
    pub(crate) async fn kit_manifest_read(&self, root: &str) -> Result<(PathBuf, KitRead), Refusal> {
        let held = self.kept_place(root).await?;
        let copy = KitManifest::at(&self.kit(), root, held.as_ref(), std::sync::Arc::clone(self.reaching()));
        let label = copy.label();
        let read = tokio::task::spawn_blocking(move || copy.read()).await.unwrap_or_else(|why| KitRead::Unread(why.to_string()));
        Ok((label, read))
    }

    /// The person's `armada.yml` for `root`, as its file holds it.
    async fn manifest_kept(&self, root: &str, remote: Option<String>) -> Result<Piece, Refusal> {
        match self.kit_manifest_read(root).await?.1 {
            KitRead::Text(text) => Ok(Piece::Manifest { remote, text }),
            KitRead::Absent => Err(self.share_refusal(NOT_YOURS, String::from("Your Kit holds no armada.yml for this repository."))),
            KitRead::Unread(why) => Err(self.share_refusal(NOT_YOURS, format!("Your armada.yml for this repository did not read: {why}"))),
        }
    }

    /// Replace the person's `armada.yml` for `root` with `text`, at the place their Kit keeps it.
    pub(crate) async fn manifest_written(&self, root: &str, text: String) -> Result<(), Refusal> {
        let held = self.kept_place(root).await?;
        let copy = KitManifest::at(&self.kit(), root, held.as_ref(), std::sync::Arc::clone(self.reaching()));
        let label = copy.label();
        let wrote = tokio::task::spawn_blocking(move || copy.write(&text)).await.unwrap_or_else(|why| Err(why.to_string()));
        wrote.map_err(|why| self.share_refusal(UNWRITABLE, format!("{} could not be written: {why}", label.display())))
    }

    /// The repository the Job whose process holds this connection works in: the root a setup Job
    /// was started in while it has no Manifest, else the root of the Manifest it is bound to.
    async fn calling_job_repository(&self, caller: &Caller) -> Option<String> {
        let id = self.session_holding(caller)?;
        let (session, hosting) = self.session_and_hosting(&id).await.ok()?;
        match hosting.manifest_id.is_empty() {
            true => Some(session.cwd),
            false => self.repositories().serving(&hosting.manifest_id).map(|served| served.root().to_string()),
        }
    }

    /// `save_kit_manifest`: the person's own `armada.yml` for `repository`, whole, at the place their
    /// Kit keeps it.
    ///
    /// **Through the agent's door, only for the repository of the Job that calls.** A person's own
    /// copy wins over the repository's and can replace a Check, so an agent that could keep one
    /// for another repository could weaken that repository's gate. `caller` is the connection a
    /// door call arrived on; Bridge's own call has none and may keep one for any repository.
    pub(crate) async fn kit_manifest_saved(&self, caller: Option<Caller>, saving: SaveKitManifest) -> Result<KitManifestSaved, Refusal> {
        let refuse = |code: &str, said: String| {
            let raised = WireError::raised(code, said, self.run_id());
            match code {
                KIT_MANIFEST_UNWRITABLE => Refusal::Fault(raised),
                _ => Refusal::Unacceptable(raised),
            }
        };
        if let Some(caller) = caller {
            match self.calling_job_repository(&caller).await {
                None => return Err(refuse(KIT_MANIFEST_ELSEWHERE, String::from("Nothing was saved. Only a Job's own agent keeps an armada.yml in Kit from here, and this call comes from none."))),
                Some(mine) if mine.trim_end_matches('/') != saving.repository.trim_end_matches('/') => {
                    return Err(refuse(
                        KIT_MANIFEST_ELSEWHERE,
                        format!(
                            "Nothing was saved. This Job works in {mine}, so it keeps an armada.yml in Kit for that repository only, and {} is another one. Send `repository` as {mine}.",
                            saving.repository
                        ),
                    ));
                }
                Some(_) => {}
            }
        }
        let root = self.run_repository(&saving.repository)?;
        if let Some(why) = config::kit_copy_fault(&saving.text) {
            return Err(refuse(KIT_MANIFEST_REFUSED, format!("Nothing was saved. {why}")));
        }
        let held = self.kept_place(&root).await?;
        let copy = KitManifest::at(&self.kit(), &root, held.as_ref(), std::sync::Arc::clone(self.reaching()));
        let label = copy.label();
        if let KitRead::Unread(why) = copy.read() {
            return Err(refuse(KIT_MANIFEST_UNPLACED, format!("Nothing was saved, because {} could not be reached: {why}", label.display())));
        }
        let text = saving.text;
        let wrote = tokio::task::spawn_blocking(move || copy.write(&text)).await.unwrap_or_else(|why| Err(why.to_string()));
        wrote.map_err(|why| refuse(KIT_MANIFEST_UNWRITABLE, format!("{} could not be written: {why}", label.display())))?;
        Ok(KitManifestSaved { repository: root, file: label.display().to_string() })
    }

    /// `read_package`: every piece, and what adding it would do.
    pub(crate) async fn package_read(&self, asked: PackageText) -> Result<PackageRead, Refusal> {
        let pieces = package::read(&asked.text).map_err(|why| self.share_refusal(MALFORMED, why))?;
        let slugs = self.served_slugs().await;
        let mut read = Vec::new();
        for piece in &pieces {
            let repository = matching(&slugs, piece.remote());
            let exists = match &repository {
                Some(root) => self.held_in(piece, root).await,
                None => false,
            };
            let shown = piece.shown();
            read.push(PackagePiece {
                kind: shown.kind,
                id: shown.id,
                name: shown.name,
                list: shown.list,
                action: shown.action,
                commands: shown.commands,
                remote: shown.remote,
                repository,
                exists,
            });
        }
        Ok(PackageRead { pieces: read })
    }

    /// Whether the person's Kit holds a piece of this id for `root`.
    async fn held_in(&self, piece: &Piece, root: &str) -> bool {
        match piece {
            Piece::Tile { list, tile, .. } => match self.repository_flight_controls_read(root.to_string(), None).await.map(|read| read.read) {
                Ok(FlightControlsRead::Read { controls }) => tiles_of(&controls, *list).iter().any(|held| held.id == tile.id),
                _ => false,
            },
            Piece::Workflow { .. } => {
                let id = piece.shown().id;
                self.repositories()
                    .served()
                    .into_iter()
                    .find(|one| one.root() == root)
                    .is_some_and(|served| served.files().iter().any(|file| file.source() == WorkflowSource::Kit && file.id().as_str() == id))
            }
            Piece::ReviewTool { id, .. } => {
                self.review_tool_held(&self.review_tool_folder(ReviewToolScope::Kit, root), ReviewToolScope::Kit, id).is_some()
            }
            Piece::Manifest { .. } => matches!(self.kit_manifest_read(root).await, Ok((_, KitRead::Text(_)))),
            Piece::Requests { collections, environments, .. } => self.own_requests_file(root).await.is_ok_and(|own| {
                collections.iter().any(|one| own.collections.iter().any(|held| held.id == one.id))
                    || environments.iter().any(|one| own.environments.iter().any(|held| held.name == one.name))
            }),
        }
    }

    /// `add_package`: every piece into the person's Kit, or unbound where its remote is none's.
    pub(crate) async fn package_added(&self, asked: AddPackage) -> Result<PackageAdded, Refusal> {
        let pieces = package::read(&asked.text).map_err(|why| self.share_refusal(MALFORMED, why))?;
        let slugs = self.served_slugs().await;
        let placed = pieces.into_iter().map(|piece| { let at = matching(&slugs, piece.remote()); (piece, at) }).collect();
        Ok(PackageAdded { pieces: self.landed(placed, asked.replace).await? })
    }

    /// `list_unbound_pieces`.
    pub(crate) fn unbound_listed(&self) -> UnboundPieces {
        let pieces = unbound::all(&self.kit())
            .into_iter()
            .map(|(unbound_id, piece)| {
                let shown = piece.shown();
                UnboundPiece { unbound_id, kind: shown.kind, id: shown.id, name: shown.name, list: shown.list, commands: shown.commands, remote: shown.remote }
            })
            .collect();
        UnboundPieces { pieces }
    }

    /// `bind_unbound_piece`: the piece, once, onto the repository chosen.
    pub(crate) async fn unbound_bound(&self, asked: BindUnboundPiece) -> Result<PieceAdded, Refusal> {
        let Some(piece) = unbound::held(&self.kit(), &asked.unbound_id) else {
            return Err(self.share_refusal(NO_UNBOUND, format!("Nothing is kept unbound as `{}`.", asked.unbound_id)));
        };
        let root = self.run_repository(&asked.repository)?;
        let added = self.landed(vec![(piece, Some(root))], asked.replace).await?.remove(0);
        unbound::forget(&self.kit(), &asked.unbound_id)
            .map_err(|why| self.share_refusal(UNWRITABLE, format!("The unbound piece {} could not be removed: {why}", asked.unbound_id)))?;
        Ok(added)
    }

    /// Check every piece, then write each: workflows first, then review tools, tiles and unbound
    /// pieces. A workflow's checks are all made before the first write.
    async fn landed(&self, placed: Vec<(Piece, Option<String>)>, replace: bool) -> Result<Vec<PieceAdded>, Refusal> {
        let mut exists = Vec::new();
        for (piece, at) in &placed {
            let held = match at {
                Some(root) => self.held_in(piece, root).await,
                None => false,
            };
            if held && !replace {
                let shown = piece.shown();
                let place = shown.list.map_or_else(String::new, |list| format!(" in {}", list_word(list)));
                let whose = at.as_deref().map_or_else(String::new, |root| format!(" for {}", shown.remote.as_deref().unwrap_or(root)));
                return Err(self.share_refusal(EXISTS, format!("Your Kit already holds the {} `{}`{place}{whose}. Add with replace to replace it.", kind_word(shown.kind), shown.id)));
            }
            exists.push(held);
        }
        // A place that will not answer is found now, with nothing written yet.
        for (piece, at) in &placed {
            if let (Piece::Manifest { .. }, Some(root)) = (piece, at) {
                if let (label, KitRead::Unread(why)) = self.kit_manifest_read(root).await? {
                    return Err(self.share_refusal(REFUSED, format!("Your armada.yml for {root} cannot be added, because {} did not read: {why}", label.display())));
                }
            }
            if let (Piece::Requests { .. }, Some(root)) = (piece, at) {
                if let Err(why) = self.own_requests_file(root).await {
                    return Err(self.share_refusal(REFUSED, format!("Your requests for {root} cannot be added, because they did not read: {why}")));
                }
            }
        }
        let tiles = self.tiles_merged(&placed).await?;

        let mut workflows = Vec::new();
        for ((piece, at), held) in placed.iter().zip(&exists) {
            if let (Piece::Workflow { text, .. }, Some(root)) = (piece, at) {
                let served = self.repositories().served().into_iter().find(|one| one.root() == root.as_str());
                let served = served.ok_or_else(|| self.share_refusal(REFUSED, format!("Fleet does not serve {root}")))?;
                let asked = SaveWorkflow { scope: WorkflowScope::Kit, definition: text.clone(), overwrite: *held };
                self.check_workflow_file(&asked, &served)?;
                workflows.push((asked, served));
            }
        }
        for (asked, served) in workflows {
            self.save_workflow_file(asked, &served)?;
        }
        for (piece, at) in &placed {
            if let (Piece::ReviewTool { id, text, .. }, Some(root)) = (piece, at) {
                let folder = self.review_tool_folder(ReviewToolScope::Kit, root);
                let path = self.review_tool_held(&folder, ReviewToolScope::Kit, id).unwrap_or_else(|| folder.join(format!("{id}.md")));
                write_over(&path, text).map_err(|why| self.share_refusal(UNWRITABLE, format!("{} could not be written: {why}", path.display())))?;
            }
        }
        for (piece, at) in &placed {
            if let (Piece::Manifest { text, .. }, Some(root)) = (piece, at) {
                self.manifest_written(root, text.clone()).await?;
            }
        }
        for (piece, at) in &placed {
            if let (Some(adding), Some(root)) = (piece.requests_file(), at) {
                self.requests_added(root, &adding).await?;
            }
        }
        for (root, controls) in tiles {
            self.repository_flight_controls_saved(SaveRepositoryFlightControls { repository: root, controls }).await?;
        }
        let mut added = Vec::new();
        for ((piece, at), held) in placed.into_iter().zip(exists) {
            let shown = piece.shown();
            added.push(match at {
                Some(root) => added_of(shown, Some(root), if held { Landed::Replaced } else { Landed::Added }, None),
                None => {
                    let id = unbound::keep(&self.kit(), &piece).map_err(|why| self.share_refusal(UNWRITABLE, why))?;
                    added_of(shown, None, Landed::Unbound, Some(id))
                }
            });
        }
        Ok(added)
    }

    /// Each repository's tiles with the package's tiles put in, read and checked now.
    async fn tiles_merged(&self, placed: &[(Piece, Option<String>)]) -> Result<BTreeMap<String, FlightControls>, Refusal> {
        let mut merged: BTreeMap<String, FlightControls> = BTreeMap::new();
        for (piece, at) in placed {
            let (Piece::Tile { list, tile, .. }, Some(root)) = (piece, at) else { continue };
            if !merged.contains_key(root) {
                let controls = match self.repository_flight_controls_read(root.clone(), None).await?.read {
                    FlightControlsRead::Read { controls } => controls,
                    FlightControlsRead::NotRead { why } => return Err(self.share_refusal(REFUSED, format!("Your tiles for {root} did not read: {why}"))),
                };
                merged.insert(root.clone(), controls);
            }
            let controls = merged.get_mut(root).expect("inserted above");
            match list {
                TileList::Job => put(&mut controls.job, tile.clone()),
                TileList::Deck => put(&mut controls.deck, tile.clone()),
            }
        }
        for (root, controls) in &merged {
            check_repository(controls).map_err(|why| self.share_refusal(REFUSED, format!("Your tiles for {root} would break a rule: {why}")))?;
        }
        Ok(merged)
    }
}

fn tiles_of(controls: &FlightControls, list: TileList) -> &[FlightControl] {
    match list {
        TileList::Job => &controls.job,
        TileList::Deck => &controls.deck,
    }
}

fn list_word(list: TileList) -> &'static str {
    match list {
        TileList::Job => "job",
        TileList::Deck => "deck",
    }
}

fn kind_word(kind: PieceKind) -> &'static str {
    match kind {
        PieceKind::Tile => "tile",
        PieceKind::Workflow => "workflow",
        PieceKind::ReviewTool => "review tool",
        PieceKind::Manifest => "manifest",
        PieceKind::Requests => "requests",
    }
}
