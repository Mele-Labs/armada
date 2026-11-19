//! The read of one Job's timeline, shared by the Context card and the Tokens tab. What was read stays drawn while the next read is
//! pending, and a read that fails after one landed keeps the one drawn.

use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{Context, Entity, SharedString, Task};

use super::model::Graph;
use super::{mock, wire};
use crate::fleet::FleetConnection;

/// A reading younger than this is not asked for again by a card that opens.
const FRESH: Duration = Duration::from_secs(30);

pub enum Held {
    Reading,
    Failed(String),
    Read(Rc<Graph>),
}

pub struct TimelineRead {
    job: SharedString,
    /// None reads the mock Job.
    fleet: Option<Entity<FleetConnection>>,
    pub held: Held,
    asking: Option<Task<()>>,
    read_at: Option<Instant>,
}

impl TimelineRead {
    pub fn new(job: SharedString, fleet: Option<Entity<FleetConnection>>) -> TimelineRead {
        TimelineRead { job, fleet, held: Held::Reading, asking: None, read_at: None }
    }

    pub fn graph(&self) -> Option<&Rc<Graph>> {
        match &self.held {
            Held::Read(graph) => Some(graph),
            _ => None,
        }
    }

    /// Whether the Job is still running, so its timeline moves.
    pub fn live(&self) -> bool {
        self.graph().is_some_and(|graph| graph.live)
    }

    /// Read where nothing is held, or what is held is old, and not while a read is out.
    pub fn wake(&mut self, cx: &mut Context<Self>) {
        let fresh = self.read_at.is_some_and(|at| at.elapsed() < FRESH);
        if self.asking.is_none() && !fresh {
            self.read(cx);
        }
    }

    /// Ask now. The reading drawn stays until the answer lands.
    pub fn read(&mut self, cx: &mut Context<Self>) {
        match &self.fleet {
            None => self.landed(Ok(mock::timeline()), cx),
            Some(fleet) => {
                let Some(port) = fleet.read(cx).held().port() else {
                    return self.landed(Err("Fleet is not connected".into()), cx);
                };
                let asked = wire::timeline(port, &self.job, cx);
                self.asking = Some(cx.spawn(async move |this, cx| {
                    let answer = asked.await.map_err(|why| why.said);
                    let _ = this.update(cx, |this, cx| this.landed(answer, cx));
                }));
            }
        }
    }

    fn landed(&mut self, answer: Result<ipc::JobTimeline, String>, cx: &mut Context<Self>) {
        self.asking = None;
        self.read_at = Some(Instant::now());
        match answer {
            Ok(timeline) => self.held = Held::Read(Rc::new(Graph::new(&timeline))),
            Err(_) if matches!(self.held, Held::Read(_)) => {}
            Err(why) => self.held = Held::Failed(why),
        }
        cx.notify();
    }
}

#[cfg(test)]
impl TimelineRead {
    /// A reading in a state of its own, which counts as just read so a tab opening on it does not ask again.
    pub fn holding(job: SharedString, held: Held) -> TimelineRead {
        TimelineRead { job, fleet: None, held, asking: None, read_at: Some(Instant::now()) }
    }
}
