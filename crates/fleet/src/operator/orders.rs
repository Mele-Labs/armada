//! Standing orders: rules Fleet enforces, never prose a call reads. **Only ever one of the forms in
//! `ipc::operator_talk`**, and only once the person has seen the form and pressed. `crates/ipc/operations/confirm_operator_order.toml`.
//!
//! **A role off** reads as the role's Off position in that repository, until the day ends if he
//! said today. **A kind always raised** goes to him with no call. **A hold on paths** refuses a write
//! under them by any Job but the one it names, in that repository, from a file tool: a shell's writes
//! cannot be read ahead of time and go through. A Job that ends releases its hold, and so does its
//! push where the order said pushes, each with a Log row.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{Actor, Timestamp};
use ipc::operator_positions::OperatorRole;
use ipc::operator_talk::{OperatorHoldUntil, OperatorOrder, OperatorOrderForm, OperatorOrders};
use store::KeptOrder;

use super::{Arrived, LiveJob, Logged, Outcome, Repository};
use crate::daemon::Fleet;

pub(super) const OPERATOR_OFF: &str = "fleet.operator_off";
const ORDER_UNKNOWN: &str = "fleet.operator_order_unknown";
const ORDER_NOT_A_FORM: &str = "fleet.operator_order_not_a_form";

/// One order standing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    pub id: i64,
    pub form: OperatorOrderForm,
    /// When an order for today stops standing.
    pub until: Option<String>,
}

/// The orders standing, as Fleet goes by them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Orders(pub Vec<Order>);

impl Orders {
    /// An order turns `role` off in `manifest` now.
    pub fn role_off(&self, role: OperatorRole, manifest: &str, now: &Timestamp) -> bool {
        self.0.iter().any(|order| {
            let in_force = order.until.as_deref().is_none_or(|until| now.as_str() < until);
            in_force && matches!(&order.form, OperatorOrderForm::RoleOff { role: off, manifest_id, .. } if *off == role && manifest_id == manifest)
        })
    }

    /// An order sends every arrival of this kind to the person with no call.
    pub fn always_raises(&self, arrived: Arrived) -> bool {
        self.0.iter().any(|order| matches!(&order.form, OperatorOrderForm::AlwaysRaise { what } if what == arrived.word()))
    }

    /// The hold that keeps `session` from writing at `path` in `manifest`, if any.
    fn holding(&self, session: &str, manifest: &str, path: &str) -> Option<(&Order, &str, OperatorHoldUntil)> {
        self.0.iter().find_map(|order| match &order.form {
            OperatorOrderForm::HoldPaths { manifest_id, paths, job, until }
                if manifest_id == manifest && job.as_str() != session && paths.iter().any(|held| under(path, held)) =>
            {
                Some((order, job.as_str(), *until))
            }
            _ => None,
        })
    }
}

/// Whether the absolute `path` is the repository-relative `held` or lies under it, in whichever
/// checkout it was written from.
fn under(path: &str, held: &str) -> bool {
    let held = held.trim_matches('/');
    !held.is_empty() && (path.ends_with(&format!("/{held}")) || path.contains(&format!("/{held}/")))
}

/// `form` as it may stand, against the repositories and live Jobs Fleet knows. `Err` says why not.
pub(super) fn checked(form: &OperatorOrderForm, repositories: &[Repository], live: &[LiveJob]) -> Result<(), String> {
    let known = |manifest: &String| repositories.iter().any(|one| one.id == *manifest);
    match form {
        OperatorOrderForm::RoleOff { manifest_id, .. } if !known(manifest_id) => Err(format!("no repository here has the id {manifest_id}")),
        OperatorOrderForm::RoleOff { .. } => Ok(()),
        OperatorOrderForm::HoldPaths { paths, .. } if paths.iter().all(|one| one.trim_matches('/').is_empty()) => Err("a hold names no paths".to_string()),
        OperatorOrderForm::HoldPaths { manifest_id, job, .. } => match live.iter().find(|one| one.session_id == job.as_str()) {
            Some(one) if one.manifest_id == *manifest_id => Ok(()),
            _ => Err(format!("no live Job {} in that repository", job.as_str())),
        },
        OperatorOrderForm::AlwaysRaise { what } => match Arrived::from_word(what) {
            Some(_) => Ok(()),
            None => Err(format!("{what} is not a kind of thing that arrives")),
        },
    }
}

/// The form in Fleet's words, each Job and repository by its name.
pub(super) fn said_of(form: &OperatorOrderForm, repositories: &[Repository], live: &[LiveJob]) -> String {
    let repository = |id: &str| repositories.iter().find(|one| one.id == id).map_or(id, |one| one.name.as_str()).to_string();
    match form {
        OperatorOrderForm::RoleOff { role, manifest_id, today } => {
            let role = match role {
                OperatorRole::Dispatch => "Dispatch",
                OperatorRole::Delegate => "Delegate",
                OperatorRole::Answer => "Answer",
                OperatorRole::File => "File",
            };
            let when = if *today { " today" } else { "" };
            format!("{role} off in {}{when}", repository(manifest_id))
        }
        OperatorOrderForm::HoldPaths { paths, job, until, .. } => {
            let title = live.iter().find(|one| one.session_id == job.as_str()).map_or(job.as_str(), |one| one.title.as_str());
            let event = match until {
                OperatorHoldUntil::Pushes => "pushes",
                OperatorHoldUntil::Ends => "ends",
            };
            format!("Hold {} until {title} {event}", paths.join(", "))
        }
        OperatorOrderForm::AlwaysRaise { what } => match Arrived::from_word(what) {
            Some(arrived) => format!("Always raise {}", arrived.named()),
            None => format!("Always raise {what}"),
        },
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
    /// The orders the store keeps, read into the seat: at start, and after each change. What does
    /// not read as a form stands for nothing.
    pub(crate) async fn operator_orders_read(&self) -> Vec<KeptOrder> {
        let kept = match self.store().lock().await.operator_orders() {
            Ok(kept) => kept,
            Err(why) => {
                self.operator_warned("the standing orders were not read", why);
                return Vec::new();
            }
        };
        let orders = kept
            .iter()
            .filter_map(|one| Some(Order { id: one.n, form: OperatorOrderForm::read(&one.form)?, until: one.until.clone() }))
            .collect();
        self.operator_seat().orders_are(Orders(orders));
        kept
    }

    /// `confirm_operator_order`: the form he saw stands from now on. **Refused** while the Operator
    /// is off, since nothing would go by it, and where it names what Fleet does not know.
    pub(crate) async fn operator_order_confirmed(self: &Arc<Self>, form: OperatorOrderForm) -> Result<OperatorOrders, Refusal> {
        let Some(operator) = self.operator_seat().operator().filter(|_| self.operator_on()) else {
            return Err(self.hosted_refusal(OPERATOR_OFF, "the Operator is off, so nothing would go by a standing order"));
        };
        let (board, live) = operator.board_and_live().await;
        if let Err(why) = checked(&form, &board.repositories, &live) {
            return Err(self.hosted_refusal(ORDER_NOT_A_FORM, &why));
        }
        let now = self.now();
        let until = match &form {
            OperatorOrderForm::RoleOff { today: true, .. } => Some(self.next_day_began(&now).await),
            _ => None,
        };
        let one = KeptOrder { n: 0, form: form.kept(), said: said_of(&form, &board.repositories, &live), until, at: now.as_str().to_string() };
        let kept = self.store().lock().await.add_operator_order(&one);
        kept.map_err(|why| self.ledger_fault(why))?;
        Ok(self.operator_orders_wire().await)
    }

    /// `remove_operator_order`: one press. A hold's Jobs write again at once.
    pub(crate) async fn operator_order_removed(self: &Arc<Self>, id: i64) -> Result<OperatorOrders, Refusal> {
        let at = self.now();
        let removed = self.store().lock().await.remove_operator_order(id, at.as_str()).map_err(|why| self.ledger_fault(why))?;
        if !removed {
            return Err(self.hosted_refusal(ORDER_UNKNOWN, &format!("no standing order {id}")));
        }
        Ok(self.operator_orders_wire().await)
    }

    /// `operator_orders`: the orders standing now. One for a day gone by, and a hold its Job has
    /// released, are let go as they are read.
    pub(crate) async fn operator_orders_wire(&self) -> OperatorOrders {
        let kept = self.operator_orders_read().await;
        let now = self.now();
        let mut orders = Vec::new();
        for one in kept {
            let Some(form) = OperatorOrderForm::read(&one.form) else { continue };
            if one.until.as_deref().is_some_and(|until| now.as_str() >= until) || self.hold_released(one.n, &form).await {
                let _ = self.store().lock().await.remove_operator_order(one.n, now.as_str());
                continue;
            }
            orders.push(OperatorOrder { id: one.n, form, said: one.said, until: one.until, at: one.at });
        }
        self.operator_orders_read().await;
        OperatorOrders { orders }
    }

    /// The local midnight after `now`. **Thirty hours past this day's**, which is always in the
    /// next day whether the clocks change or not.
    async fn next_day_began(&self, now: &Timestamp) -> String {
        const THIRTY_HOURS: i64 = 30 * 60 * 60 * 1000;
        let millis = now.epoch_millis().unwrap_or(0);
        let began = crate::clock::local_day_began(millis).await;
        crate::clock::rfc3339_utc(crate::clock::local_day_began(began + THIRTY_HOURS).await)
    }

    /// Why `session`'s write at `path` is refused, where a hold stands over it. A hold its Job has
    /// released lets the write through and is let go.
    pub(crate) async fn order_holding(&self, session: &str, manifest: &str, path: &str) -> Option<String> {
        let orders = self.operator_seat().orders();
        let (order, job, until) = orders.holding(session, manifest, path)?;
        if self.hold_released(order.id, &order.form).await {
            let _ = self.store().lock().await.remove_operator_order(order.id, self.now().as_str());
            self.operator_orders_read().await;
            return None;
        }
        let title = self.store().lock().await.session(job).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| job.to_string());
        let event = match until {
            OperatorHoldUntil::Pushes => "pushes",
            OperatorHoldUntil::Ends => "ends",
        };
        Some(format!("A standing order holds this path until Job \"{title}\" {event}, so this write is refused. Work elsewhere, or stop and say what you need."))
    }

    /// `form` is a hold whose Job has ended, or pushed where it waits on a push. The Log says so.
    async fn hold_released(&self, id: i64, form: &OperatorOrderForm) -> bool {
        let OperatorOrderForm::HoldPaths { manifest_id, paths, job, until } = form else { return false };
        let (released, subject) = {
            let store = self.store().lock().await;
            let session = store.session(job.as_str()).ok().flatten();
            let ended = session.as_ref().is_none_or(|one| one.state == store::SessionState::Ended);
            let pushed = store.session_work(job.as_str(), "").is_ok_and(|work| work.first_push.is_some());
            let released = match until {
                OperatorHoldUntil::Pushes => ended || pushed,
                OperatorHoldUntil::Ends => ended,
            };
            (released, session.and_then(|one| one.title).unwrap_or_else(|| job.as_str().to_string()))
        };
        if released {
            let mut row = Logged::about(job.as_str(), manifest_id, &subject, Outcome::HoldLifted, Actor::Fleet, self.now());
            row.arrival = format!("order:{id}");
            row.because = format!("The hold on {} lifted: \"{subject}\" moved as the order waited for.", paths.join(", "));
            row.shared = paths.clone();
            self.operator_logged(row).await;
        }
        released
    }
}
