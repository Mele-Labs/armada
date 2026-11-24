//! A project's live metrics: the recipes its file keeps under `look.metrics`, and what Fleet read with them while its
//! dashboard was open. `docs/practices/protocol/projects.md`, *Metrics*.
//!
//! **No model, no timer.** Bridge asks while the dashboard is open, and Fleet fetches a recipe only when its last
//! attempt is older than the recipe's interval. Two asks at once wait on one fetch: each recipe's reading is behind its
//! own lock, and the second ask finds it fresh.
//!
//! **A token is read for the one call** from this Mac's keychain, by the credential's name, and dropped with it. It is
//! in no reading, answer, error or log: the adapter scrubs every why, and a keychain's own refusal names no value.
//! Recipes are in the project's file and so reach his other Macs; tokens never do.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use adapter_traits::{Secret, SecretVariable};
use api::{Projects, Refusal};
use core_model::Timestamp;
use ipc::{
    BoardBlock, BoardLayout, EditProject, Instant, LookSet, MetricAnswer, MetricCredential, MetricPoint, MetricReading, MetricReadings,
    MetricRecipe, MetricRow, MetricSaved, MetricState, MetricValue, ProjectAct, SaveMetricToken, TryMetric,
};
use tokio::sync::{Mutex as Gate, OwnedMutexGuard};

use crate::daemon::Fleet;
use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

/// A 422 for a recipe that cannot read, and nothing is kept.
pub(crate) const UNREADABLE: &str = "fleet.metric_unreadable";
/// A 422 for a token the keychain will not take, or one that is blank.
pub(crate) const TOKEN_REFUSED: &str = "fleet.metric_token_refused";

/// The most points one recipe keeps, oldest dropped first.
const KEPT_AT_MOST: usize = 500;
/// The size of the block a saved recipe is given at the bottom of the board, in the grid's columns and rows.
const BLOCK_WIDTH: u8 = 3;
const BLOCK_HEIGHT: u8 = 2;

/// What Fleet holds of one recipe, in memory only: **nothing here is written down**, so a restart reads again.
#[derive(Default)]
pub(crate) struct Entry {
    /// The last answer that read.
    answer: Option<MetricAnswer>,
    read_at: Option<Timestamp>,
    /// The last fetch, whether it read or not: a recipe that is broken is asked again after its interval, not at every ask.
    tried_at: Option<Timestamp>,
    broken: Option<String>,
    pub(crate) kept: Vec<MetricPoint>,
}

impl Entry {
    fn reading(&self, recipe: &MetricRecipe, state: MetricState) -> MetricReading {
        MetricReading {
            id: recipe.id.clone(),
            title: recipe.title.clone(),
            state,
            answer: self.answer.clone(),
            read_at: self.read_at.as_ref().map(Instant::from),
            kept: self.kept.clone(),
        }
    }

    fn state(&self) -> MetricState {
        match &self.broken {
            Some(why) => MetricState::Broken { why: why.clone() },
            None => MetricState::Read,
        }
    }

    /// What a fetch came to, at `now`.
    pub(crate) fn took(&mut self, now: &Timestamp, fetched: Result<MetricAnswer, String>) {
        self.tried_at = Some(now.clone());
        match fetched {
            Ok(answer) => {
                if let Some(value) = first_number(&answer) {
                    self.kept.push(MetricPoint { at: Instant::from(now), value });
                    let over = self.kept.len().saturating_sub(KEPT_AT_MOST);
                    self.kept.drain(..over);
                }
                self.answer = Some(answer);
                self.read_at = Some(now.clone());
                self.broken = None;
            }
            Err(why) => self.broken = Some(why),
        }
    }

    fn stale(&self, recipe: &MetricRecipe, now: &Timestamp) -> bool {
        match &self.tried_at {
            None => true,
            Some(at) => crate::clock::elapsed(at, now).as_secs() >= u64::from(recipe.every_secs()),
        }
    }
}

/// The first number of the answer's rows, which is what Graph draws where the answer is not itself a series.
fn first_number(answer: &MetricAnswer) -> Option<f64> {
    answer.rows.iter().find_map(|row: &MetricRow| match row.value {
        MetricValue::Number(value) if value.is_finite() => Some(value),
        _ => None,
    })
}

/// The readings this Fleet holds, by project and recipe.
#[derive(Default)]
pub(crate) struct Readings {
    held: Mutex<BTreeMap<(String, String), Arc<Gate<Entry>>>>,
}

impl Readings {
    fn of(&self, project: &str, id: &str) -> Arc<Gate<Entry>> {
        let mut held = self.held.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        Arc::clone(held.entry((project.to_string(), id.to_string())).or_default())
    }

    fn forget(&self, project: &str, id: &str) {
        self.held.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&(project.to_string(), id.to_string()));
    }
}

/// A recipe as the wire carries it.
pub(crate) fn recipe_wire(recipe: &core_model::MetricRecipe) -> MetricRecipe {
    use core_model::MetricCall as Held;
    let headers = |held: &[(String, String)]| held.iter().map(|(name, value)| ipc::MetricHeader { name: name.clone(), value: value.clone() }).collect();
    MetricRecipe {
        id: recipe.id.clone(),
        title: recipe.title.clone(),
        call: match &recipe.call {
            Held::Cli { argv, env } => ipc::MetricCall::Cli { argv: argv.clone(), env: headers(env) },
            Held::Http { method, url, headers: held, body } => {
                ipc::MetricCall::Http { method: method.clone(), url: url.clone(), headers: headers(held), body: body.clone() }
            }
            Held::Mcp { server, env, tool, arguments } => {
                ipc::MetricCall::Mcp { server: server.clone(), env: headers(env), tool: tool.clone(), arguments: arguments.clone() }
            }
        },
        map: recipe.map.clone(),
        every_secs: recipe.every_secs,
        credential: recipe
            .credential
            .as_ref()
            .map(|one| MetricCredential { name: one.name.clone(), how: one.how.clone(), url: one.url.clone() }),
    }
}

/// The recipes as the wire carries them: **none where the file keeps none**.
pub(crate) fn recipes_wire(recipes: &[core_model::MetricRecipe]) -> Option<Vec<MetricRecipe>> {
    (!recipes.is_empty()).then(|| recipes.iter().map(recipe_wire).collect())
}

fn recipe_held(recipe: &MetricRecipe) -> core_model::MetricRecipe {
    use core_model::MetricCall as Held;
    let pairs = |wire: &[ipc::MetricHeader]| wire.iter().map(|one| (one.name.clone(), one.value.clone())).collect();
    core_model::MetricRecipe {
        id: recipe.id.trim().to_string(),
        title: recipe.title.trim().to_string(),
        call: match &recipe.call {
            ipc::MetricCall::Cli { argv, env } => Held::Cli { argv: argv.clone(), env: pairs(env) },
            ipc::MetricCall::Http { method, url, headers, body } => {
                Held::Http { method: method.clone(), url: url.clone(), headers: pairs(headers), body: body.clone() }
            }
            ipc::MetricCall::Mcp { server, env, tool, arguments } => {
                Held::Mcp { server: server.clone(), env: pairs(env), tool: tool.clone(), arguments: arguments.clone() }
            }
        },
        map: recipe.map.clone(),
        every_secs: recipe.every_secs,
        credential: recipe.credential.as_ref().map(|one| core_model::MetricCredential {
            name: one.name.trim().to_string(),
            how: one.how.clone(),
            url: one.url.clone(),
        }),
    }
}

/// A read in flight: the recipe's lock is held until its answer is taken.
struct Fetching {
    recipe: MetricRecipe,
    entry: OwnedMutexGuard<Entry>,
    run: tokio::task::JoinHandle<Result<MetricAnswer, String>>,
}

/// One recipe, either read already or being fetched.
enum Asked {
    Done(MetricReading),
    Fetching(Fetching),
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
    /// A recipe the file holds can be unreadable by name, so the recipes are the wire's, in the file's order.
    async fn metric_recipes(&self, project: &str) -> Result<Vec<MetricRecipe>, Refusal> {
        let held = self.project_held(project).await?;
        Ok(held.values().look.metrics.iter().map(recipe_wire).collect())
    }

    /// `credential`'s token from this Mac's keychain, or why it could not be read. **A why never holds a value.**
    async fn metric_token(&self, credential: &MetricCredential) -> Result<Option<Secret<String>>, String> {
        let tokens = Arc::clone(self.metric_tokens());
        let variable = SecretVariable { repository: String::new(), environment: String::new(), name: credential.name.clone() };
        match tokio::task::spawn_blocking(move || tokens.read(&variable)).await {
            Ok(Ok(token)) => Ok(token),
            Ok(Err(why)) => Err(why.said()),
            Err(_) => Err("the keychain was not read".to_string()),
        }
    }

    /// `recipe` on a blocking thread with `token`, which goes with it and is dropped there.
    fn metric_run(&self, recipe: &MetricRecipe, token: Option<Secret<String>>) -> tokio::task::JoinHandle<Result<MetricAnswer, String>> {
        let recipe = recipe.clone();
        tokio::task::spawn_blocking(move || adapters::run_metric(&recipe, token.as_ref()))
    }

    /// The recipe's reading, **fetched only where its last attempt is older than its interval**. The recipe's own lock
    /// is held from here until the fetch is taken, so a second ask for it waits and then finds it fresh.
    async fn metric_asked(&self, project: &str, recipe: &MetricRecipe) -> Asked {
        let mut entry = self.metrics_held().of(project, &recipe.id).lock_owned().await;
        let now = self.now();
        if !entry.stale(recipe, &now) {
            return Asked::Done(entry.reading(recipe, entry.state()));
        }
        let token = match &recipe.credential {
            None => None,
            Some(credential) => match self.metric_token(credential).await {
                Ok(Some(token)) => Some(token),
                Ok(None) => return Asked::Done(entry.reading(recipe, MetricState::NoToken { credential: credential.clone() })),
                Err(why) => {
                    entry.took(&now, Err(why));
                    return Asked::Done(entry.reading(recipe, entry.state()));
                }
            },
        };
        let run = self.metric_run(recipe, token);
        Asked::Fetching(Fetching { recipe: recipe.clone(), entry, run })
    }

    /// The readings of every recipe the project keeps, those that need a fetch running together.
    async fn metric_readings(&self, project: &str) -> Result<MetricReadings, Refusal> {
        let recipes = self.metric_recipes(project).await?;
        let mut asked = Vec::with_capacity(recipes.len());
        for recipe in &recipes {
            asked.push(self.metric_asked(project, recipe).await);
        }
        let mut readings = Vec::with_capacity(asked.len());
        for one in asked {
            readings.push(match one {
                Asked::Done(reading) => reading,
                Asked::Fetching(Fetching { recipe, mut entry, run }) => {
                    let fetched = run.await.unwrap_or_else(|_| Err("the read did not finish".to_string()));
                    entry.took(&self.now(), fetched);
                    entry.reading(&recipe, entry.state())
                }
            });
        }
        Ok(MetricReadings { project: project.to_string(), readings })
    }

    /// `read_metrics`.
    pub(crate) async fn metrics_read_for(&self, project: String) -> Result<MetricReadings, Refusal> {
        self.metric_readings(&project).await
    }

    /// `recipe` run once, and nothing kept: a try asks its token and reads, whatever the interval.
    async fn metric_tried(&self, recipe: &MetricRecipe) -> (MetricReading, Option<MetricAnswer>) {
        let blank = Entry::default();
        let token = match &recipe.credential {
            None => None,
            Some(credential) => match self.metric_token(credential).await {
                Ok(Some(token)) => Some(token),
                Ok(None) => return (blank.reading(recipe, MetricState::NoToken { credential: credential.clone() }), None),
                Err(why) => return (blank.reading(recipe, MetricState::Broken { why }), None),
            },
        };
        let fetched = self.metric_run(recipe, token).await.unwrap_or_else(|_| Err("the read did not finish".to_string()));
        let mut tried = Entry::default();
        tried.took(&self.now(), fetched);
        // A try keeps no point; the reading it makes is a single one.
        tried.kept.clear();
        (tried.reading(recipe, tried.state()), tried.answer)
    }

    /// `try_metric`.
    pub(crate) async fn metric_tried_for(&self, project: String, ask: TryMetric) -> Result<MetricReading, Refusal> {
        self.project_held(&project).await?;
        Ok(self.metric_tried(&ask.recipe).await.0)
    }

    /// `save_metric`, and what the Librarian Job's op calls: **the recipe is tried and kept only where it read**, or where
    /// the only thing missing is this Mac's token. Its block goes at the bottom of the board the project saved.
    pub(crate) async fn metric_tried_and_saved(&self, project: String, recipe: MetricRecipe) -> Result<MetricSaved, Refusal> {
        let held = self.project_held(&project).await?;
        let (reading, answer) = self.metric_tried(&recipe).await;
        if let MetricState::Broken { why } = &reading.state {
            let said = format!("The recipe {} did not read, so it is not kept: {why}", recipe.id);
            return Err(Refusal::Unacceptable(self.project_refusal(UNREADABLE, said)));
        }
        let mut recipes: Vec<MetricRecipe> = held.values().look.metrics.iter().map(recipe_wire).collect();
        match recipes.iter_mut().find(|one| one.id == recipe.id) {
            Some(same) => *same = recipe.clone(),
            None => recipes.push(recipe.clone()),
        }
        let mut board = held.values().look.board.as_ref().map(crate::values::board_wire);
        if let Some(board) = board.as_mut().filter(|board| !board.blocks.iter().any(|one| one.kind == recipe.block_kind())) {
            let below = board.blocks.iter().map(|one| u16::from(one.y) + u16::from(one.h)).max().unwrap_or(0);
            let y = u8::try_from(below).unwrap_or(u8::MAX);
            board.blocks.push(BoardBlock { kind: recipe.block_kind(), x: 0, y, w: BLOCK_WIDTH, h: BLOCK_HEIGHT, view: "list".to_string() });
        }
        self.metrics_written(&project, recipes, board).await?;
        // What it read is what the dashboard's next ask finds, so the call is not made twice.
        let slot = self.metrics_held().of(&project, &recipe.id);
        let mut entry = slot.lock().await;
        *entry = Entry::default();
        if let (MetricState::Read, Some(answer)) = (&reading.state, answer) {
            entry.took(&self.now(), Ok(answer));
        }
        let reading = match &reading.state {
            MetricState::Read => entry.reading(&recipe, MetricState::Read),
            _ => reading,
        };
        Ok(MetricSaved { project, recipe, reading })
    }

    /// `save_metric`.
    pub(crate) async fn metric_saved(&self, project: String, ask: TryMetric) -> Result<MetricSaved, Refusal> {
        self.metric_tried_and_saved(project, ask.recipe).await
    }

    /// `remove_metric`: the recipe and its block go, and the rest are read. **Removing what is not kept is not an error.**
    pub(crate) async fn metric_removed(&self, project: String, id: String) -> Result<MetricReadings, Refusal> {
        let held = self.project_held(&project).await?;
        let kind = format!("metric:{id}");
        let recipes: Vec<MetricRecipe> = held.values().look.metrics.iter().filter(|one| one.id != id).map(recipe_wire).collect();
        let board = held.values().look.board.as_ref().map(crate::values::board_wire).map(|mut board| {
            board.blocks.retain(|one| one.kind != kind);
            board
        });
        self.metrics_written(&project, recipes, board).await?;
        self.metrics_held().forget(&project, &id);
        self.metric_readings(&project).await
    }

    /// `save_metric_token`: the token into this Mac's keychain, under the credential's name, and the project read again.
    pub(crate) async fn metric_token_saved(&self, project: String, save: SaveMetricToken) -> Result<MetricReadings, Refusal> {
        let held = self.project_held(&project).await?;
        let name = save.credential.trim().to_string();
        let name_ok = !name.is_empty() && name.len() <= 64 && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        if !name_ok {
            return Err(Refusal::Unacceptable(self.project_refusal(TOKEN_REFUSED, "a credential is named by letters, digits, dashes and underscores")));
        }
        let token = Secret::new(save.token.trim().to_string());
        let tokens = Arc::clone(self.metric_tokens());
        let variable = SecretVariable { repository: String::new(), environment: String::new(), name: name.clone() };
        let kept = tokio::task::spawn_blocking(move || tokens.keep(&variable, token)).await;
        match kept {
            Ok(Ok(())) => {}
            Ok(Err(why)) => return Err(Refusal::Unacceptable(self.project_refusal(TOKEN_REFUSED, why.said()))),
            Err(_) => return Err(Refusal::Unacceptable(self.project_refusal(TOKEN_REFUSED, "the keychain was not written"))),
        }
        // Each recipe that signs in with it is read at the next ask, whatever its interval.
        for recipe in held.values().look.metrics.iter().filter(|one| one.credential.as_ref().is_some_and(|c| c.name == name)) {
            let slot = self.metrics_held().of(&project, &recipe.id);
            slot.lock().await.tried_at = None;
        }
        self.metric_readings(&project).await
    }

    /// The recipe `id` and why it last failed to read, for the Librarian Job's fix. `None` where the project keeps none by that id.
    pub(crate) async fn metric_for_fix(&self, project: &str, id: &str) -> Option<(MetricRecipe, Option<String>)> {
        let held = self.project_held(project).await.ok()?;
        let recipe = held.values().look.metrics.iter().find(|one| one.id == id).map(recipe_wire)?;
        let broken = self.metrics_held().of(project, id).lock().await.broken.clone();
        Some((recipe, broken))
    }

    /// What the project's file keeps for metrics: the recipes and the board, written as `edit_project` writes values. A
    /// copy of another Mac's project sends the edit there.
    async fn metrics_written(&self, project: &str, recipes: Vec<MetricRecipe>, board: Option<BoardLayout>) -> Result<(), Refusal> {
        if let Some(mac) = self.copy_of_project(Some(project)).await? {
            let shown = self.project_copies().await?.into_iter().find(|one| one.mac.as_deref() == Some(mac.as_str()) && one.id == project);
            let mut values = shown.and_then(|one| one.values).unwrap_or_default();
            let mut look = values.look.take().unwrap_or_default();
            look.board = board;
            look.metrics = (!recipes.is_empty()).then_some(recipes);
            values.look = (look != LookSet::default()).then_some(look);
            let edit = EditProject { values: Some(values), ..EditProject::default() };
            self.act_on_copy(&mac, ProjectAct::EditProject { project: project.to_string(), edit }).await?;
            return Ok(());
        }
        {
            let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
            self.project_in(&kept, project)?;
        }
        let mut values = self.project_read(project)?.values().clone();
        values.look.board = self.board_read(board.as_ref())?;
        values.look.metrics = self.metrics_read(Some(&recipes))?;
        let values = crate::values::values_set(&values).unwrap_or_default();
        self.edit_project(project.to_string(), EditProject { values: Some(values), ..EditProject::default() }).await?;
        Ok(())
    }

    /// The recipes a person wrote, **each with an id, a title and a map, and no id twice**.
    pub(crate) fn metrics_read(&self, recipes: Option<&[MetricRecipe]>) -> Result<Vec<core_model::MetricRecipe>, Refusal> {
        let Some(recipes) = recipes else { return Ok(Vec::new()) };
        let mut held: Vec<core_model::MetricRecipe> = Vec::with_capacity(recipes.len());
        for one in recipes {
            let one = recipe_held(one);
            if one.id.is_empty() || one.title.is_empty() || one.map.trim().is_empty() {
                return Err(self.values_refused("a metric recipe names its id, its title and its map, and one of them is blank".to_string()));
            }
            if held.iter().any(|already| already.id == one.id) {
                return Err(self.values_refused(format!("the metric {} is written twice, and a recipe's id is the project's own", one.id)));
            }
            held.push(one);
        }
        Ok(held)
    }
}
