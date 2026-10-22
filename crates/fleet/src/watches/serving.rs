//! [`Watches`] for Fleet: a start checked and kept, a weighing and a stop on what is kept.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Watches};
use config::settings as keys;
use core_model::{NotFilled, NotWeighed, Verdict, WatchState};
use ipc::{SessionId, StartWatch, StopWatch, WatchList, WatchStarts, WatchSummary, WatchVerdict, WeighWatch, WireError};

use super::*;
use crate::daemon::Fleet;

/// What a start comes to before it is kept: the command filled, and its two spans in seconds.
struct Resolved {
    command: Vec<String>,
    recipe: Option<String>,
    every: u32,
    gives_up_after: Option<u32>,
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
    fn watch_unacceptable(&self, code: &str, why: impl Into<String>) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why.into(), self.run_id()))
    }

    fn watch_illegal(&self, code: &str, why: impl Into<String>) -> Refusal {
        Refusal::IllegalMove(WireError::raised(code, why.into(), self.run_id()))
    }

    /// The hosted Job holding the connection, else the one named.
    // next group: refuse a named Job Fleet does not hold, as `fleet.no_such_session`.
    fn watching_job(&self, caller: Option<api::Caller>, named: Option<&SessionId>) -> Option<String> {
        caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| named.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
    }

    fn placed_job(&self, caller: Option<api::Caller>, named: Option<&SessionId>) -> Result<String, Refusal> {
        self.watching_job(caller, named)
            .ok_or_else(|| self.watch_unacceptable(UNPLACED, "A watch is a Job's, and nothing places one."))
    }

    /// The recipes the Job's repository and this machine hold, read on every start.
    async fn recipes_for(&self, job: &str) -> config::WatchRecipes {
        let manifest = match self.store().lock().await.session(job).ok().flatten() {
            Some(session) => session.manifest_id.filter(|id| !id.is_empty()),
            None => None,
        };
        let served = manifest.and_then(|id| self.served_named(Some(&ipc::ManifestId::carried(&id))).ok());
        let written = match served {
            Some(served) => self.locating().watch_recipes(Path::new(served.root()), served.manifest().base()),
            None => Vec::new(),
        };
        config::WatchRecipes::of(written)
    }

    async fn resolved(&self, job: &str, starts: WatchStarts) -> Result<Resolved, Refusal> {
        match starts {
            WatchStarts::Recipe { recipe, values } => {
                let recipes = self.recipes_for(job).await;
                let Some(found) = recipes.named(&recipe) else {
                    return Err(self.watch_unacceptable(RECIPE_UNKNOWN, unknown_recipe(&recipe, &recipes)));
                };
                let command = core_model::fill(&found.recipe.command, &values)
                    .map_err(|why| self.watch_unacceptable(VALUE_REFUSED, unfilled(&recipe, &why)))?;
                Ok(Resolved {
                    command,
                    recipe: Some(recipe),
                    every: found.recipe.every_seconds,
                    gives_up_after: found.recipe.gives_up_after_seconds,
                })
            }
            WatchStarts::Command { command, every, gives_up_after } => {
                if command.is_empty() || command.iter().any(|item| item.trim().is_empty() || item.contains('\0')) {
                    return Err(self.watch_unacceptable(REFUSED, "`command` is a list of arguments, none of them blank."));
                }
                let span = |key: &str, written: &str| {
                    core_model::span_seconds(written).ok_or_else(|| {
                        self.watch_unacceptable(REFUSED, format!("`{key}` is `{written}`, where it is a count and `s`, `m` or `h`: `60s`."))
                    })
                };
                let every = span("every", &every)?;
                let gives_up_after = gives_up_after.map(|written| span("gives_up_after", &written)).transpose()?;
                Ok(Resolved { command, recipe: None, every, gives_up_after })
            }
        }
    }

    fn summary_of(&self, id: &str, job: Option<&str>) -> Result<WatchSummary, Refusal> {
        self.watches()
            .change(id, job, |held| held.summary())
            .ok_or_else(|| self.no_such_watch(id))
    }

    fn no_such_watch(&self, id: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(NO_SUCH_WATCH, format!("No watch of this Job is {id}."), self.run_id()))
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
        let job = self.placed_job(caller, start.session_id.as_ref())?;
        let name = start.name.trim().to_string();
        if name.is_empty() {
            return Err(self.watch_unacceptable(REFUSED, "`name` is what the person reads, and it is blank."));
        }
        if start.set_aside.iter().any(|aside| aside.part.trim().is_empty() || aside.sentence.trim().is_empty()) {
            return Err(self.watch_unacceptable(REFUSED, "Each part set aside names the part and says why in a sentence."));
        }
        let settings = self.machine_settings().now();
        let per_job = usize::try_from(settings.get(keys::WATCHES_PER_JOB)).unwrap_or(0);
        if self.watches().live_for(&job) >= per_job {
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
        let mut watch = core_model::Watch::new(name, resolved.command, resolved.every, gives_up_after);
        watch.follows = start.follows;
        watch.set_aside = start
            .set_aside
            .into_iter()
            .map(|aside| core_model::SetAside { part: aside.part, sentence: aside.sentence })
            .collect();
        // Set aside at the start, a part counts as reported, so the agent can weigh it again.
        watch.reported = watch.set_aside.iter().map(|aside| aside.part.clone()).collect();
        let held = Held {
            id: format!("w-{}", self.mint().ulid().as_str()),
            job,
            recipe: resolved.recipe,
            started_at: ipc::Instant::from(&self.now()),
            watch,
        };
        let summary = held.summary();
        self.watches().keep(held);
        // next group: schedule its first run on the pull request reading's tick, and publish it.
        Ok(summary)
    }

    async fn weigh_watch(&self, caller: Option<api::Caller>, weigh: WeighWatch) -> Result<WatchSummary, Refusal> {
        let job = self.watching_job(caller, weigh.session_id.as_ref());
        let verdict = match weigh.verdict {
            WatchVerdict::Ours => Verdict::Ours,
            WatchVerdict::NotOurs { sentence, parts } => Verdict::NotOurs { sentence, parts },
        };
        let weighed = self
            .watches()
            .change(&weigh.watch, job.as_deref(), |held| held.watch.weigh(verdict).map(|_| held.summary()))
            .ok_or_else(|| self.no_such_watch(&weigh.watch))?;
        // next group: a weighing that ends the watch tells nobody, since the agent gave it; publish it.
        weighed.map_err(|why| match why {
            NotWeighed::Ended => self.watch_illegal(ENDED, "This watch has ended, so there is nothing to weigh."),
            NotWeighed::NotFailed => self.watch_illegal(NOT_FAILED, "`ours` ends a failed watch, and this one has not failed."),
            NotWeighed::NoParts => self.watch_unacceptable(REFUSED, "`not_ours` names the parts to set aside."),
            NotWeighed::NoSentence => self.watch_unacceptable(REFUSED, "`not_ours` says why in a sentence."),
            NotWeighed::Unreported(part) => {
                self.watch_unacceptable(PART_UNREPORTED, format!("No reading of this watch carried a part named {part}."))
            }
        })
    }

    async fn stop_watch(&self, caller: Option<api::Caller>, stop: StopWatch) -> Result<WatchSummary, Refusal> {
        let job = self.watching_job(caller, stop.session_id.as_ref());
        self.watches()
            .change(&stop.watch, job.as_deref(), |held| held.watch.end(WatchState::Stopped))
            .ok_or_else(|| self.no_such_watch(&stop.watch))?;
        // next group: end the command if one is running, and publish the change.
        self.summary_of(&stop.watch, job.as_deref())
    }

    async fn list_watches(&self, caller: Option<api::Caller>, session_id: Option<SessionId>) -> Result<WatchList, Refusal> {
        let job = self.placed_job(caller, session_id.as_ref())?;
        Ok(WatchList { watches: self.watches().of(&job).iter().map(Held::summary).collect() })
    }
}
