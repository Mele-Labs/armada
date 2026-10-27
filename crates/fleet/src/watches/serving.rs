//! [`Watches`] for Fleet: a start checked and kept, a weighing and a stop on what is kept, each
//! kept in the store and published as `watches.changed`.

use std::collections::BTreeMap;
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Watches};
use config::settings as keys;
use core_model::{NotFilled, NotWeighed, Verdict, WatchState};
use ipc::{DeleteWatch, SessionId, StartWatch, StopWatch, WatchList, WatchStarts, WatchSummary, WatchVerdict, WeighWatch, WireError};
use store::{KeptWatch, KeptWatchSetAside, KeptWatchVerdict};

use super::running::Folder;
use super::telling::{owed, unsaid};
use super::*;
use crate::daemon::Fleet;
use crate::session_host::NO_SUCH_SESSION;

/// What a start comes to before it is kept: the command filled, and its two spans in seconds.
struct Resolved {
    command: Vec<String>,
    recipe: Option<String>,
    environment: Option<String>,
    every: u32,
    gives_up_after: Option<u32>,
    /// A terminal watch's text, and whether it is a pattern. Its command is empty.
    in_terminal: Option<(String, bool)>,
}

/// A recipe as a start or a try runs it: its command filled, and what else it names.
pub(super) struct Filled {
    pub(super) command: Vec<String>,
    pub(super) environment: Option<String>,
    pub(super) every: u32,
    pub(super) gives_up_after: Option<u32>,
}

/// The program a shipped recipe names, which is Fleet's own binary: an `armada` on the person's
/// `PATH` may be another build, or none.
const ARMADA: &str = "armada";

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
    pub(super) fn watch_unacceptable(&self, code: &str, why: impl Into<String>) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why.into(), self.run_id()))
    }

    pub(super) fn watch_illegal(&self, code: &str, why: impl Into<String>) -> Refusal {
        Refusal::IllegalMove(WireError::raised(code, why.into(), self.run_id()))
    }

    /// The hosted Job holding the connection, else the one named, which must be one Fleet holds.
    pub(super) async fn watching_job(&self, caller: Option<api::Caller>, named: Option<&SessionId>) -> Result<Option<String>, Refusal> {
        if let Some(held) = caller.and_then(|caller| self.session_holding(&caller)) {
            return Ok(Some(held));
        }
        let Some(named) = named.map(|named| named.as_str().trim().to_string()).filter(|id| !id.is_empty()) else {
            return Ok(None);
        };
        match self.store().lock().await.session(&named).map_err(|why| self.ledger_fault(why))? {
            Some(_) => Ok(Some(named)),
            None => Err(self.watch_unacceptable(NO_SUCH_SESSION, format!("No Job Fleet holds is named {named}."))),
        }
    }

    pub(super) async fn placed_job(&self, caller: Option<api::Caller>, named: Option<&SessionId>) -> Result<String, Refusal> {
        self.watching_job(caller, named)
            .await?
            .ok_or_else(|| self.watch_unacceptable(UNPLACED, "A watch is a Job's, and nothing places one."))
    }

    /// The recipes Armada ships, the Job's repository holds and this machine holds, read on every
    /// start. A Job in no repository Fleet serves has the shipped ones and this machine's.
    pub(super) async fn recipes_for(&self, job: &str) -> config::WatchRecipes {
        let manifest = match self.store().lock().await.session(job).ok().flatten() {
            Some(session) => session.manifest_id.filter(|id| !id.is_empty()),
            None => None,
        };
        let served = manifest.and_then(|id| self.served_named(Some(&ipc::ManifestId::carried(&id))).ok());
        let written = match served {
            Some(served) => self.locating().watch_recipes(Path::new(served.root()), served.manifest().base()),
            None => self.locating().machine_watch_recipes(),
        };
        config::WatchRecipes::of(written)
    }

    /// `recipe` among `job`'s, its command filled from `values`.
    pub(super) async fn recipe_filled(
        &self,
        job: &str,
        recipe: &str,
        values: &BTreeMap<String, String>,
    ) -> Result<Filled, Refusal> {
        let recipes = self.recipes_for(job).await;
        let Some(found) = recipes.named(recipe) else {
            return Err(self.watch_unacceptable(RECIPE_UNKNOWN, unknown_recipe(recipe, &recipes)));
        };
        let mut command = core_model::fill(&found.recipe.command, values)
            .map_err(|why| self.watch_unacceptable(VALUE_REFUSED, unfilled(recipe, &why)))?;
        if found.source == config::WatchRecipeSource::Armada && command.first().is_some_and(|program| program == ARMADA) {
            if let Ok(own) = std::env::current_exe() {
                command[0] = own.to_string_lossy().into_owned();
            }
        }
        Ok(Filled {
            command,
            environment: found.recipe.environment.clone(),
            every: found.recipe.every_seconds,
            gives_up_after: found.recipe.gives_up_after_seconds,
        })
    }

    /// `command` as the agent gave it, refused where it is not a list of whole arguments.
    pub(super) fn command_checked(&self, command: &[String]) -> Result<(), Refusal> {
        if command.is_empty() || command.iter().any(|item| item.trim().is_empty() || item.contains('\0')) {
            return Err(self.watch_unacceptable(REFUSED, "`command` is a list of arguments, none of them blank."));
        }
        Ok(())
    }

    /// `written`, a span named `key`, in seconds.
    pub(super) fn span(&self, key: &str, written: &str) -> Result<u32, Refusal> {
        core_model::span_seconds(written).ok_or_else(|| {
            self.watch_unacceptable(REFUSED, format!("`{key}` is `{written}`, where it is a count and `s`, `m` or `h`: `60s`."))
        })
    }

    async fn resolved(&self, job: &str, starts: WatchStarts) -> Result<Resolved, Refusal> {
        match starts {
            WatchStarts::Recipe { recipe, values } => {
                let filled = self.recipe_filled(job, &recipe, &values).await?;
                Ok(Resolved {
                    command: filled.command,
                    recipe: Some(recipe),
                    environment: filled.environment,
                    every: filled.every,
                    gives_up_after: filled.gives_up_after,
                    in_terminal: None,
                })
            }
            WatchStarts::Command { command, every, gives_up_after } => {
                self.command_checked(&command)?;
                let every = self.span("every", &every)?;
                let gives_up_after = gives_up_after.map(|written| self.span("gives_up_after", &written)).transpose()?;
                Ok(Resolved { command, recipe: None, environment: None, every, gives_up_after, in_terminal: None })
            }
            WatchStarts::Terminal { text, regex, gives_up_after } => {
                if text.trim().is_empty() {
                    return Err(self.watch_unacceptable(REFUSED, "`in_terminal` is what to wait for, and it is blank."));
                }
                super::listening::wanted(&text, regex).map_err(|why| self.watch_unacceptable(REFUSED, why))?;
                if self.session_runs().shell_heard(job).is_none() {
                    return Err(self.watch_illegal(
                        TERMINAL_CLOSED,
                        "This Job has no terminal open, so nothing would print. type_in_terminal opens it.",
                    ));
                }
                let gives_up_after = gives_up_after.map(|written| self.span("gives_up_after", &written)).transpose()?;
                // How soon it listens again after its terminal ended: the fastest a watch may run.
                let every = u32::try_from(self.machine_settings().now().get(keys::WATCH_EVERY_SECONDS).as_secs()).unwrap_or(u32::MAX);
                Ok(Resolved { command: Vec::new(), recipe: None, environment: None, every, gives_up_after, in_terminal: Some((text, regex)) })
            }
        }
    }

    fn not_weighed(&self, why: NotWeighed) -> Refusal {
        match why {
            NotWeighed::Ended => self.watch_illegal(ENDED, "This watch has ended, so there is nothing to weigh."),
            NotWeighed::NotFailed => self.watch_illegal(NOT_FAILED, "`ours` ends a failed watch, and this one has not failed."),
            NotWeighed::NoParts => self.watch_unacceptable(REFUSED, "`not_ours` names the parts to set aside."),
            NotWeighed::NoSentence => self.watch_unacceptable(REFUSED, "A verdict says why in a sentence."),
            NotWeighed::Unreported(part) => {
                self.watch_unacceptable(PART_UNREPORTED, format!("No reading of this watch carried a part named {part}."))
            }
        }
    }

    /// `job`'s live watches.
    async fn live_for(&self, job: &str) -> Result<usize, Refusal> {
        let all = self.store().lock().await.watches_of(job).map_err(|why| self.ledger_fault(why))?;
        Ok(all.iter().filter(|one| one.watch.live).count())
    }
}

fn unknown_recipe(named: &str, recipes: &config::WatchRecipes) -> String {
    let there: Vec<&str> = recipes.recipes().iter().map(|one| one.recipe.name.as_str()).collect();
    let mut said = match there.is_empty() {
        true => format!("No watch recipe is named {named}, and none is there."),
        false => format!("No watch recipe is named {named}. There are: {}.", there.join(", ")),
    };
    for left in recipes.left_out() {
        said.push(' ');
        said.push_str(&left.to_string());
        said.push('.');
    }
    said
}

fn unfilled(recipe: &str, why: &NotFilled) -> String {
    match why {
        NotFilled::Missing(name) => format!("The recipe {recipe} needs a value for `{name}`."),
        NotFilled::Unknown(name) => format!("The recipe {recipe} names no value `{name}`."),
        NotFilled::Unusable(name) => format!("The value for `{name}` is blank or holds a character no argument can carry."),
    }
}

impl<H, V, W> Watches for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn start_watch(&self, caller: Option<api::Caller>, start: StartWatch) -> Result<WatchSummary, Refusal> {
        let job = self.placed_job(caller, start.session_id.as_ref()).await?;
        let name = start.name.trim().to_string();
        if name.is_empty() {
            return Err(self.watch_unacceptable(REFUSED, "`name` is what the person reads, and it is blank."));
        }
        if start.set_aside.iter().any(|aside| aside.part.trim().is_empty() || aside.sentence.trim().is_empty()) {
            return Err(self.watch_unacceptable(REFUSED, "Each part set aside names the part and says why in a sentence."));
        }
        let settings = self.machine_settings().now();
        let per_job = usize::try_from(settings.get(keys::WATCHES_PER_JOB)).unwrap_or(0);
        if self.live_for(&job).await? >= per_job {
            return Err(self.watch_illegal(
                LIMIT_REACHED,
                format!("This Job already holds {per_job} live watches, the most `limits.watchesPerJob` allows. Stop one first."),
            ));
        }
        let resolved = self.resolved(&job, start.starts).await?;
        let floor = u32::try_from(settings.get(keys::WATCH_EVERY_SECONDS).as_secs()).unwrap_or(u32::MAX);
        if resolved.every < floor {
            return Err(self.watch_unacceptable(
                TOO_OFTEN,
                format!("It asks to run every {}s, and `timeouts.watchEverySeconds` allows every {floor}s at most.", resolved.every),
            ));
        }
        let shipped = u32::try_from(settings.get(keys::WATCH_GIVE_UP_MINUTES)).unwrap_or(0).saturating_mul(60);
        let gives_up_after = resolved.gives_up_after.unwrap_or(shipped);
        if gives_up_after > GIVE_UP_CAP_SECONDS {
            return Err(self.watch_unacceptable(GIVES_UP_TOO_LATE, "A watch gives up after a day at most: `24h`."));
        }
        let (in_terminal, regex) = match resolved.in_terminal {
            Some((text, regex)) => (Some(text), regex),
            None => (None, false),
        };
        let mut watch = core_model::Watch::new(name, resolved.command, resolved.every, gives_up_after);
        watch.follows = start.follows;
        watch.set_aside = start
            .set_aside
            .into_iter()
            .map(|aside| core_model::SetAside { part: aside.part, sentence: aside.sentence })
            .collect();
        // Set aside at the start, a part counts as reported, so the agent can weigh it again.
        watch.reported = watch.set_aside.iter().map(|aside| aside.part.clone()).collect();
        let now = self.now();
        let kept = KeptWatch {
            id: format!("w-{}", self.mint().ulid().as_str()),
            session_id: job.clone(),
            name: watch.name.clone(),
            command: watch.command.clone(),
            recipe: resolved.recipe,
            environment: resolved.environment,
            every_seconds: watch.every_seconds,
            gives_up_after_seconds: watch.gives_up_after_seconds,
            follows: watch.follows,
            state: watch.state.as_wire().to_string(),
            live: true,
            started_at: now.as_str().to_string(),
            reported: watch.reported.clone(),
            changed_at: now.as_str().to_string(),
            ended_at: None,
            unsaid: None,
            in_terminal,
            regex,
        };
        // A terminal watch runs nothing, so no environment is read for it.
        if kept.in_terminal.is_none() {
            if let Folder::At(folder) = self.watch_folder(&job).await {
                let login = super::environment::read(self.watches().shell().as_deref(), &folder).await;
                self.watches().keep_environment(&kept.id, login);
            }
        }
        let whole = {
            let mut store = self.store().lock().await;
            store.keep_watch(&kept).map_err(|why| self.ledger_fault(why))?;
            for aside in &watch.set_aside {
                let aside = KeptWatchSetAside { part: aside.part.clone(), sentence: aside.sentence.clone(), at: now.as_str().to_string() };
                store.keep_watch_set_aside(&kept.id, &aside).map_err(|why| self.ledger_fault(why))?;
            }
            store.watch_whole(&kept.id).map_err(|why| self.ledger_fault(why))?
        };
        // Its first reading is the runner's next pass, `running::EVERY` away at most.
        self.watches_published(&job).await;
        self.watching_row_published(&job).await;
        whole.map(|whole| self.watch_summary(&whole)).ok_or_else(|| self.no_such_watch(&kept.id))
    }

    async fn weigh_watch(&self, caller: Option<api::Caller>, weigh: WeighWatch) -> Result<WatchSummary, Refusal> {
        let job = self.watching_job(caller, weigh.session_id.as_ref()).await?;
        let (verdict, given) = match weigh.verdict {
            WatchVerdict::Ours { sentence } => {
                (Verdict::Ours { sentence: sentence.clone() }, (true, sentence, Vec::new()))
            }
            WatchVerdict::NotOurs { sentence, parts } => {
                (Verdict::NotOurs { sentence: sentence.clone(), parts: parts.clone() }, (false, sentence, parts))
            }
        };
        let now = self.now();
        let (job, whole) = {
            let mut store = self.store().lock().await;
            let kept = store
                .watch_whole(&weigh.watch)
                .map_err(|why| self.ledger_fault(why))?
                .filter(|kept| job.as_deref().is_none_or(|job| kept.watch.session_id == job))
                .ok_or_else(|| self.no_such_watch(&weigh.watch))?;
            let mut watch = domain(&kept);
            watch.weigh(verdict).map_err(|why| self.not_weighed(why))?;
            let (ours, sentence, parts) = given;
            let at = now.as_str().to_string();
            for part in &parts {
                let aside = KeptWatchSetAside { part: part.clone(), sentence: sentence.trim().to_string(), at: at.clone() };
                store.keep_watch_set_aside(&kept.watch.id, &aside).map_err(|why| self.ledger_fault(why))?;
            }
            let given = KeptWatchVerdict { at: at.clone(), ours, sentence: sentence.trim().to_string(), parts };
            store.keep_watch_verdict(&kept.watch.id, &given).map_err(|why| self.ledger_fault(why))?;
            let mut next = moved(&kept.watch, &watch, &now);
            if next.state != kept.watch.state {
                next.changed_at = at.clone();
            }
            // The agent gave this, so what it comes to tells nobody.
            if let Some(key) = owed(&watch) {
                let _ = store.keep_watch_notice(&kept.watch.id, &key, &at);
            }
            next.unsaid = unsaid(&store, &kept.watch.id, &watch);
            store.keep_watch(&next).map_err(|why| self.ledger_fault(why))?;
            let whole = store.watch_whole(&kept.watch.id).map_err(|why| self.ledger_fault(why))?;
            (next.session_id, whole)
        };
        self.watches_published(&job).await;
        whole.map(|whole| self.watch_summary(&whole)).ok_or_else(|| self.no_such_watch(&weigh.watch))
    }

    async fn stop_watch(&self, caller: Option<api::Caller>, stop: StopWatch) -> Result<WatchSummary, Refusal> {
        let job = self.watching_job(caller, stop.session_id.as_ref()).await?;
        let now = self.now();
        let (job, whole) = {
            let mut store = self.store().lock().await;
            let kept = store
                .watch_whole(&stop.watch)
                .map_err(|why| self.ledger_fault(why))?
                .filter(|kept| job.as_deref().is_none_or(|job| kept.watch.session_id == job))
                .ok_or_else(|| self.no_such_watch(&stop.watch))?;
            let mut watch = domain(&kept);
            watch.end(WatchState::Stopped);
            let mut next = moved(&kept.watch, &watch, &now);
            next.unsaid = unsaid(&store, &kept.watch.id, &watch);
            store.keep_watch(&next).map_err(|why| self.ledger_fault(why))?;
            let whole = store.watch_whole(&kept.watch.id).map_err(|why| self.ledger_fault(why))?;
            (next.session_id, whole)
        };
        self.watches().stop(&stop.watch);
        self.watches_published(&job).await;
        self.watching_row_published(&job).await;
        whole.map(|whole| self.watch_summary(&whole)).ok_or_else(|| self.no_such_watch(&stop.watch))
    }

    async fn delete_watch(&self, caller: Option<api::Caller>, delete: DeleteWatch) -> Result<WatchList, Refusal> {
        let job = self.watching_job(caller, delete.session_id.as_ref()).await?;
        let job = {
            let mut store = self.store().lock().await;
            let kept = store
                .watch_whole(&delete.watch)
                .map_err(|why| self.ledger_fault(why))?
                .filter(|kept| job.as_deref().is_none_or(|job| kept.watch.session_id == job))
                .ok_or_else(|| self.no_such_watch(&delete.watch))?;
            if kept.watch.live {
                return Err(self.watch_illegal(LIVE, "This watch is live. Stop it first."));
            }
            store.delete_watch(&kept.watch.id).map_err(|why| self.ledger_fault(why))?;
            kept.watch.session_id
        };
        self.watches_published(&job).await;
        self.watch_list(&job).await
    }

    async fn list_watches(
        &self,
        caller: Option<api::Caller>,
        session_id: Option<SessionId>,
        ended: bool,
    ) -> Result<WatchList, Refusal> {
        let job = self.placed_job(caller, session_id.as_ref()).await?;
        let mut list = self.watch_list(&job).await?;
        list.watches.retain(|watch| ended || watch.live);
        Ok(list)
    }

    async fn try_watch(&self, caller: Option<api::Caller>, tried: ipc::TryWatch) -> Result<ipc::WatchTried, Refusal> {
        self.watch_tried(caller, tried).await
    }

    async fn list_watch_recipes(
        &self,
        caller: Option<api::Caller>,
        session_id: Option<SessionId>,
    ) -> Result<ipc::WatchRecipeList, Refusal> {
        self.watch_recipes_listed(caller, session_id).await
    }

    async fn save_watch_recipe(
        &self,
        caller: Option<api::Caller>,
        save: ipc::SaveWatchRecipe,
    ) -> Result<ipc::WatchRecipeSaved, Refusal> {
        self.watch_recipe_saved(caller, save).await
    }
}
