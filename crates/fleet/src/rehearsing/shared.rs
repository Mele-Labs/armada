//! What a run is, once whose tree it is in has been settled.
//!
//! **Every function here takes a [`Place`].** Which tree, whose ports and where
//! the records go were decided in [`owner`](super::owner); what is left is one
//! command in one directory.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Timestamp;
use ipc::WireError;
use tokio::sync::watch;

use super::entries::{self, Entry};
use super::owner::{Owner, Place, Tree};
use super::record::{Record, Underway};
use super::{records, running, Seen, Unrehearsable, STOPPING};
use crate::daemon::Fleet;

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
    /// When the Manifest was last changed by a commit made at or before
    /// `before`.
    pub(super) fn edited_before(
        &self,
        place: &Place,
        before: &Timestamp,
        tree: &Tree,
    ) -> Option<ipc::Instant> {
        let before = before.epoch_millis()?.div_euclid(1_000);
        let root = tree.path.clone();
        let file = self.manifest_file(place.checkout.served()?);
        let seconds =
            adapters::snapshot::last_touched(&root, &file.to_string_lossy(), before).ok()??;
        let at = Timestamp::from_rfc3339(crate::clock::rfc3339_utc(seconds.saturating_mul(1_000)));
        Some(ipc::Instant::from(&at))
    }

    /// The entry a run names and the tree it would run in, checked in the
    /// order that refuses before anything is written.
    pub(super) fn entry_at(
        &self,
        place: &Place,
        name: &str,
    ) -> Result<(Entry, Tree), Unrehearsable> {
        // A server never exits, so a run of one would hold the owner's one run
        // slot for good — and Fleet would not know to hand it on or stop it.
        let Some(manifest) = self.manifest_at(place) else {
            return Err(Unrehearsable::NoManifest);
        };
        if manifest.server(name).is_some() {
            return Err(Unrehearsable::IsAServer {
                name: name.to_string(),
            });
        }
        let tree = self.tree_at(place);
        if let Some(out) = self.rehearsals().in_flight(&place.owner) {
            return Err(Unrehearsable::AlreadyRunning { name: out.name });
        }
        Ok((entries::declared(manifest).named(name)?, tree))
    }

    pub(super) fn history_at(&self, place: &Place) -> (Vec<Record>, Vec<ipc::UnreadableRun>) {
        let running = self.rehearsals().in_flight(&place.owner).map(|out| out.id);
        records::every(
            place.checkout.records_root(),
            &place.handle,
            running.as_deref(),
        )
    }

    pub(super) fn no_such_run(&self, place: &Place, id: String) -> Refusal {
        self.refused_run(
            &place.owner,
            Unrehearsable::NoSuchRun { id },
        )
    }

    pub(super) fn observed_at(&self, place: &Place, id: &str) -> Option<Seen> {
        let (root, handle) = (place.checkout.records_root(), place.handle.as_str());
        let (name, live) = match self.rehearsals().watching(&place.owner, id) {
            Some((name, watch)) => (name, Some(watch)),
            None => match records::read(root, handle, id) {
                Some(Ok(record)) => (record.name, None),
                _ => return None,
            },
        };
        let log = records::dir_of(root, handle, id)?.join(records::LOG);
        let (history, skipped, read_to, unreadable) = match records::history(&log, live.is_none()) {
            Some((history, skipped, read_to)) => (history, skipped, read_to, false),
            None => (Vec::new(), 0, 0, true),
        };
        Some(Seen {
            id: id.to_string(),
            name,
            path: records::relative_log(handle, id),
            live,
            history,
            skipped,
            read_to,
            unreadable,
        })
    }

    /// One run's log. **The owner's own runs are the allowlist**: an id that
    /// names none of them reaches no file.
    pub(super) fn output_at(&self, place: &Place, id: &str) -> Option<ipc::RunOutput> {
        let (root, handle) = (place.checkout.records_root(), place.handle.as_str());
        let name = match self
            .rehearsals()
            .in_flight(&place.owner)
            .filter(|out| out.id == id)
        {
            Some(out) => Some(out.name),
            None => match records::read(root, handle, id) {
                Some(Ok(record)) => Some(record.name),
                _ => None,
            },
        };
        name.and_then(|name| records::output(root, handle, id, name))
    }

    pub(super) fn refused_run(&self, owner: &Owner, why: Unrehearsable) -> Refusal {
        let (code, refusal) = why.spelled();
        let raised = WireError::raised(code, why.to_string(), self.run_id());
        let raised = match why.faults() {
            Some(faults) => raised.with_field("faults", faults),
            None => raised,
        };
        let Owner::Checkout(_) = owner;
        refusal(raised)
    }

    /// Take the owner's one run slot, make the run's directory, and spawn it.
    /// `verify` is the Verify this run is a step of, where it is one.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn started_at(
        self: Arc<Self>,
        place: Place,
        tree: Tree,
        entry: Entry,
        command: String,
        requester: ipc::Requester,
        verify: Option<tokio::sync::oneshot::Sender<super::verifying::Handed>>,
    ) -> Result<Underway, Unrehearsable> {
        let underway = Underway {
            id: self.mint().ulid().as_str().to_string(),
            name: entry.name.clone(),
            command,
            started_at: ipc::Instant::from(&self.now()),
            workspace: tree
                .within
                .as_ref()
                .map(|within| within.dir.to_string_lossy().into_owned()),
            requester,
        };
        let (stop, stopped) = watch::channel(false);
        let (done, ended) = watch::channel(None);
        let feed = api::RunFeed::new();
        let Some(held) = self
            .rehearsals()
            .take(&place.owner, &underway, stop, ended, feed.clone())
        else {
            return Err(Unrehearsable::AlreadyRunning { name: entry.name });
        };
        let (root, handle) = (place.checkout.records_root(), place.handle.clone());
        // Retention comes for a run before this one starts, and a Studio may
        // be holding one of them: what each says is kept on its Run node here,
        // while the log is still on disk to read. `crate::studio_runs`.
        let due = records::due(root, &handle, &self.now(), self.run_log_retention());
        let going: Vec<ipc::CheckoutRunRecord> = due.iter().map(Record::of_checkout).collect();
        let held_back = self.kept_what_studios_hold(root, &handle, &going).await;
        records::sweep(root, &handle, &due, &held_back, |reference| {
            let _ = adapters::snapshot::forget(&tree.path, reference);
        });
        let dir =
            records::made(root, &handle, &underway.id).map_err(|why| Unrehearsable::NotKept {
                why: why.to_string(),
            })?;
        let plan = running::Plan {
            place,
            tree,
            entry,
            underway: underway.clone(),
            dir,
            feed,
            verify,
        };
        let this = Arc::clone(&self);
        tokio::spawn(async move { this.rehearsed(plan, held, stopped, done).await });
        Ok(underway)
    }

    pub(super) async fn stopped_at(
        &self,
        place: &Place,
        id: String,
    ) -> Result<Record, Unrehearsable> {
        let (root, handle) = (place.checkout.records_root(), place.handle.as_str());
        let Some((stop, mut done)) = self.rehearsals().stopping(&place.owner, &id) else {
            return Err(match records::read(root, handle, &id) {
                Some(Ok(_)) => Unrehearsable::NotRunning { id },
                _ => Unrehearsable::NoSuchRun { id },
            });
        };
        let _ = stop.send(true);
        let ended = tokio::time::timeout(STOPPING, done.wait_for(Option::is_some)).await;
        match ended {
            Ok(Ok(seen)) => seen.clone(),
            _ => None,
        }
        .ok_or_else(|| Unrehearsable::NotKept {
            why: String::from("the run did not write its record after it was stopped"),
        })
    }

    pub(super) async fn undone_at(
        &self,
        place: &Place,
        id: String,
    ) -> Result<Record, Unrehearsable> {
        if self.rehearsals().in_flight(&place.owner).is_some() {
            return Err(Unrehearsable::RunInFlight);
        }
        let tree = self.tree_at(place);
        let (root, handle) = (place.checkout.records_root(), place.handle.as_str());
        let mut record = match records::read(root, handle, &id) {
            Some(Ok(record)) => record,
            Some(Err(why)) => return Err(Unrehearsable::NothingToUndo { id, why }),
            None => return Err(Unrehearsable::NoSuchRun { id }),
        };
        let nothing = |why: &str| Unrehearsable::NothingToUndo {
            id: record.id.clone(),
            why: why.to_string(),
        };
        if record.undone_at.is_some() {
            return Err(Unrehearsable::AlreadyUndone { id });
        }
        if record.changed.is_empty() {
            return Err(nothing("the run changed nothing"));
        }
        // **Never offered where no snapshot was taken.** This can be the tree
        // holding a person's own uncommitted work, and nothing puts that back
        // without a copy of what was there before.
        let Some(reference) = record.snapshot.clone() else {
            return Err(nothing(
                record
                    .changed_unreadable
                    .as_deref()
                    .unwrap_or("no snapshot was kept"),
            ));
        };
        let path = tree.path.clone();
        let undone =
            tokio::task::spawn_blocking(move || adapters::snapshot::undo(&path, &reference)).await;
        match undone {
            Ok(Ok(_)) => {}
            Ok(Err(adapters::snapshot::NotUndone::Moved { paths })) => {
                return Err(Unrehearsable::Moved { paths })
            }
            Ok(Err(adapters::snapshot::NotUndone::NotRestored { path, cause })) => {
                return Err(Unrehearsable::NotKept {
                    why: format!("{path} could not be written back: {cause}"),
                })
            }
            Ok(Err(other)) => return Err(nothing(&other.to_string())),
            Err(_) => return Err(nothing("the undo did not finish")),
        }
        record.undone_at = Some(ipc::Instant::from(&self.now()));
        let kept = records::dir_of(root, handle, &record.id)
            .map(|dir| records::write(&dir, &record))
            .unwrap_or_else(|| Err(std::io::Error::other("not one path component")));
        kept.map_err(|why| Unrehearsable::NotKept {
            why: format!("the tree was put back and the record was not: {why}"),
        })?;
        Ok(record)
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
    /// Where the Manifest Fleet holds sits, relative to the repository, so the
    /// same file can be read in a worktree and in git.
    pub(super) fn manifest_file(&self, served: &crate::repositories::Served) -> PathBuf {
        let held = served.manifest().path();
        held.strip_prefix(served.root())
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| {
                held.file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("armada.yml"))
            })
    }
}
