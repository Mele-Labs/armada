//! A native walk built from the Job's own commit, so the screen he is asked about is the one the
//! Job made. `docs/concepts/session/showing-a-window.md`, *A native walk*.
//!
//! **What a build runs is the repository's own `walks:`** (`config::Walks`), read from the Manifest
//! Fleet serves it under: the native Bridge is one declaration of it, and another repository's app
//! is built, listed, opened and pictured the same way.
//!
//! **One warm tree per repository**, a detached `git worktree` under Fleet's support folder
//! checked out at each commit in turn, and the folder `walks.seed` names first cloned from the preview
//! checkout's so the first build compiles the Job's crates and not the world's. **One build at a time across
//! Fleet**, in a queue that keeps one ask for each Job and address: a newer commit replaces a queued
//! older one. The compilers carry the Job's `ARMADA_SESSION`, so a loaded machine holds them in the
//! turns `crate::build_turns` takes.
//!
//! **Each build's binary is copied out to `bins/<commit>`**, the last [`KEPT`] of a repository, so a
//! walk that is running is never written over by the next build.
//!
//! **The row carries where it stands** (`ipc::WalkBuild`). Building and Failed ask him nothing,
//! and a Failed one is told to the Job, never to him.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::leasing::Holder;
use config::Walks;
use ipc::{SessionRow, WalkBuild, WalkBuildState};

use crate::approved_looks::tail_of;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// Under Fleet's support folder: `<repository key>/tree` and `<repository key>/bins/<commit>`.
pub(crate) const DIR: &str = "walk-builds";

/// How many builds of a repository are kept.
const KEPT: usize = 3;

/// What a repository with no `walks:` is told: one sentence, naming the field.
pub(crate) const NO_WALKS: &str = "a native walk needs `walks:` in this repository's armada.yml, and it declares none";

/// What a build may take, held turns included: a cold build on a loaded machine is long.
const BUDGET: Duration = Duration::from_secs(90 * 60);

/// Where one repository's walk builds are kept.
fn root_of(home: &str, key: &str) -> PathBuf {
    crate::runtime::support_dir(home).join(DIR).join(key)
}

/// What a repository's `walks:` says, or the native Bridge's where it declares none and holds the native Bridge.
/// Goes once armada.yml declares `walks:` itself.
fn walks_of(served: &Served) -> Option<Walks> {
    let holds = || Path::new(served.root()).join("crates/bridge-native/Cargo.toml").is_file();
    served.manifest().walks().cloned().or_else(|| holds().then(Walks::native_bridge))
}

/// The file name a build's program is kept under: the one `walks.binary` ends in.
fn kept_name(walks: &Walks) -> &str {
    Path::new(walks.binary()).file_name().and_then(|name| name.to_str()).unwrap_or("walk")
}

/// The binary of `commit`'s build, where it is kept.
pub(crate) fn binary_of(home: &str, key: &str, commit: &str, walks: &Walks) -> PathBuf {
    root_of(home, key).join("bins").join(commit).join(kept_name(walks))
}

/// A declared command as the words to run, **split first and filled after**, so a value holding a
/// space is still one word. Each `${name}` in `given` is replaced by its value.
pub(crate) fn words_of(command: &str, given: &[(&str, &str)]) -> Result<Vec<String>, String> {
    let (program, args) = checks_runner::split(command).ok_or_else(|| "a command in `walks:` of armada.yml is empty".to_string())?;
    let fill = |word: String| given.iter().fold(word, |word, (name, value)| word.replace(&format!("${{{name}}}"), value));
    Ok(std::iter::once(program).chain(args).map(fill).collect())
}

/// A repository's name for a folder: its Manifest's id, with anything a path would read as more kept out.
pub(crate) fn key_of(manifest: &str) -> String {
    manifest.chars().map(|one| if one.is_ascii_alphanumeric() || one == '-' || one == '_' { one } else { '_' }).collect()
}

/// Where a Job's commit is, and where its repository's builds are kept.
#[derive(Clone, Debug)]
pub(crate) struct Place {
    pub(crate) key: String,
    /// The Job's slot: git runs there, so the tree is a worktree of the Job's own repository.
    pub(crate) slot: PathBuf,
    /// The checkout whose `walks.seed` folder seeds a new tree's.
    pub(crate) preview: PathBuf,
    /// How this repository's walks are built, as its Manifest declares them.
    pub(crate) walks: Walks,
}

/// One build asked for: the Job's walk at one commit.
#[derive(Clone, Debug)]
pub(crate) struct Ask {
    pub(crate) session: String,
    pub(crate) url: String,
    pub(crate) commit: String,
    pub(crate) place: Place,
    /// Set where no row waits on the build: `crate::approved_looks_checking`.
    pub(crate) answer: Option<crate::approved_looks_checking::Answer>,
}

#[derive(Default)]
struct Queue {
    waiting: Vec<Ask>,
    running: Option<Ask>,
    /// What a Job is owed telling, where it could not be told as it happened.
    owed: Vec<(String, String, ipc::Notice)>,
}

/// Every walk build Fleet owes and the one it is making. Never written down: **a Fleet that starts
/// asks again for every Building row of a live Job** on its first look, so a restart mid-build never
/// leaves a walk reading Building with nothing building it.
pub(crate) struct WalkBuilds {
    queue: Mutex<Queue>,
    resumed: std::sync::atomic::AtomicBool,
}

impl Default for WalkBuilds {
    fn default() -> WalkBuilds {
        WalkBuilds { queue: Mutex::default(), resumed: std::sync::atomic::AtomicBool::new(false) }
    }
}

impl WalkBuilds {
    /// Ask for a build. **A newer commit of the same Job and address replaces one waiting**, and the
    /// one running is left to end: its row is no longer the commit wanted, so its result is dropped.
    pub(crate) fn asked(&self, ask: Ask) {
        let mut queue = self.queue.lock().expect("not poisoned");
        queue.waiting.retain(|one| !one.replaced_by(&ask));
        let running = queue.running.as_ref().is_some_and(|one| one.replaced_by(&ask) && one.commit == ask.commit);
        if !running {
            queue.waiting.push(ask);
        }
    }

    /// The next to build, once none is.
    fn next(&self) -> Option<Ask> {
        let mut queue = self.queue.lock().expect("not poisoned");
        if queue.running.is_some() || queue.waiting.is_empty() {
            return None;
        }
        let ask = queue.waiting.remove(0);
        queue.running = Some(ask.clone());
        Some(ask)
    }

    #[cfg(test)]
    pub(crate) fn next_for_test(&self) -> Option<Ask> {
        self.next()
    }

    #[cfg(test)]
    pub(crate) fn done_for_test(&self) {
        self.done();
    }

    fn done(&self) {
        self.queue.lock().expect("not poisoned").running = None;
    }

    /// A Job to tell at the next look, for a failure found where Fleet holds no `Arc` of itself.
    pub(crate) fn owed(&self, session: &str, text: String, notice: ipc::Notice) {
        self.queue.lock().expect("not poisoned").owed.push((session.to_string(), text, notice));
    }

    fn owed_taken(&self) -> Vec<(String, String, ipc::Notice)> {
        std::mem::take(&mut self.queue.lock().expect("not poisoned").owed)
    }

    /// Whether this Job and address has a build waiting or running.
    #[cfg(test)]
    pub(crate) fn busy(&self) -> bool {
        let queue = self.queue.lock().expect("not poisoned");
        queue.running.is_some() || !queue.waiting.is_empty()
    }
}

/// `git <args>` in `at`, at utility priority; its trimmed output, or what it said where it failed.
fn git(at: &Path, args: &[&str]) -> Result<String, String> {
    let done = checks_runner::Priority::Low
        .spawning("git")
        .args(args)
        .current_dir(at)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|why| format!("git could not run: {why}"))?;
    if done.status.success() {
        return Ok(String::from_utf8_lossy(&done.stdout).trim().to_string());
    }
    Err(format!("git {} said: {}", args.join(" "), String::from_utf8_lossy(&done.stderr).trim()))
}

/// The commit a slot stands on.
pub(crate) fn head_of(slot: &Path) -> Result<String, String> {
    git(slot, &["rev-parse", "HEAD"])
}

/// The repository's tree at `commit`, made as a worktree the first time. **A new tree's `walks.seed`
/// folder is an APFS clone of the preview checkout's**, where it has one: a clone costs no copying,
/// and what it holds of the dependencies is not built again.
fn tree_at(root: &Path, place: &Place, commit: &str) -> Result<PathBuf, String> {
    let tree = root.join("tree");
    if tree.join(".git").exists() {
        git(&tree, &["checkout", "--detach", "--force", "--quiet", commit])?;
        return Ok(tree);
    }
    std::fs::create_dir_all(root).map_err(|why| format!("{} could not be made: {why}", root.display()))?;
    let _ = std::fs::remove_dir_all(&tree);
    let held = tree.display().to_string();
    git(&place.slot, &["worktree", "prune"])?;
    git(&place.slot, &["worktree", "add", "--detach", "--force", &held, commit])?;
    let Some(folder) = place.walks.seed() else { return Ok(tree) };
    let seed = place.preview.join(folder);
    let into = tree.join(folder);
    if seed.is_dir() && !into.exists() {
        // A volume that cannot clone leaves the tree to build cold, which is slow and not wrong.
        let _ = std::fs::create_dir_all(into.parent().expect("inside the tree"));
        // macOS clones with `-c`; GNU cp, as on CI, clones where it can with `--reflink=auto`.
        let clone = if cfg!(target_os = "macos") { "-c" } else { "--reflink=auto" };
        let _ = checks_runner::Priority::Low.spawning("cp").args([clone, "-R"]).arg(&seed).arg(&into).status();
    }
    Ok(tree)
}

/// [`tree_at`] for a test of it.
#[cfg(test)]
pub(crate) fn tree_at_for_test(root: &Path, place: &Place, commit: &str) -> Result<PathBuf, String> {
    tree_at(root, place, commit)
}

/// [`pruned`] for a test of it.
#[cfg(test)]
pub(crate) fn pruned_for_test(bins: &Path, just: &str) {
    pruned(bins, just);
}

/// Keep the newest [`KEPT`] builds of a repository, and `just` whatever its age.
fn pruned(bins: &Path, just: &str) {
    let Ok(read) = std::fs::read_dir(bins) else { return };
    let mut made: Vec<(std::time::SystemTime, PathBuf)> = read
        .filter_map(Result::ok)
        .filter(|one| one.file_name() != just)
        .filter_map(|one| Some((one.metadata().ok()?.modified().ok()?, one.path())))
        .collect();
    made.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, path) in made.into_iter().skip(KEPT - 1) {
        let _ = std::fs::remove_dir_all(path);
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
    /// Where the Job's commit is and where its repository's builds are kept, or why it has none:
    /// **committed work only**, so a Job that holds no slot has no commit of its own.
    pub(crate) async fn walk_place(&self, session: &str) -> Result<(Place, String), String> {
        let (_, hosting) = self.session_and_hosting(session).await.map_err(|_| "Fleet hosts no such Job".to_string())?;
        if hosting.lease_slot.is_none() {
            return Err("this Job holds no slot, so it has no commit of its own to build: write and commit its change in a slot first".to_string());
        }
        let served = self
            .repositories()
            .serving(&hosting.manifest_id)
            .ok_or_else(|| "this Job's repository is not served, so there is nothing to build it from".to_string())?;
        let walks = walks_of(&served).ok_or_else(|| NO_WALKS.to_string())?;
        let slot = PathBuf::from(Self::directory_of(&served, &hosting));
        let commit = tokio::task::spawn_blocking({
            let slot = slot.clone();
            move || head_of(&slot)
        })
        .await
        .map_err(|why| why.to_string())??;
        let preview = Path::new(served.root()).join(".armada/preview");
        Ok((Place { key: key_of(&hosting.manifest_id), slot, preview, walks }, commit))
    }

    /// The `walks:` the repository a Job belongs to declares, or `None` where it is not served or declares none.
    pub(crate) fn walks_declared(&self, manifest_id: &str) -> Option<Walks> {
        walks_of(&self.repositories().serving(manifest_id)?)
    }

    /// Why a native walk cannot be shown for this Job's repository, or `None`: it is served and
    /// declares no `walks:`, or the walk has takes and `walks.take` is not declared. **A Job whose
    /// repository is not served is left to the build's own answer**, which says why on the row.
    pub(crate) async fn walks_refused(&self, session: &str, takes: bool) -> Option<&'static str> {
        let (_, hosting) = self.session_and_hosting(session).await.ok()?;
        match walks_of(&self.repositories().serving(&hosting.manifest_id)?) {
            None => Some(NO_WALKS),
            Some(walks) if takes && walks.take().is_none() => Some("this repository's armada.yml declares no `walks.take`, so its native walks have no takes"),
            Some(_) => None,
        }
    }

    /// The build to write on a walk's row now, and the ask that makes it, **to queue once the row
    /// is out**: a build that ends before its row exists finds no walk wanting it.
    pub(crate) async fn walk_build_wanted(&self, session: &str, url: &str) -> (WalkBuild, Option<Ask>) {
        match self.walk_place(session).await {
            Ok((place, commit)) => {
                if self.walk_in_his_build(&place, &commit, url).await {
                    return (WalkBuild { state: WalkBuildState::Ready, commit, at: self.instant(), why: None }, None);
                }
                let build = WalkBuild { state: WalkBuildState::Building, commit: commit.clone(), at: self.instant(), why: None };
                (build, Some(Ask { session: session.to_string(), url: url.to_string(), commit, place, answer: None }))
            }
            Err(why) => {
                let text = format!("Your native walk {url} cannot be built: {why}. The person is not asked about it until it builds.");
                self.walk_builds().owed(session, text, ipc::Notice::WalkFailed);
                (WalkBuild { state: WalkBuildState::Failed, commit: String::new(), at: self.instant(), why: Some(why) }, None)
            }
        }
    }

    /// **Nothing to build where the build he runs already holds `commit` and plays the walk**: its
    /// binary is linked in as the build of `commit`, so the walk is Ready at once and opens from it. Only
    /// Armada's own commits are under his build, so another repository's walk is always built.
    async fn walk_in_his_build(&self, place: &Place, commit: &str, url: &str) -> bool {
        let Some(name) = crate::native_walks::walk_name(url) else { return false };
        if !self.build_holds(self.preview_build().as_ref(), "", commit) {
            return false;
        }
        let home = self.host().home.clone();
        let his = crate::native_walks::running_build(&home).join(crate::native_walks::APP);
        if !crate::native_walks::walks_listed(&place.walks, &his).await.is_ok_and(|known| known.iter().any(|one| one == name)) {
            return false;
        }
        let kept = binary_of(&home, &place.key, commit, &place.walks);
        if kept.is_file() {
            return true;
        }
        let linked = kept.parent().map(std::fs::create_dir_all).transpose().and_then(|_| std::fs::hard_link(&his, &kept).or_else(|_| std::fs::copy(&his, &kept).map(|_| ())));
        linked.is_ok()
    }

    /// The newest open native walk row on `url`, with the build it carries.
    pub(crate) async fn walk_row_of(&self, session: &str, url: &str) -> Option<(SessionRow, Vec<String>)> {
        let rows = self.rows_of(session).await.ok()?;
        let row = rows.into_iter().rev().find(|row| matches!(row, SessionRow::Window { url: shown, ended_at: None, build: Some(_), .. } if shown == url))?;
        let names = match &row {
            SessionRow::Window { url, variations, .. } => std::iter::once(url.as_str())
                .chain(variations.iter().map(|take| take.url.as_str()))
                .filter_map(crate::native_walks::walk_name)
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        };
        Some((row, names))
    }

    /// Set the build on the open row of `url`, **where it is still the build of `commit`**: a row that
    /// moved on to a newer commit is not written over by an older one's end.
    async fn walk_build_set(&self, session: &str, url: &str, was: Option<&str>, build: WalkBuild) -> bool {
        let Some((SessionRow::Window { id, at, title, url, variations, steps, origins, brief, group, build: held, ended_at }, _)) =
            self.walk_row_of(session, url).await
        else {
            return false;
        };
        if was.is_some_and(|commit| held.as_ref().map(|one| one.commit.as_str()) != Some(commit)) {
            return false;
        }
        let row = SessionRow::Window { id, at, title, url, variations, steps, origins, brief, group, build: Some(build), ended_at };
        self.row_put(session, row).await;
        let _ = self.published_hosted(session).await;
        true
    }

    /// Every open native walk of the Job whose build is not of the commit its slot stands on now is
    /// Building at that one, and asked for. **Run where its turn ends and where each build ends**, so
    /// no Job has to ask for a walk to be built again after it commits.
    pub(crate) async fn walk_builds_moved(&self, session: &str) {
        let Ok(rows) = self.rows_of(session).await else { return };
        let open: Vec<String> = rows
            .iter()
            .filter_map(|row| match row {
                SessionRow::Window { url, ended_at: None, build: Some(build), .. } if !build.commit.is_empty() => Some(url.clone()),
                _ => None,
            })
            .collect();
        if open.is_empty() {
            return;
        }
        let Ok((place, head)) = self.walk_place(session).await else { return };
        for url in open {
            let Some((SessionRow::Window { build: Some(held), .. }, _)) = self.walk_row_of(session, &url).await else { continue };
            if held.commit == head {
                // Building at this commit with nothing building it, as after a restart: asked again,
                // and `asked` leaves one already waiting or running alone.
                if held.state == WalkBuildState::Building {
                    self.walk_builds().asked(Ask { session: session.to_string(), url, commit: head.clone(), place: place.clone(), answer: None });
                }
                continue;
            }
            let building = WalkBuild { state: WalkBuildState::Building, commit: head.clone(), at: self.instant(), why: None };
            if self.walk_build_set(session, &url, None, building).await {
                self.walk_builds().asked(Ask { session: session.to_string(), url, commit: head.clone(), place: place.clone(), answer: None });
            }
        }
    }

    /// The walk's build is asked for again at the commit its slot stands on: its kept binary was pruned.
    pub(crate) async fn walk_rebuild_asked(&self, session: &str, url: &str) {
        let (build, ask) = self.walk_build_wanted(session, url).await;
        if self.walk_build_set(session, url, None, build).await {
            if let Some(ask) = ask {
                self.walk_builds().asked(ask);
            }
        }
    }

    /// One look of the notice loop: tell what is owed, and start the next build once none is going.
    pub(crate) async fn walk_builds_driven(self: &Arc<Self>) {
        if !self.walk_builds().resumed.swap(true, std::sync::atomic::Ordering::SeqCst) {
            Box::pin(self.walk_builds_resumed()).await;
        }
        for (session, text, notice) in self.walk_builds().owed_taken() {
            let _ = self.told_by_fleet(&session, &text, notice).await;
        }
        let Some(ask) = self.walk_builds().next() else { return };
        let fleet = Arc::clone(self);
        tokio::spawn(async move {
            fleet.walk_driven(&ask).await;
            // Before the turn is given up, so what moved while it built is asked for as this one ends.
            fleet.walk_builds_moved(&ask.session).await;
            fleet.walk_builds().done();
        });
    }

    /// Fleet started: every live Job's walks read again, and each still Building asked for again.
    pub(crate) async fn walk_builds_resumed(&self) {
        let live: Vec<String> = {
            let store = self.store().lock().await;
            store
                .find_sessions(&store::SessionSearch { state: Some(store::SessionState::Live), ..Default::default() })
                .unwrap_or_default()
                .into_iter()
                .map(|session| session.id.to_string())
                .collect()
        };
        for session in live {
            self.walk_builds_moved(&session).await;
        }
    }

    /// Build one ask, and write how it went on the walk's row. A walk that has gone, or moved to a
    /// newer commit, since it was asked for is not built.
    pub(crate) async fn walk_built(self: &Arc<Self>, ask: &Ask) {
        let Some((SessionRow::Window { build: Some(held), .. }, names)) = self.walk_row_of(&ask.session, &ask.url).await else { return };
        if held.commit != ask.commit || held.state != WalkBuildState::Building {
            return;
        }
        let short = ask.commit.get(..7).unwrap_or(&ask.commit).to_string();
        let result = match self.walk_compiled(ask).await {
            Ok(binary) => match crate::native_walks::walks_listed(&ask.place.walks, &binary).await {
                Ok(known) => match names.iter().find(|name| !known.contains(name)) {
                    Some(name) => Err(format!("the build of {short} has no walk named {name}: add the walk to the app and commit it")),
                    None => Ok(()),
                },
                Err(why) => Err(why),
            },
            Err(why) => Err(why),
        };
        let (state, why) = match result {
            Ok(()) => (WalkBuildState::Ready, None),
            Err(why) => (WalkBuildState::Failed, Some(why)),
        };
        let build = WalkBuild { state, commit: ask.commit.clone(), at: self.instant(), why: why.clone() };
        if !self.walk_build_set(&ask.session, &ask.url, Some(&ask.commit), build).await {
            return;
        }
        if let Some(why) = why {
            let text = format!(
                "Your native walk {} did not build at {short}: {why}. The person is not asked about it until it builds. \
                 Fix it and commit: it builds again from your branch.",
                ask.url
            );
            let _ = self.told_by_fleet(&ask.session, &text, ipc::Notice::WalkFailed).await;
        }
    }

    /// The binary of the ask's commit: kept already, or built in the repository's tree and copied out.
    pub(crate) async fn walk_compiled(&self, ask: &Ask) -> Result<PathBuf, String> {
        let home = self.host().home.clone();
        let walks = &ask.place.walks;
        let binary = binary_of(&home, &ask.place.key, &ask.commit, walks);
        if binary.is_file() {
            return Ok(binary);
        }
        let root = root_of(&home, &ask.place.key);
        let (place, commit) = (ask.place.clone(), ask.commit.clone());
        let tree = tokio::task::spawn_blocking({
            let root = root.clone();
            move || tree_at(&root, &place, &commit)
        })
        .await
        .map_err(|why| why.to_string())??;
        let mut words = words_of(walks.build(), &[])?.into_iter();
        let program = words.next().expect("a command has a program");
        let env = [
            ("PATH".to_string(), self.host().path.clone()),
            ("HOME".to_string(), home),
            // What a loaded machine reads to hold this Job's compilers in their turn.
            (Holder::SESSION_ENV.to_string(), ask.session.clone()),
        ];
        let attempt = checks_runner::run_words(
            program,
            words.collect(),
            &tree,
            BUDGET,
            checks_runner::Writing::Nowhere,
            &env,
            std::future::pending::<checks_runner::Stop>(),
            checks_runner::Priority::Low,
            checks_runner::GRACE,
        )
        .await;
        if attempt.exit != verification::Exit::Code(0) {
            return Err(format!("the build failed ({:?}): {}", attempt.exit, tail_of(&attempt.output.stderr, &attempt.output.stdout)));
        }
        let made = tree.join(walks.binary());
        let bins = root.join("bins");
        let held = bins.join(&ask.commit);
        std::fs::create_dir_all(&held).map_err(|why| format!("{} could not be made: {why}", held.display()))?;
        // Written under another name and moved in, so a binary is never seen half copied.
        let partial = held.join(format!("{}.partial", kept_name(walks)));
        std::fs::copy(&made, &partial).map_err(|why| format!("the build left no program at {}: {why}", made.display()))?;
        std::fs::rename(&partial, &binary).map_err(|why| format!("{} could not be kept: {why}", binary.display()))?;
        pruned(&bins, &ask.commit);
        Ok(binary)
    }
}
