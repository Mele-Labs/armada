//! The models the harness offers, as Fleet last asked for them.
//!
//! **Asked after Fleet is serving, never before.** The answer comes from a process the
//! adapter starts, so nothing waits on it: until it has come, and wherever it cannot, the
//! built-in names ship and the answer says it did not read them, and why. A read moves
//! what ships as `models.roster` to the names it found, so a roster nobody saved is every
//! model the harness offers; a roster somebody saved still wins.

use std::sync::{Arc, Mutex, OnceLock};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings::{self as keys, Key};
use ipc::{Event, ModelChoices, ModelsRead, OfferedModel};

use crate::daemon::Fleet;

/// What asks the harness for its models. **A seam so Fleet names no vendor**, and a test
/// can plant an answer. Blocking: Fleet runs it off the async threads.
pub trait ModelReader: Send + Sync + 'static {
    /// The models offered by an alias, each with `on` unset; or the reason it could not.
    fn read(&self) -> Result<Vec<OfferedModel>, String>;
}

/// The last answer, and who gives the next.
#[derive(Default)]
pub(crate) struct ModelsHeld {
    reader: OnceLock<Arc<dyn ModelReader>>,
    answer: Mutex<Option<Result<Vec<OfferedModel>, String>>>,
    /// One ask at a time, so a press of fresh during the start's ask waits and reads the same answer.
    asking: tokio::sync::Mutex<()>,
}

impl ModelsHeld {
    fn answer(&self) -> std::sync::MutexGuard<'_, Option<Result<Vec<OfferedModel>, String>>> {
        self.answer.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Why nothing has been read, said when nothing has asked or nothing can.
const NOT_ASKED: &str = "The agent has not been asked for its models yet.";
const NO_READER: &str = "Nothing here can ask the agent for its models.";

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
    /// What asks the harness, handed in by the composition root once Fleet is built.
    pub fn reading_models_with(&self, reader: Arc<dyn ModelReader>) {
        let _ = self.models_held().reader.set(reader);
    }

    /// Ask the harness and hold the answer, moving what ships as the roster to what it found.
    /// **Blocks only this task**: the process runs on a blocking thread. Says so on the
    /// stream when the answer is not the one held, so Bridge reads the models again.
    pub async fn models_asked(&self) {
        let held = self.models_held();
        let _one = held.asking.lock().await;
        let answer = match held.reader.get() {
            None => Err(NO_READER.to_string()),
            Some(reader) => {
                let reader = Arc::clone(reader);
                tokio::task::spawn_blocking(move || reader.read())
                    .await
                    .unwrap_or_else(|_| Err(String::from("The question to the agent was cut short.")))
            }
        };
        if let Ok(rows) = &answer {
            let names: Vec<String> = rows.iter().map(|row| row.name.clone()).collect();
            self.machine_settings().offering(&names);
        }
        let changed = held.answer().replace(answer.clone()) != Some(answer);
        if changed {
            self.publish(Event::SettingsChanged(self.machine_settings().listed()));
        }
    }

    /// The models as served: the rows with their switches, the ones switched on, and whether
    /// the harness was read. `default` and `session` are the settings' as they stand.
    pub(crate) fn models_now(&self) -> ModelChoices {
        let now = self.machine_settings().now();
        let on = now.models();
        let (rows, read) = match self.models_held().answer().clone() {
            Some(Ok(rows)) => {
                let rows: Vec<OfferedModel> = rows
                    .into_iter()
                    .map(|row| OfferedModel { on: on.contains(&row.name), ..row })
                    .collect();
                (rows, ModelsRead::Read)
            }
            asked => {
                let why = match asked {
                    Some(Err(why)) => why,
                    _ => NOT_ASKED.to_string(),
                };
                (self.built_in_rows(&on), ModelsRead::NotRead { why })
            }
        };
        let (default, session) = (self.default_model(), self.session_model());
        let mut models: Vec<String> = match read {
            // A roster that names a model the harness no longer offers is not a list of models to pick from.
            ModelsRead::Read => rows.iter().filter(|row| row.on).map(|row| row.name.clone()).collect(),
            ModelsRead::NotRead { .. } => on,
        };
        for named in [&default, &session] {
            if !models.contains(named) {
                models.push(named.clone());
            }
        }
        ModelChoices { models, default, session, offered: rows, read, ..self.models().clone() }
    }

    /// The built-in names, bare, each switched on or off by the roster in force.
    fn built_in_rows(&self, on: &[String]) -> Vec<OfferedModel> {
        let now = self.machine_settings().now();
        let shipped = config::settings::entry(keys::MODELS_ROSTER.name())
            .map(|found| keys::List::read(&now.default_of(found)))
            .unwrap_or_default();
        let mut names = shipped;
        for named in on {
            if !names.contains(named) {
                names.push(named.clone());
            }
        }
        names
            .into_iter()
            .map(|name| OfferedModel {
                on: on.contains(&name),
                name,
                display: String::new(),
                version: String::new(),
                description: String::new(),
                efforts: Vec::new(),
            })
            .collect()
    }

    /// The names a Job, a definition or a proposal may name: those switched on.
    pub fn models_on(&self) -> Vec<String> {
        self.models_now().models
    }
}
