//! A Job whose usage crossed a rule, told once. `docs/concepts/session.md`, *Where the tokens went*.
//!
//! **Read off what was just ingested.** `turns_followed` names the Jobs whose transcripts gained a
//! turn, and only those are read: an hour's rows and a few more of one Job, by its index. A Job that
//! was not read this round costs nothing.
//!
//! The words are sent the way every Fleet message to a Job is, `told_by_fleet`, so a Job that is
//! ended hears nothing and is read again at its next turn. The trip is written after the words were
//! taken, never before.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Timestamp;
use store::KeptTrip;

use crate::daemon::Fleet;
use crate::usage_rules::{covered, hour_burn, measured_after, tripped, Limits};

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
    /// Read the rules against each of `sessions`, whose turns just changed, and tell those that
    /// crossed one they have not been told.
    pub(crate) async fn usage_rules_read(self: &Arc<Self>, sessions: &[String]) {
        let limits = Limits::of(&self.machine_settings().now());
        for session in sessions {
            let Ok(turns) = self.store().lock().await.turns_for_rules(session, limits.main_turns_needed()) else {
                continue;
            };
            let Some(newest) = turns.last() else { continue };
            let now = Timestamp::from_rfc3339(newest.at.clone());
            self.trips_measured(session, &now).await;
            let over = tripped(&limits, &turns);
            for trip in &over {
                if !self.usage_watch().knows(session, trip.rule) {
                    let last = self.store().lock().await.last_usage_rule_trip(session, trip.rule.name());
                    if let Ok(Some(at)) = last {
                        self.usage_watch().remembered(session, trip.rule, &Timestamp::from_rfc3339(at));
                    }
                }
            }
            for trip in self.usage_watch().due(session, &over, &now) {
                if !matches!(self.told_by_fleet(session, &trip.text).await, Ok(true)) {
                    continue;
                }
                self.usage_watch().told(session, trip.rule, &now);
                let kept = KeptTrip {
                    session_id: session.clone(),
                    rule: trip.rule.name().to_string(),
                    at: now.as_str().to_string(),
                    figure: trip.figure,
                    burn_per_hour_before: hour_burn(&turns),
                    burn_per_hour_after: None,
                    after_turns: None,
                };
                let _ = self.store().lock().await.keep_usage_rule_trip(&kept);
            }
        }
    }

    /// Fill in the trips of `session` whose hour after is now covered by its turns, `newest` being
    /// the latest. A trip reads the Job's turns in that one hour by the index, and a Job with
    /// nothing unmeasured costs the one prefix query.
    async fn trips_measured(&self, session: &str, newest: &Timestamp) {
        let Ok(open) = self.store().lock().await.unmeasured_usage_rule_trips(session) else { return };
        for trip in open {
            if !covered(&Timestamp::from_rfc3339(trip.at.clone()), newest) {
                continue;
            }
            let mut store = self.store().lock().await;
            let Ok(after) = store.turns_in_the_hour_after(session, &trip.at) else { continue };
            let measured = measured_after(&after);
            let _ = store.keep_usage_rule_trip_measured(session, &trip.rule, &trip.at, measured.burn_per_hour, measured.turns);
        }
    }
}
