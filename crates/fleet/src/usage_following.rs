//! A Job whose usage crossed a rule, told once. `docs/concepts/session.md`, *Where the tokens went*.
//!
//! **Read off what was just ingested.** `turns_followed` names the Jobs whose transcripts gained a
//! turn, and only those are read: an hour's rows and a few more of one Job, by its index. A Job that
//! was not read this round costs nothing.
//!
//! The words are sent the way every Fleet message to a Job is, `told_by_fleet`, so a Job that is
//! ended hears nothing and is read again at its next turn. The trip is written after the words were
//! taken, never before.
//!
//! **The Context rule answers to the owner first.** His answer on the Job's own card (stay and do not
//! ask again, or remind at a figure) wins for that Job; then the Large context setting says ask, carry
//! on automatically, or never. `session_host::handoff_answers`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Timestamp;
use store::KeptTrip;

use crate::daemon::Fleet;
use crate::session_host::handoff_answers::ContextAct;
use crate::session_host::tell_refusal;
use crate::usage_rules::{covered, hour_burn, measured_after, tripped, Limits, Rule};

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
        let mut sessions = sessions.to_vec();
        for held in self.usage_following().waiting_for_quiet() {
            if !sessions.contains(&held) {
                sessions.push(held);
            }
        }
        for session in &sessions {
            let Ok(turns) = self.store().lock().await.turns_for_rules(session, limits.main_turns_needed()) else {
                continue;
            };
            let Some(newest) = turns.last() else { continue };
            let now = Timestamp::from_rfc3339(newest.at.clone());
            self.trips_measured(session, &now).await;
            let over = tripped(&limits, &turns);
            for trip in &over {
                if !self.usage_following().knows(session, trip.rule) {
                    let last = self.store().lock().await.last_usage_rule_trip(session, trip.rule.name());
                    if let Ok(Some(_)) = last {
                        self.usage_following().remembered(session, trip.rule);
                    }
                }
            }
            let mut due = self.usage_following().due(session, &over);
            let mut handing = false;
            let mut reminded = false;
            if let Some(context) = over.iter().find(|trip| trip.rule == Rule::Context) {
                let was_due = due.iter().any(|trip| trip.rule == Rule::Context);
                due.retain(|trip| trip.rule != Rule::Context);
                match self.context_act_of(session, context.figure, was_due).await {
                    ContextAct::Tell { reminded: again } => {
                        reminded = again;
                        due.insert(0, context.clone());
                    }
                    ContextAct::Hand if !self.handing_on_already(session).await => {
                        handing = true;
                        due.insert(0, context.clone());
                    }
                    ContextAct::Hand | ContextAct::Nothing => {}
                }
            }
            for trip in due {
                let context = trip.rule == Rule::Context;
                // Whether the Session has the words: a trip a rule keeps quiet is handled all the same,
                // and is not kept as a tell the Session's reply could be read as refusing.
                let mut sent = false;
                if context && handing {
                    self.handed_on_by_setting(session).await;
                } else {
                    // **One nudge, at a quiet moment**: with anything waiting on him, or a turn running, it
                    // is held and read again, never sent over a decision.
                    if !self.quiet_moment(session).await {
                        self.usage_following().held(session);
                        continue;
                    }
                    match self.noticed_by_fleet(session, &trip.text, ipc::Notice::UsageTrip).await {
                        Ok(told) if told.settled() => sent = told.sent(),
                        _ => continue,
                    }
                }
                let kind = match trip.rule {
                    Rule::Context if !handing => Some(tell_refusal::CONTEXT),
                    Rule::Context => None,
                };
                if let Some(kind) = kind.filter(|_| sent) {
                    self.tell_kept(session, kind).await;
                }
                if context && reminded && sent {
                    self.handoff_reminded(session).await;
                }
                self.usage_following().told(session, trip.rule);
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

    /// Whether a turn has ended and nothing waits on the person: no turn running, no question open or
    /// lapsed, no item, walk, plan or draft awaiting him. The same quiet the stop nudge needs
    /// (`session_host/stopping.rs`).
    async fn quiet_moment(&self, session: &str) -> bool {
        let record = {
            let store = self.store().lock().await;
            let Some(row) = store.session(session).ok().flatten() else { return false };
            let Ok(record) = self.ledger_row(&store, &row) else { return false };
            record
        };
        let held_back = {
            let runtime = self.hosts().of(session);
            let state = runtime.state();
            state.asked.is_some() || matches!(state.turn, ipc::SessionTurn::Working { .. })
        };
        let resting = matches!(record.standing, Some(ipc::SessionStanding::Idle | ipc::SessionStanding::Replied));
        let rows = self.rows_of(session).await.unwrap_or_default();
        resting
            && !held_back
            && record.waiting_for.is_empty()
            && record.running.is_empty()
            && crate::session_host::handoff_answers::lapsed_question(&rows).is_none()
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
