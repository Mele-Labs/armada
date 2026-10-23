//! What Fleet hands the modules around it.
//!
//! **Accessors rather than `pub(crate)` fields**, which is the whole subject of
//! the file: a field would be assignable from the other module, and Fleet's own
//! configuration is fixed at assembly. Most of what is here reads something
//! resolved once.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, CiConfiguration, Delivery, SpawnConfigRefused, Trackers, Vcs, WorkProduct,
};
use core_model::{Timestamp, Ulid};
use tokio::sync::Mutex;

use super::{Fleet, Local};
use crate::clock::Clock;
use crate::drone::{environment, HostPaths};
use crate::headroom::{Headroom, Machine};
use crate::judging::Explaining;
use crate::mint::Mint;
use crate::review_runs::Reviewing;
use crate::session_proposer::Proposing;

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
    /// What happened, for the timeline. Public so the composition root hands it to the loops that
    /// see things Fleet does not publish.
    pub fn timeline(&self) -> &crate::timeline::Timeline {
        &self.timeline
    }
    pub(crate) fn store(&self) -> &crate::store_lock::StoreLock {
        &self.store
    }
    pub(crate) fn harness(&self) -> &Arc<H> {
        &self.harness
    }
    pub(crate) fn vcs(&self) -> &Arc<V> {
        &self.vcs
    }
    pub(crate) fn work(&self) -> &W {
        &self.work
    }
    /// Every repository this Fleet serves — `crate::repositories`.
    pub(crate) fn repositories(&self) -> &Arc<crate::repositories::Repositories> {
        &self.repositories
    }
    pub(crate) fn locating(&self) -> &Arc<dyn crate::repositories::Locating> {
        &self.locating
    }
    /// Reading the setup a person already has — `crate::kit`. **Cloned rather
    /// than borrowed**, because the read happens off this thread.
    pub(crate) fn setup(&self) -> Arc<dyn adapter_traits::HarnessSetup + Send + Sync> {
        Arc::clone(&self.setup)
    }
    pub(crate) fn port_range(&self) -> crate::ports::PortRange {
        self.port_range
    }
    pub(crate) fn run_log_retention(&self) -> std::time::Duration {
        self.tuned(|dials| dials.run_log_retention)
    }
    /// How far this machine lets Helm act rather than only read. **The one
    /// place `settings.helm-action-authority-tier-1-redirect-enabled-vs-read-
    /// only` is resolved to a value** — `crate::helm::admitting` is the one
    /// place it is asked, and the brief and the door both follow that answer.
    /// `#943`.
    /// Whether a Session may follow a workflow: `features.workflows`.
    pub(crate) fn workflows_on(&self) -> bool {
        self.tuned(|dials| dials.workflows)
    }
    /// Whether a dispute starts a review, and replies are read for one: `features.disputeReviews`.
    pub(crate) fn dispute_reviews_on(&self) -> bool {
        self.tuned(|dials| dials.dispute_reviews)
    }

    pub(crate) fn helm_authority(&self) -> crate::helm::Authority {
        self.tuned(|dials| dials.helm_authority)
    }
    pub(crate) fn host(&self) -> &Local {
        &self.host
    }
    /// What sends a request and the secrets it is filled with. See [`crate::requesting`].
    pub(crate) fn requests(&self) -> &crate::requesting::Requesting {
        &self.requests
    }
    pub(crate) fn mods(&self) -> &crate::mods::Told {
        &self.mods
    }
    /// settings.json as Fleet holds it. See [`crate::settings`].
    pub(crate) fn machine_settings(&self) -> &crate::settings::MachineSettings {
        &self.settings
    }
    /// The account this machine is signed in to. See [`crate::signing_in`].
    pub(crate) fn signing_in(&self) -> &crate::signing_in::SigningIn {
        &self.signing_in
    }
    /// How a claim on a red main reaches the store. See [`crate::red_claim`].
    pub(crate) fn reaching(&self) -> &Arc<dyn crate::red_claim::Reaching> {
        &self.reaching
    }
    /// One read of the live dials, at one instant. See [`crate::tuning`].
    pub(crate) fn tuned<T>(&self, read: impl FnOnce(&crate::tuning::Tuning) -> T) -> T {
        read(
            &self
                .tuning
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }
    /// The prompts in force, read once by whoever is building one. See
    /// [`crate::prompts`].
    pub(crate) fn prompts(&self) -> crate::prompts::Prompts {
        self.tuned(|dials| dials.prompts.clone())
    }
    /// Put what `settings` chose over the dials as shipped. `crate::settings`' alone.
    pub(crate) fn retuned(&self, settings: &config::settings::Resolved) {
        let tuned = self.shipped_tuning.overlaid_by(settings);
        *self
            .tuning
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = tuned;
    }
    /// How long one Check may run. See [`crate::budget::CheckBudget`].
    pub(crate) fn budget(&self) -> crate::budget::CheckBudget {
        self.tuned(|dials| dials.budget)
    }
    pub(crate) fn usage_following(&self) -> &crate::usage_rules::Following {
        &self.usage_following
    }

    pub(crate) fn behind_told(&self) -> &crate::behind_bay::BehindTold {
        &self.behind_told
    }
    pub(crate) fn logbook_distils(&self) -> &crate::logbook::Distils {
        &self.logbook
    }
    pub(crate) fn review_going(&self) -> &Mutex<std::collections::BTreeSet<String>> {
        &self.review_going
    }

    /// What a plain model call needs in order to ask.
    ///
    /// **The Judge's client, budget and dial**: the call is one turn, no
    /// toolset, no directory, on the cheap model `judge_model` derives and not a
    /// fourth spelling of a vendor's name.
    pub(crate) fn explaining(&self) -> Result<Explaining, SpawnConfigRefused> {
        Ok(Explaining {
            client: Arc::clone(&self.judge),
            budget: self.tuned(|dials| dials.judge_budget),
            model: self.tuned(|dials| dials.judge_model.clone()),
            environment: environment(
                HostPaths {
                    path: &self.host.path,
                    user: &self.host.user,
                    home: &self.host.home,
                    width: self.places_width(),
                },
                &[],
            )?,
        })
    }

    /// What the dispatch path needs in order to ask the proposer.
    ///
    /// **The Judge's client, and the proposer's own budget.** The call is the same call — one
    /// turn, no toolset, no directory — so the client is shared. The wait is not: a Judge's
    /// budget is the only thing that can end a call nobody is watching, and a proposal has a
    /// person who can end it, so a budget tight enough to be that backstop would take the
    /// decision away from them.
    pub(crate) fn proposing(&self) -> Result<Proposing, SpawnConfigRefused> {
        Ok(Proposing {
            client: Arc::clone(&self.judge),
            budget: self.tuned(|dials| dials.proposer_budget),
            model: self.tuned(|dials| dials.proposer_model.clone()),
            environment: environment(
                HostPaths {
                    path: &self.host.path,
                    user: &self.host.user,
                    home: &self.host.home,
                    width: self.places_width(),
                },
                // No worktree, no Manifest ports resolved against one: a Judge and a proposer
                // call carry neither.
                &[],
            )?,
            // The same list `list_models` serves, so what the proposer may pick and what a
            // picker offers cannot disagree.
            choices: self.models_now().models,
        })
    }

    /// What an agent run about a pull request needs in order to ask: the Judge's client, the
    /// proposer's budget (a run reads a repository and writes a long answer, which the Judge's
    /// two minutes would cut), and `models.review`. **No directory**: the run's own bay is the
    /// one thing it may read, and it is leased per run.
    pub(crate) fn reviewing(&self) -> Result<Reviewing, SpawnConfigRefused> {
        Ok(Reviewing {
            client: Arc::clone(&self.judge),
            budget: self.tuned(|dials| dials.proposer_budget),
            model: adapter_traits::Model::named(&self.review_model())?,
            environment: environment(
                HostPaths {
                    path: &self.host.path,
                    user: &self.host.user,
                    home: &self.host.home,
                    width: self.places_width(),
                },
                &[],
            )?,
        })
    }

    /// What writing a Job's retro needs: [`Fleet::explaining`]'s client,
    /// budget and environment, on the retro's own dial.
    ///
    /// **Not the Judge's model.** A retro is read by a person, once per ended
    /// Job, and the cheap tier's was rejected, so it is `retro_model` and a
    /// machine can raise or lower it without moving a Judge's.
    pub(crate) fn writing_retros(&self) -> Result<Explaining, SpawnConfigRefused> {
        Ok(Explaining {
            model: self.tuned(|dials| dials.retro_model.clone()),
            ..self.explaining()?
        })
    }

    /// The roster a Job may name and the harnesses, as Fleet started. Its `default` is the one at
    /// start: [`default_model`](Fleet::default_model) is the one in force.
    pub(crate) fn models(&self) -> &ipc::ModelChoices {
        &self.models
    }
    /// The models the harness offered, as last asked. See [`crate::models_read`].
    pub(crate) fn models_held(&self) -> &crate::models_read::ModelsHeld {
        &self.models_held
    }
    pub(crate) fn phone_held(&self) -> &crate::phone::PhoneHeld {
        &self.phone_held
    }
    pub(crate) fn home_held(&self) -> &crate::home::HomeHeld {
        &self.home_held
    }
    pub(crate) fn downshift(&self) -> &crate::model_downshifting::Holding {
        &self.downshift
    }
    /// The model a proposal that names none is given. **Read when it is asked**: `models.default`
    /// is live, so a change reaches the next Job.
    pub(crate) fn default_model(&self) -> String {
        self.machine_settings().now().get(config::settings::MODELS_DEFAULT)
    }
    /// The model a hosted Session that names none starts on, read when its process starts for
    /// `default_model`'s reason. A Job's Drone never reads it.
    pub(crate) fn session_model(&self) -> String {
        self.machine_settings().now().get(config::settings::MODELS_SESSION)
    }
    /// The model an agent run about a pull request runs on, read when the run starts.
    pub(crate) fn review_model(&self) -> String {
        self.machine_settings().now().get(config::settings::MODELS_REVIEW)
    }
    /// Which tracker and link reader a repository's choice names. Asked
    /// through `crate::tracking`, per repository.
    pub(crate) fn trackers(&self) -> &Arc<dyn Trackers + Send + Sync> {
        &self.trackers
    }
    /// The tracker and what Fleet holds of it — `crate::tracking`.
    pub(crate) fn tracking(&self) -> &crate::tracking::Tracking {
        &self.tracking
    }
    /// What reads a repository's CI configuration, for Scan.
    pub(crate) fn ci_configuration(&self) -> &Arc<dyn CiConfiguration + Send + Sync> {
        &self.ci_configuration
    }
    pub(crate) fn mint(&self) -> &Arc<dyn Mint> {
        &self.mint
    }
    /// **The one clock reading in the crate.** Everything below Fleet takes its
    /// instant as an argument, and this is where the argument comes from.
    pub(crate) fn now(&self) -> Timestamp {
        self.clock.now()
    }
    /// The clock itself, for the one thing that outlives a call and still has
    /// to stamp what it writes: a transcript's writer.
    pub(crate) fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
    pub(crate) fn run(&self) -> &Ulid {
        &self.run
    }
    pub(crate) fn publish(&self, event: ipc::Event) {
        self.events.publish(event);
    }

    pub(crate) fn peers(&self) -> &Arc<dyn crate::peer::PeerOf> {
        &self.peers
    }
    pub(crate) fn hosts(&self) -> &crate::session_host::Hosts {
        &self.sessions
    }
    pub(crate) fn helm(&self) -> &crate::helm::Conversations {
        &self.helm
    }
    pub(crate) fn building(&self) -> &crate::building::Building {
        &self.building
    }
    /// The local language model, loaded or not yet.
    pub fn language_model(&self) -> &crate::language_model::LanguageModel {
        &self.language_model
    }
    pub(crate) fn reply_reads(&self) -> &Arc<crate::session_host::asks_model::Reads> {
        &self.reply_reads
    }
    pub(crate) fn refusal_reads(&self) -> &Arc<crate::session_host::asks_model::Reads> {
        &self.refusal_reads
    }
    pub(crate) fn item_vectors(&self) -> &Arc<crate::session_host::dedup::Vectors> {
        &self.item_vectors
    }
    pub(crate) fn extract_reads(&self) -> &Arc<crate::session_host::asks_model::Reads<crate::session_host::extracting::Extracted>> {
        &self.extract_reads
    }

    /// The machine reader and the headroom in force: what admission asks before a
    /// Drone starts, and what [`Fleet::room`] asks before a Check does.
    pub(crate) fn machine(&self) -> &Arc<dyn Machine> {
        &self.machine
    }
    /// How many Checks run at once on this machine, in force. #284.
    pub(crate) fn checks_at_once(&self) -> crate::places::ChecksAtOnce {
        self.places.at_once()
    }
    /// Put a saved Checks-at-once in force. `crate::limits`' alone.
    pub(crate) fn rechecked(&self, at_once: crate::places::ChecksAtOnce) {
        self.places.limit(at_once);
    }

    /// How wide one Check may run on this machine right now. #1444.
    pub(crate) fn places_width(&self) -> checks_runner::CheckWidth {
        self.places.width()
    }

    /// Put a width in force after a saved Jobs bound moved — the two are one
    /// number, so `crate::limits` sets this wherever it rebounds the roster.
    /// #1444.
    pub(crate) fn rewidened(&self, bound: crate::slots::Concurrency) {
        self.places
            .widen(checks_runner::CheckWidth::read(bound.jobs()));
    }
    /// The headroom in force, **by value**: a save replaces it, so a borrow
    /// would be a lock held across whatever the caller did next.
    pub(crate) fn headroom(&self) -> Headroom {
        *self
            .headroom
            .lock()
            .expect("the headroom lock is not poisoned")
    }
    /// Put a saved headroom in force. `crate::limits`' alone.
    /// The Jobs bound in force.
    pub(crate) fn concurrency(&self) -> crate::slots::Concurrency {
        *self
            .concurrency
            .lock()
            .expect("the concurrency lock is not poisoned")
    }
    pub(crate) fn reconcurred(&self, bound: crate::slots::Concurrency) {
        *self
            .concurrency
            .lock()
            .expect("the concurrency lock is not poisoned") = bound;
    }
    pub(crate) fn rehoused(&self, headroom: Headroom) {
        *self
            .headroom
            .lock()
            .expect("the headroom lock is not poisoned") = headroom;
    }
    /// The limits the composition root handed in, before anything was saved.
    pub(crate) fn shipped(&self) -> crate::limits::Limits {
        self.shipped
    }
    pub(crate) fn noticing(&self) -> crate::noticing::Noticing {
        self.tuned(|dials| dials.noticing)
    }
    pub(crate) fn rehearsals(&self) -> &crate::rehearsing::Rehearsals {
        &self.rehearsals
    }
    pub(crate) fn watches(&self) -> &crate::watches::Watches {
        &self.watches
    }
    pub(crate) fn servers(&self) -> &crate::servers::Servers {
        &self.servers
    }
    pub(crate) fn worktrees_memory(&self) -> &crate::worktrees_kept::Kept {
        &self.kept
    }
    pub(crate) fn sweeping(&self) -> &Mutex<crate::noticing::Sweep> {
        &self.sweeping
    }
    pub(crate) fn writes(&self) -> &Arc<crate::clashing::Writes> {
        &self.writes
    }
    pub(crate) fn feed(&self) -> &Arc<crate::test_map_feed::Feed> {
        &self.feed
    }
    /// A room in the machine's line of places, asking as `asking`: what a heavy run outside a
    /// Check holds around itself.
    pub(crate) fn room_asking(&self, asking: crate::places::Asking) -> crate::places::Room {
        crate::places::Room::sharing(&self.places, asking, Arc::clone(&self.machine), self.headroom())
    }
    /// How many of the machine's places are held right now.
    #[cfg(test)]
    pub(crate) fn places_held(&self) -> usize {
        self.places.held()
    }
    pub(crate) fn territory(&self) -> &Arc<crate::territory::Territory> {
        &self.territory
    }

    /// The rebase-and-push tail's lock. See the field.
    pub(crate) fn merge_end(&self) -> &Mutex<()> {
        &self.merge_end
    }

}
