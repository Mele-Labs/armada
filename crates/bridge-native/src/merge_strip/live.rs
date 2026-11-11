//! The strip on a live Fleet: the lines of the title bar's pick, from Fleet's merge line view, and what
//! each of the strip's acts asks of Fleet and the rest of the app (`Deck.tsx` `MergeQueue`, `merge-line.tsx`).

use gpui::{App, Context, Entity, SharedString, Subscription};
use ipc::{MergeLineView, RepositorySummary, SessionRecord};

use super::{Asked, Line, MergeStrip, Recent, Strip};
use crate::fleet::{FleetConnection, Held};
use crate::jobs::JobsList;
use crate::link::LinkState;
use crate::log::Note;

/// Where a live strip reads from.
pub(super) struct Source {
    fleet: Entity<FleetConnection>,
    list: Entity<JobsList>,
    _watch: Vec<Subscription>,
}

impl Source {
    /// A strip on mock Jobs has no Fleet, and keeps what its walk gives it.
    pub(super) fn of(list: &Entity<JobsList>, cx: &mut Context<MergeStrip>) -> Option<Source> {
        let fleet = list.read(cx).fleet()?;
        let mut watch = vec![
            cx.observe(&fleet, |strip, _, cx| strip.follow(cx)),
            cx.observe(list, |strip, _, cx| strip.follow(cx)),
            cx.subscribe(&cx.entity(), |strip, _, asked: &Asked, cx| strip.act(asked.clone(), cx)),
        ];
        if let Some(bar) = crate::titlebar::live(cx) {
            watch.push(cx.observe(&bar, |strip, _, cx| strip.follow(cx)));
        }
        Some(Source { fleet, list: list.clone(), _watch: watch })
    }
}

impl MergeStrip {
    /// Fleet's answer, the pick or the open Job moved: the strip draws what they make now.
    pub(super) fn follow(&mut self, cx: &mut Context<Self>) {
        let Some(source) = &self.source else { return };
        let opened = source.list.read(cx).opened_id();
        let held = source.fleet.read(cx).held();
        let data = strip_of(held, picked_roots(held, cx), opened);
        if data != self.data {
            self.set(data, cx);
        }
    }

    fn act(&mut self, asked: Asked, cx: &mut Context<Self>) {
        let Some(source) = &self.source else { return };
        let held = source.fleet.read(cx).held();
        let port = held.port();
        match asked {
            // Electron opens its side panel; until this app has one, the forge's page.
            Asked::OpenPull { root, number } => {
                let address = held.merge_lines.iter().flatten().filter(|line| line.root == root).flat_map(|line| &line.pulls).find(|view| view.pull.number == number).map(|view| view.pull.url.clone());
                if let Some(address) = address.filter(|address| !address.is_empty()) {
                    cx.open_url(&address);
                }
            }
            Asked::OpenJob(id) => {
                let list = source.list.clone();
                list.update(cx, |list, cx| list.open(id, cx));
            }
            Asked::RefreshPull { root, number } => {
                let key = format!("pull:{root}#{number}");
                match manifest_of(held.repositories.as_deref().unwrap_or_default(), &root) {
                    Some(manifest) => self.ask(key, port, format!("/pull_requests/{}/{number}/refresh", crate::link::percent_encoded(&manifest)), None, cx),
                    None => self.refuse(key, "This repository has no Manifest, so Fleet cannot refresh its pull requests.".into(), cx),
                }
            }
            // Take over is the same new Job, as Electron's is: it never asks Fleet for the claim.
            Asked::DispatchFix { root, request } | Asked::TakeOver { root, request } => {
                let repository = held.repositories.as_deref().unwrap_or_default().iter().find(|one| one.root == root).map(name_of);
                self.fixing = None;
                crate::dispatch::screen::open_drafted(request, repository, cx);
                cx.notify();
            }
            Asked::SendBack { root, session, message } => {
                let body = ipc::encode(&ipc::SendSessionMessageBody {
                    message: ipc::SendSessionMessage { session_id: ipc::SessionId::carried(session.to_string()), text: message, attachments: Vec::new(), mentions: Vec::new() },
                    aim: None,
                })
                .map_err(|why| format!("{why:?}"));
                let key = format!("main:{root}");
                match body {
                    Ok(body) => self.ask(key, port, "/sessions/message".into(), Some(body), cx),
                    Err(why) => self.refuse(key, why, cx),
                }
            }
        }
    }
}

impl MergeStrip {
    /// Ask Fleet off the main thread. A refusal is drawn on the chip or light `key` names, in Fleet's words,
    /// where Electron raises a toast; asking again clears it.
    fn ask(&mut self, key: String, port: Option<u16>, path: String, body: Option<String>, cx: &mut Context<Self>) {
        let key = SharedString::from(key);
        self.refused.remove(&key);
        cx.notify();
        let Some(port) = port else { return self.refuse(key.to_string(), "Fleet is not connected.".into(), cx) };
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(async {
                        match body {
                            Some(body) => crate::transport::post(port, &path, body).await,
                            None => crate::transport::post_bare(port, &path).await,
                        }
                    })?;
                    if (200..300).contains(&status) {
                        Ok(())
                    } else {
                        Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
                    }
                })
                .await;
            if let Err(why) = answer {
                let _ = this.update(cx, |strip, cx| strip.refuse(key.to_string(), why, cx));
            }
        })
        .detach();
    }

    fn refuse(&mut self, key: String, why: String, cx: &mut Context<Self>) {
        Note::warn("Fleet refused the strip's act").field("on", key.clone()).field("why", why.clone()).emit();
        self.refused.insert(key.into(), why.into());
        cx.notify();
    }
}

/// The repositories of the title bar's pick, by root; `None` on All. The bar lists Fleet's repositories in
/// Fleet's order, so its places are places in Fleet's list.
fn picked_roots(held: &Held, cx: &App) -> Option<Vec<String>> {
    let served = held.repositories.as_deref()?;
    let bar = crate::titlebar::live(cx)?;
    let face = &bar.read(cx).face;
    let at: Vec<usize> = match face.scoped.and_then(|at| face.projects.get(at)) {
        Some(project) => project.members.iter().map(|member| member.repository).collect(),
        None => vec![face.chosen?],
    };
    Some(at.into_iter().filter_map(|at| served.get(at)).map(|one| one.root.clone()).collect())
}

/// What the strip draws of what Fleet holds: the lines of the pick, the open Job's pull requests ahead of
/// the rest, and the live Jobs a red main's work can go back to.
fn strip_of(held: &Held, picked: Option<Vec<String>>, opened: Option<SharedString>) -> Strip {
    let served = held.repositories.as_deref().unwrap_or_default();
    let lines: Vec<MergeLineView> = held.merge_lines.iter().flatten().filter(|line| picked.as_ref().is_none_or(|roots| roots.contains(&line.root))).cloned().collect();
    let open = opened.as_ref().and_then(|id| held.jobs.iter().find(|job| job.id.as_str() == id.as_ref()));
    let related = opened
        .as_ref()
        .map(|id| lines.iter().flat_map(|line| &line.pulls).filter(|view| view.held_by.as_ref().is_some_and(|job| job.id == id.as_ref())).map(|view| view.pull.number).collect())
        .unwrap_or_default();
    let selected_root = open.and_then(|job| job.manifest_id.as_ref()).and_then(|manifest| served.iter().find(|one| manifest_id(one).as_deref() == Some(manifest.as_str()))).map(|one| one.root.clone());
    // Placeholder chips while a connected Fleet has not answered; a Fleet that is down draws what it last said.
    let loading = held.merge_lines.is_none() && matches!(held.state, LinkState::Connected);
    Strip {
        lines: lines.into_iter().map(|view| Line { view }).collect(),
        calls: held.calls.as_ref().map(|calls| calls.calls.clone()).unwrap_or_default(),
        on_all: picked.is_none(),
        selected_root,
        related,
        loading,
        recent: recent(&held.jobs),
    }
}

/// The live Jobs, the one heard from last first.
fn recent(jobs: &[SessionRecord]) -> Vec<Recent> {
    let mut live: Vec<&SessionRecord> = jobs.iter().filter(|job| job.ended_at.is_none()).collect();
    live.sort_by(|a, b| b.last_seen_at.as_str().cmp(a.last_seen_at.as_str()));
    live.into_iter()
        .map(|job| Recent { id: job.id.as_str().to_string().into(), title: job.title.clone().unwrap_or_else(|| job.id.as_str().to_string()).into(), branch: None })
        .collect()
}

/// The Manifest id Fleet names a served repository by, else the one it gives one with none.
fn manifest_id(one: &RepositorySummary) -> Option<String> {
    one.manifest.as_ref().map(|manifest| manifest.id.as_str().to_string()).or_else(|| one.implicit_id.as_ref().map(|id| id.as_str().to_string()))
}

fn manifest_of(served: &[RepositorySummary], root: &str) -> Option<String> {
    served.iter().find(|one| one.root == root).and_then(manifest_id)
}

/// The name the dispatch sheet's chips carry, as the title bar names a repository.
fn name_of(one: &RepositorySummary) -> String {
    one.manifest.as_ref().map_or_else(|| one.root.rsplit('/').find(|part| !part.is_empty()).unwrap_or(&one.root).to_string(), |manifest| manifest.repository.clone())
}

/// A walk's scene: Fleet's answers as the live app folds them, and the strip it reads of them. `lines` is
/// `None` where Fleet has not answered for the merge line; `down` is a Fleet that dropped after it did.
pub(super) fn walked(lines: Option<Vec<MergeLineView>>, picked: Option<Vec<String>>, opened: Option<&str>, down: bool) -> Strip {
    use crate::link::Signal;
    let mut held = Held::new();
    let mut said = vec![
        Signal::State(LinkState::Connected),
        Signal::Repositories(ipc::RepositoryList { repositories: crate::mock::repositories() }),
        Signal::Jobs(crate::mock::jobs("jobs-many").unwrap_or_default()),
    ];
    said.extend(lines.map(|lines| Signal::MergeLines(ipc::MergeLineViews { lines })));
    if down {
        said.push(Signal::State(LinkState::NotRunning("no runtime file".into())));
    }
    held.fold(said);
    strip_of(&held, picked, opened.map(|id| SharedString::from(id.to_string())))
}
