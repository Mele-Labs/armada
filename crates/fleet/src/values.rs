//! Values: how work behaves, laid over its layers (`config::values`), read for a project, a repository or
//! both, and **frozen on a Job when it starts**. `docs/practices/protocol/projects.md`, *Values*.
//!
//! | layer | where Fleet reads it |
//! |---|---|
//! | default | what Armada ships, and the review model the Machine's settings ship |
//! | Machine | settings.json, only what is saved there |
//! | Kit | `values.yml` in Kit's folder, read each time it is asked: absent is nothing set |
//! | repository | the Manifest's `pr_mode:` and `auto_merge:`, only where it writes them |
//! | Project | the `values:` block of the project's file |
//!
//! **A Job takes its project's values at start** and keeps them: later edits move the layers and not the
//! Job. A Job started for no project freezes its repository's.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Review, Vcs, WorkProduct};
use api::Refusal;
use config::settings::{self as keys, Key as _};
use config::{kit_values_text, parse_kit_values, shipped_values, Layers, ResolvedValue, ResolvedValues};
use core_model::{AgentStarts, AutoMerge, Component, Envelope, FieldValue, Layer, Level, Look, ManifestId as Repository, PrMode, ReviewMode, Reviews, ReviewsNamed, ScopeId, TrackerNamed, Ulid, Values, WalkReview, DeliverySetting};
use ipc::{KitValuesSaved, LookSet, ManifestId, ReviewsSet, SaveKitValues, ValuesSet};
use store::KeptValue;

use crate::daemon::Fleet;
use crate::project_files::UNREADABLE;
use crate::projects::NO_SUCH_PROJECT;

/// A value a project or Kit was given that its key does not take: an `auto_merge` that is not `never`,
/// `checks-pass` or `always`, a `density` that is not `compact`, `default` or `roomy`, or blank text. A 422.
pub(crate) const REFUSED: &str = "fleet.project_values_refused";
/// Kit's `values.yml` could not be written. A 500.
pub(crate) const KIT_UNWRITABLE: &str = "fleet.kit_values_unwritable";

/// The file Kit keeps its values in, under Kit's folder.
pub(crate) const KIT_FILE: &str = "values.yml";

const AUTONOMY: &str = "autonomy.";
const REVIEWS_PLANS: &str = "reviews.plans";
const REVIEWS_PULL_REQUESTS: &str = "reviews.pull_requests";
const REVIEWS_WALKS: &str = "reviews.walks";
const AGENT_STARTS: &str = "agent_starts";
const DELIVERY: &str = "delivery";

/// The tracker key that names the implementation, as the Manifest writes it.
const PROVIDER: &str = "provider";

/// What one layer sets, as the wire carries it. `None` where it sets nothing.
pub(crate) fn values_set(values: &Values) -> Option<ValuesSet> {
    if values.is_empty() {
        return None;
    }
    let look = &values.look;
    Some(ValuesSet {
        pr_mode: values.pr_mode.map(Into::into),
        auto_merge: values.auto_merge.map(|policy| policy.as_written().to_string()),
        review_model: values.review_model.clone(),
        look: (*look != Look::default())
            .then(|| LookSet { layout: look.layout.clone(), theme: look.theme.clone(), density: look.density.clone() }),
        tracker: values.tracker.as_ref().map(tracker_wire),
        reviews: (values.reviews != ReviewsNamed::default()).then(|| ReviewsSet {
            plans: values.reviews.plans.map(|mode| mode.as_wire().to_string()),
            pull_requests: values.reviews.pull_requests.map(|mode| mode.as_wire().to_string()),
            walks: values.reviews.walks.map(|mode| mode.as_wire().to_string()),
        }),
        agent_starts: values.agent_starts.map(|starts| starts.as_wire().to_string()),
        delivery: values.delivery.map(|delivery| delivery.as_wire().to_string()),
    })
}

/// A tracker as the wire carries it: the Manifest block's keys, `provider` among them.
fn tracker_wire(named: &TrackerNamed) -> BTreeMap<String, String> {
    let mut block = named.keys.clone();
    if let Some(provider) = &named.provider {
        block.insert(PROVIDER.to_string(), provider.clone());
    }
    block
}

/// What a Manifest's `tracker:` block names, as a layer holds it.
pub(crate) fn tracker_named(block: &config::TrackerBlock) -> TrackerNamed {
    TrackerNamed { provider: block.provider().map(str::to_string), keys: block.keys().clone() }
}

fn layer_wire(layer: Layer) -> ipc::ValueLayer {
    match layer {
        Layer::Default => ipc::ValueLayer::Default,
        Layer::Machine => ipc::ValueLayer::Machine,
        Layer::Kit => ipc::ValueLayer::Kit,
        Layer::Repository => ipc::ValueLayer::Repository,
        Layer::Project => ipc::ValueLayer::Project,
        Layer::Job => ipc::ValueLayer::Job,
    }
}

fn layer_read(word: &str) -> Layer {
    Layer::from_wire(word).unwrap_or(Layer::Default)
}

fn word<T>(one: &ResolvedValue<T>, said: impl Fn(&T) -> String) -> ipc::ResolvedValue {
    ipc::ResolvedValue { value: one.value.as_ref().map(said), from: layer_wire(one.from) }
}

/// Every key as the wire carries it.
pub(crate) fn resolved_wire(resolved: &ResolvedValues) -> ipc::ResolvedValues {
    ipc::ResolvedValues {
        pr_mode: word(&resolved.pr_mode, |mode| mode.as_wire().to_string()),
        auto_merge: word(&resolved.auto_merge, |policy| policy.as_written().to_string()),
        review_model: word(&resolved.review_model, Clone::clone),
        layout: word(&resolved.layout, Clone::clone),
        theme: word(&resolved.theme, Clone::clone),
        density: word(&resolved.density, Clone::clone),
        tracker: Some(ipc::ResolvedTracker { value: resolved.tracker.value.as_ref().map(tracker_wire), from: layer_wire(resolved.tracker.from) }),
        reviews: Some(reviews_wire(
            &resolved.reviews.all(),
            layer_wire(resolved.reviews.nearest_layer()),
            [layer_wire(resolved.reviews.plans.from), layer_wire(resolved.reviews.pull_requests.from), layer_wire(resolved.reviews.walks.from)],
        )),
        agent_starts: Some(word(&resolved.agent_starts, |starts| starts.as_wire().to_string())),
        delivery: Some(word(&resolved.delivery, |delivery| delivery.as_wire().to_string())),
        autonomy: resolved
            .autonomy
            .iter()
            .map(|one| ipc::ResolvedAutonomy { kind: one.kind.as_wire().to_string(), value: one.value.as_wire().to_string(), from: layer_wire(one.from) })
            .collect(),
    }
}

/// The reviews as the wire carries them. `kinds` is the layer of the plans, the pull requests and the walks in turn.
fn reviews_wire(reviews: &Reviews, from: ipc::ValueLayer, kinds: [ipc::ValueLayer; 3]) -> ipc::ResolvedReviews {
    ipc::ResolvedReviews {
        plans: reviews.plans.as_wire().to_string(),
        pull_requests: reviews.pull_requests.as_wire().to_string(),
        walks: reviews.walks.as_wire().to_string(),
        from,
        plans_from: Some(kinds[0]),
        pull_requests_from: Some(kinds[1]),
        walks_from: Some(kinds[2]),
    }
}

/// What a Job freezes, a row for each key.
fn rows_of(resolved: &ResolvedValues) -> Vec<KeptValue> {
    let row = |key: &str, value: Option<String>, from: Layer| KeptValue { key: key.to_string(), value, layer: from.as_wire().to_string() };
    let mut rows = vec![
        row("pr_mode", resolved.pr_mode.value.map(|mode| mode.as_wire().to_string()), resolved.pr_mode.from),
        row("auto_merge", resolved.auto_merge.value.map(|policy| policy.as_written().to_string()), resolved.auto_merge.from),
        row("review_model", resolved.review_model.value.clone(), resolved.review_model.from),
        row("layout", resolved.layout.value.clone(), resolved.layout.from),
        row("theme", resolved.theme.value.clone(), resolved.theme.from),
        row("density", resolved.density.value.clone(), resolved.density.from),
    ];
    let reviews = resolved.reviews.all();
    rows.push(row(REVIEWS_PLANS, Some(reviews.plans.as_wire().to_string()), resolved.reviews.plans.from));
    rows.push(row(REVIEWS_PULL_REQUESTS, Some(reviews.pull_requests.as_wire().to_string()), resolved.reviews.pull_requests.from));
    rows.push(row(REVIEWS_WALKS, Some(reviews.walks.as_wire().to_string()), resolved.reviews.walks.from));
    rows.push(row(AGENT_STARTS, Some(resolved.agent_starts.value.unwrap_or_default().as_wire().to_string()), resolved.agent_starts.from));
    rows.push(row(DELIVERY, resolved.delivery.value.map(|delivery| delivery.as_wire().to_string()), resolved.delivery.from));
    rows.extend(resolved.autonomy.iter().map(|one| row(&format!("{AUTONOMY}{}", one.kind.as_wire()), Some(one.value.as_wire().to_string()), one.from)));
    rows
}

/// What a Job froze, read back from its rows. **The words are the ones [`rows_of`] wrote**.
pub(crate) struct Frozen(Vec<KeptValue>);

impl Frozen {
    /// What a Job froze, `None` where it froze nothing.
    pub(crate) fn of(rows: Vec<KeptValue>) -> Option<Frozen> {
        (!rows.is_empty()).then_some(Frozen(rows))
    }

    fn row(&self, key: &str) -> Option<&KeptValue> {
        self.0.iter().find(|one| one.key == key)
    }

    /// The value of `key` where a layer above the Machine set it. **The Machine's word and the default stay live**:
    /// they are settings a person moves while a Job runs, and a Job reads them where it needs them.
    fn set_above_the_machine(&self, key: &str) -> Option<&str> {
        let row = self.row(key)?;
        (layer_read(&row.layer) > Layer::Machine).then_some(row.value.as_deref()).flatten()
    }

    /// The Job's pull request mode, where Kit, its repository, its project or the Job itself set it.
    pub(crate) fn pr_mode(&self) -> Option<PrMode> {
        self.set_above_the_machine("pr_mode").and_then(PrMode::from_wire)
    }

    /// The model an agent run that reviews the Job's work runs on, where a layer above the Machine set it.
    pub(crate) fn review_model(&self) -> Option<String> {
        self.set_above_the_machine("review_model").map(str::to_string)
    }

    /// How the Job's work is reviewed before the person sees it, as its start said. **Off for a Job that froze
    /// nothing for it**, one that started before the key existed among them: a setting never reaches back.
    pub(crate) fn reviews(&self) -> Reviews {
        let word = |key: &str| self.row(key).and_then(|row| row.value.as_deref());
        Reviews {
            plans: word(REVIEWS_PLANS).and_then(ReviewMode::from_wire).unwrap_or_default(),
            pull_requests: word(REVIEWS_PULL_REQUESTS).and_then(ReviewMode::from_wire).unwrap_or_default(),
            walks: word(REVIEWS_WALKS).and_then(WalkReview::from_wire).unwrap_or_default(),
        }
    }

    /// Every key as the wire carries it, the autonomy kinds in the order of the kinds.
    pub(crate) fn wire(&self) -> ipc::ResolvedValues {
        let one = |key: &str| match self.row(key) {
            Some(row) => ipc::ResolvedValue { value: row.value.clone(), from: layer_wire(layer_read(&row.layer)) },
            None => ipc::ResolvedValue { value: None, from: ipc::ValueLayer::Default },
        };
        ipc::ResolvedValues {
            pr_mode: one("pr_mode"),
            auto_merge: one("auto_merge"),
            review_model: one("review_model"),
            layout: one("layout"),
            theme: one("theme"),
            density: one("density"),
            tracker: None,
            reviews: self.row(REVIEWS_PLANS).map(|_| {
                let kind = |key: &str| self.row(key).map_or(Layer::Default, |row| layer_read(&row.layer));
                let kinds = [kind(REVIEWS_PLANS), kind(REVIEWS_PULL_REQUESTS), kind(REVIEWS_WALKS)];
                // The nearest layer that set a kind: one left to the default is the furthest.
                let nearest = kinds.into_iter().max().unwrap_or(Layer::Default);
                reviews_wire(&self.reviews(), layer_wire(nearest), kinds.map(layer_wire))
            }),
            agent_starts: self.row(AGENT_STARTS).map(|row| ipc::ResolvedValue { value: row.value.clone(), from: layer_wire(layer_read(&row.layer)) }),
            delivery: self.row(DELIVERY).map(|row| ipc::ResolvedValue { value: row.value.clone(), from: layer_wire(layer_read(&row.layer)) }),
            autonomy: core_model::rule::ActKind::ALL
                .iter()
                .filter_map(|kind| {
                    let row = self.row(&format!("{AUTONOMY}{}", kind.as_wire()))?;
                    Some(ipc::ResolvedAutonomy { kind: kind.as_wire().to_string(), value: row.value.clone()?, from: layer_wire(layer_read(&row.layer)) })
                })
                .collect(),
        }
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
    fn values_refused(&self, said: String) -> Refusal {
        Refusal::Unacceptable(self.project_refusal(REFUSED, said))
    }

    /// The values a person wrote on the wire, or the first word that is not one its key takes: a 422
    /// `fleet.project_values_refused`. **Only what is present is set**, and text is trimmed.
    pub(crate) fn values_read(&self, written: &ValuesSet) -> Result<Values, Refusal> {
        let text = |key: &str, held: &Option<String>| match held.as_deref().map(str::trim) {
            Some("") => Err(self.values_refused(format!("{key} is blank, and a value that is set has a word"))),
            Some(held) => Ok(Some(held.to_string())),
            None => Ok(None),
        };
        let auto_merge = match written.auto_merge.as_deref().map(str::trim) {
            None => None,
            Some(word) => Some(AutoMerge::from_written(word).ok_or_else(|| {
                self.values_refused(format!("`{word}` is not an auto merge: it is `never`, `checks-pass` or `always`"))
            })?),
        };
        let look = written.look.clone().unwrap_or_default();
        let density = text("density", &look.density)?;
        if let Some(density) = density.as_deref().filter(|word| !keys::SPACINGS.contains(word)) {
            return Err(self.values_refused(format!("`{density}` is not a density: it is `compact`, `default` or `roomy`")));
        }
        let tracker = match &written.tracker {
            None => None,
            Some(block) => {
                let mut keys = BTreeMap::new();
                for (name, held) in block {
                    match held.trim() {
                        "" => return Err(self.values_refused(format!("tracker.{name} is blank, and a key that is set has a word"))),
                        held => keys.insert(name.trim().to_string(), held.to_string()),
                    };
                }
                let provider = keys.remove(PROVIDER);
                Some(TrackerNamed { provider, keys })
            }
        };
        let reviews = match &written.reviews {
            None => ReviewsNamed::default(),
            Some(block) => self.reviews_read(block)?,
        };
        let agent_starts = match written.agent_starts.as_deref().map(str::trim) {
            None => None,
            Some(word) => Some(AgentStarts::from_wire(word).ok_or_else(|| {
                self.values_refused(format!("`{word}` is not an agent start: it is `at_once` or `ask_me`"))
            })?),
        };
        let delivery = match written.delivery.as_deref().map(str::trim) {
            None => None,
            Some(word) => Some(DeliverySetting::from_wire(word).ok_or_else(|| {
                self.values_refused(format!("`{word}` is not a delivery: it is `auto_land`, `open_pr`, `keep_local` or `ask`"))
            })?),
        };
        Ok(Values {
            tracker,
            reviews,
            agent_starts,
            delivery,
            pr_mode: written.pr_mode.map(|mode| mode.domain()),
            auto_merge,
            review_model: text("review_model", &written.review_model)?,
            look: Look { layout: text("layout", &look.layout)?, theme: text("theme", &look.theme)?, density },
        })
    }

    /// The `reviews` a person wrote, or the first word that is not one its kind takes. **A kind left out is not
    /// set**: it falls through to the next layer.
    fn reviews_read(&self, block: &ReviewsSet) -> Result<ReviewsNamed, Refusal> {
        let of = |kind: &str, held: &Option<String>| -> Result<Option<ReviewMode>, Refusal> {
            match held.as_deref().map(str::trim) {
                None => Ok(None),
                Some(word) => ReviewMode::from_wire(word).map(Some).ok_or_else(|| {
                    self.values_refused(format!("`{word}` is not a review for {kind}: it is `off`, `review` or `review_and_approve`"))
                }),
            }
        };
        let walks = match block.walks.as_deref().map(str::trim) {
            None => None,
            Some(word) => Some(WalkReview::from_wire(word)
                .ok_or_else(|| self.values_refused(format!("`{word}` is not a review for walks: it is `off` or `review`, since a walk is his to look at")))?),
        };
        Ok(ReviewsNamed { plans: of("plans", &block.plans)?, pull_requests: of("pull_requests", &block.pull_requests)?, walks })
    }

    /// What Kit sets: `values.yml` in Kit's folder. **Nothing is set where the file is absent**, and where it
    /// will not read as well, which is said once on the log: a Job is not refused for a file it cannot see.
    pub(crate) fn kit_values(&self) -> Values {
        let path: PathBuf = self.kit_folder().join(KIT_FILE);
        match std::fs::read_to_string(&path) {
            Ok(text) => parse_kit_values(&path, &text).unwrap_or_else(|why| {
                self.values_warned("Kit's values were not read, so Kit sets none", None, why);
                Values::default()
            }),
            Err(_) => Values::default(),
        }
    }

    /// A warning in the envelope on Fleet's standard error: what failed here raises nothing for the person.
    pub(crate) fn values_warned(&self, msg: &str, subject: Option<&str>, why: impl std::fmt::Display) {
        let mut envelope = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), msg);
        if let Some(subject) = subject {
            envelope = envelope.with_field("subject", FieldValue::Str(subject.to_string()));
        }
        crate::transcript::note_aloud(&envelope.with_field("why", FieldValue::Str(why.to_string())));
    }

    /// What the Machine's settings say, **only what is saved there**: an unsaved key defers.
    fn machine_values(&self) -> Values {
        let now = self.machine_settings().now();
        Values {
            pr_mode: now.chosen(keys::DRAFT_PULL_REQUESTS).map(|draft| if draft { PrMode::Draft } else { PrMode::Ready }),
            auto_merge: None,
            review_model: now.chosen(keys::MODELS_REVIEW).filter(|model| !model.trim().is_empty()),
            look: Look {
                layout: now.chosen(keys::LAYOUT).flatten().map(|saved| saved.to_json_text()),
                theme: now.chosen(keys::THEME).filter(|theme| !theme.trim().is_empty()),
                density: now.chosen(keys::SPACING).filter(|density| !density.trim().is_empty()),
            },
            tracker: None,
            reviews: ReviewsNamed::default(),
            agent_starts: None,
            delivery: None,
        }
    }

    /// What Armada ships, with the review model the settings ship.
    fn default_values(&self) -> Values {
        let now = self.machine_settings().now();
        let model = keys::entry(keys::MODELS_REVIEW.name()).map(|entry| keys::Words::read(&now.default_of(entry)));
        Values { review_model: model.filter(|model| !model.trim().is_empty()), ..shipped_values() }
    }

    /// The project `id`'s values, or a refusal: a 404 `fleet.no_such_project` where he has none by that id and
    /// a 422 `fleet.project_unreadable` where its file did not read. **With a repository the project must hold it**
    /// (422 `fleet.project_not_member`), as a Job started in it requires.
    async fn project_values(&self, id: &str, repository: Option<&Repository>) -> Result<Values, Refusal> {
        if let Some(repository) = repository {
            self.project_started_for(id, repository.as_str()).await?;
        }
        self.project_held(id).await.map(|project| project.values().clone())
    }

    /// The project `id` as its file read, or the refusals `project_values` gives.
    pub(crate) async fn project_held(&self, id: &str) -> Result<core_model::Project, Refusal> {
        let read = ScopeId::new(id).ok().and_then(|scope| self.projects_held().get(&scope).cloned());
        match read {
            Some(project) => Ok(project),
            None => {
                let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
                if !kept.projects.iter().any(|one| one.id == id) {
                    return Err(Refusal::NotFound(self.project_refusal(NO_SUCH_PROJECT, format!("Fleet holds no project {id}"))));
                }
                Err(Refusal::Unacceptable(self.project_refusal(UNREADABLE, format!("The file of project {id} did not read, so Fleet cannot say what it sets"))))
            }
        }
    }

    /// Every key for `project` and `repository`, each with the layer it came from. **Either may be absent**: no
    /// project leaves the Project layer empty, and no repository leaves the repository layer and autonomy empty.
    pub(crate) async fn values_resolved(&self, project: Option<&str>, repository: Option<&ManifestId>) -> Result<ResolvedValues, Refusal> {
        let repository = repository.map(|named| self.served_named(Some(named))).transpose()?;
        let repository_id = repository.as_ref().map(|served| Repository::carried(Ulid::carried(served.manifest().id().as_str())));
        let from_project = match project {
            Some(id) => Some(self.project_values(id, repository_id.as_ref()).await?),
            None => None,
        };
        let mut layers = Layers {
            default: self.default_values(),
            machine: self.machine_values(),
            kit: self.kit_values(),
            project: from_project.unwrap_or_default(),
            ..Layers::default()
        };
        if let Some(served) = &repository {
            let manifest = served.manifest();
            layers.repository = Values {
                pr_mode: manifest.pr_mode(),
                auto_merge: manifest.auto_merge_written(),
                tracker: manifest.tracker().map(tracker_named),
                ..Values::default()
            };
            let scope = project.and_then(|id| ScopeId::new(id).ok());
            layers.autonomy = self.autonomy_applying_to(&Repository::carried(Ulid::carried(manifest.id().as_str())))?.layers(scope.as_ref());
        }
        let mut resolved = config::resolve(&layers);
        // No repository, no autonomy: its pieces are read for one.
        if repository.is_none() {
            resolved.autonomy.clear();
        }
        Ok(resolved)
    }

    /// Whether a Job that an agent starts in `repository` waits on the person's start card, for `project` where it
    /// has one. **True only for `ask_me`**: a project that does not read or does not hold the repository falls back
    /// to no project, and a layer that cannot be read is `at_once`, since his work is never held by a fault.
    pub(crate) async fn agent_start_held(&self, project: Option<&str>, repository: &str) -> bool {
        let repository = ManifestId::carried(repository);
        let resolved = match self.values_resolved(project, Some(&repository)).await {
            Ok(resolved) => Ok(resolved),
            Err(_) if project.is_some() => self.values_resolved(None, Some(&repository)).await,
            Err(why) => Err(why),
        };
        resolved.ok().and_then(|resolved| resolved.agent_starts.value) == Some(AgentStarts::AskMe)
    }

    /// `get_values`.
    pub(crate) async fn values_asked(&self, project: Option<String>, repository: Option<ManifestId>) -> Result<ipc::ResolvedValues, Refusal> {
        let project = project.map(|id| id.trim().to_string()).filter(|id| !id.is_empty());
        self.values_resolved(project.as_deref(), repository.as_ref()).await.map(|resolved| resolved_wire(&resolved))
    }

    /// The values a Job starting in `repository` for `project` takes. **`None` where they could not be read**, said
    /// on the log: a start is never refused for it, and the Job reads the live layers where it reads a value.
    pub(crate) async fn values_to_freeze(&self, repository: &str, project: Option<&str>) -> Option<ResolvedValues> {
        match self.values_resolved(project, Some(&ManifestId::carried(repository))).await {
            Ok(resolved) => Some(resolved),
            Err(why) => {
                self.values_warned("a Job's values were not frozen", Some(repository), &why.error().message);
                None
            }
        }
    }

    /// Keep `resolved` as what the Job `id` froze.
    pub(crate) async fn job_values_frozen(&self, id: &str, resolved: &ResolvedValues) {
        if let Err(why) = self.store().lock().await.keep_job_values(id, &rows_of(resolved)) {
            self.values_warned("a Job's values were not kept", Some(id), format!("{why:?}"));
        }
    }

    /// What the Job `id` froze, where it froze any.
    pub(crate) async fn job_values(&self, id: &str) -> Option<Frozen> {
        Frozen::of(self.store().lock().await.job_values(id).ok()?)
    }

    /// How the Job `id` offers a pull request: what a layer above the Machine set at its start, else the Machine's
    /// word as it stands now, else ready.
    pub(crate) async fn pr_mode_of_job(&self, id: &str) -> PrMode {
        match self.job_values(id).await.and_then(|frozen| frozen.pr_mode()) {
            Some(mode) => mode,
            None => self.machine_values().pr_mode.unwrap_or_default(),
        }
    }

    /// `review` as the Job `id` offers it: a draft where its pull request mode says draft.
    pub(crate) async fn review_as_offered_by(&self, id: &str, review: Review) -> Review {
        match self.pr_mode_of_job(id).await {
            PrMode::Draft => review.as_draft(),
            PrMode::Ready => review,
        }
    }

    /// The model an agent run that reviews the Job `id`'s work runs on, where a layer above the Machine set one at
    /// its start. **Absent defers to `models.review`**, which is live.
    pub(crate) async fn review_model_of_job(&self, id: &str) -> Option<String> {
        self.job_values(id).await.and_then(|frozen| frozen.review_model())
    }

    /// `save_kit_values`: Kit's `values.yml`, whole. Nothing is kept where a word is not one its key takes.
    pub(crate) async fn kit_values_saved(&self, save: SaveKitValues) -> Result<KitValuesSaved, Refusal> {
        if save.values.tracker.is_some() {
            return Err(self.values_refused("Kit sets no tracker: a tracker is a project's, or a repository's own".to_string()));
        }
        let values = self.values_read(&save.values)?;
        let path = self.kit_folder().join(KIT_FILE);
        let text = kit_values_text(&values).map_err(|why| self.kit_values_unwritable(&path, why))?;
        let at = path.clone();
        tokio::task::spawn_blocking(move || crate::piece_saving::write_over(&at, &text))
            .await
            .map_err(|why| self.kit_values_unwritable(&path, why))?
            .map_err(|why| self.kit_values_unwritable(&path, why))?;
        Ok(KitValuesSaved { file: path.display().to_string(), values: values_set(&values).unwrap_or_default() })
    }

    fn kit_values_unwritable(&self, path: &std::path::Path, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(self.project_refusal(KIT_UNWRITABLE, format!("{} could not be written: {why}", path.display())))
    }
}
