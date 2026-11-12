//! Seating the Operator: once, at start, on the Operator model with one call kept started. **A Job's
//! own events are never handed to it**, so nothing a Job does waits on its ruling.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Model, Vcs, WorkProduct};
use config::settings as keys;
use core_model::{Component, Envelope, FieldValue, Level};

use super::{Blind, Calling, FleetActing, FleetDesk, KeptStarted, Looking, Operator, ReadingScout};
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
    /// Seat the Operator, its call kept started on the Operator model as settings.json names it
    /// at each call. A Fleet whose call cannot be assembled seats none, and every arrival goes on
    /// as it did before the Operator.
    pub fn operator_seated(self: &Arc<Self>) {
        let proposing = match self.proposing() {
            Ok(proposing) => proposing,
            Err(why) => {
                self.operator_warned("the Operator was not seated", format!("{why:?}"));
                return;
            }
        };
        let model_now = |fleet: std::sync::Weak<Self>| move || Model::named(&fleet.upgrade()?.machine_settings().now().get(keys::MODELS_OPERATOR)).ok();
        let call = Arc::new(KeptStarted::new(proposing.client, model_now(Arc::downgrade(self)), proposing.environment, proposing.budget));
        // A message to the box reads; a ruling does not.
        let looking = Arc::new(ReadingScout::new(self.scouts().host(), model_now(Arc::downgrade(self))));
        let fleet = Arc::clone(self);
        tokio::spawn(async move { fleet.operator_kept(call, looking).await });
    }

    /// Seat an Operator ruling through `call`, kept started while the switch is on, and putting a message to `looking`.
    pub(crate) async fn operator_kept(self: &Arc<Self>, call: Arc<KeptStarted>, looking: Arc<dyn Looking>) {
        if self.operator_seated_looking(call.clone(), looking) {
            self.operator_seat().keep(call);
            self.operator_kept_in_step().await;
        }
    }

    /// The call kept started while the Operator is on, and stopped while it is off. Once a tick, so
    /// a switch turned live is followed within one.
    pub async fn operator_kept_in_step(&self) {
        let Some(call) = self.operator_seat().kept() else { return };
        match (self.operator_on(), call.is_waiting().await) {
            (true, false) => {
                if let Err(why) = call.ready().await {
                    self.operator_warned("the Operator's call did not start ahead of its next arrival", why);
                }
            }
            (false, true) => call.stopped().await,
            _ => {}
        }
    }

    /// What went wrong for the Operator, in Fleet's log.
    pub(super) fn operator_warned(&self, msg: &str, why: impl std::fmt::Display) {
        let envelope = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), msg)
            .with_field("why", FieldValue::Str(why.to_string()));
        crate::transcript::note_aloud(&envelope);
    }

    /// What the Operator did that nobody is told of, in Fleet's log so it can be traced.
    pub(super) fn operator_noted(&self, msg: &str, why: impl std::fmt::Display) {
        let envelope = Envelope::new(self.now(), Level::Info, Component::Fleet, self.run().clone(), msg)
            .with_field("why", FieldValue::Str(why.to_string()));
        crate::transcript::note_aloud(&envelope);
    }

    /// Seat an Operator making its calls through `call`. `false` where one is already seated.
    pub(crate) fn operator_seated_with(self: &Arc<Self>, call: Arc<dyn Calling>) -> bool {
        self.operator_seated_looking(Arc::clone(&call), Arc::new(Blind(call)))
    }

    /// Seat an Operator ruling through `call` and putting a message to `looking`. `false` where one is already seated.
    pub(crate) fn operator_seated_looking(self: &Arc<Self>, call: Arc<dyn Calling>, looking: Arc<dyn Looking>) -> bool {
        let fleet = Arc::downgrade(self);
        let desk = FleetDesk::on(self, move |why| {
            if let Some(fleet) = fleet.upgrade() {
                fleet.operator_warned("the Operator's Log was not kept or read", why);
            }
        });
        self.operator_seat().seat(Operator::new_looking(call, looking, Arc::new(desk), Arc::new(FleetActing::on(self))))
    }
}
