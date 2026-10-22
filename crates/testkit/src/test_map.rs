//! A collector that hands back fixed laps, for proving what Fleet does with
//! them: the source and commit it stamps, and which source answers when two
//! disagree.

use std::sync::Mutex;

use adapter_traits::{CollectorSource, Lap, LapCost, NotRecorded, TestCollector, TestsToRecord};

pub struct FakeCollector {
    name: String,
    source: CollectorSource,
    laps: Vec<Lap>,
    cost: LapCost,
    rebuilds: bool,
    by_name: bool,
    /// Every `(check, dir, tests)` it was asked to record, in order.
    asked: Mutex<Vec<(String, String, TestsToRecord)>>,
}

impl FakeCollector {
    /// A collector of `source` that answers every record with `laps`, costing
    /// a run and needing no rebuild.
    pub fn handing_back(name: &str, source: CollectorSource, laps: Vec<Lap>) -> FakeCollector {
        FakeCollector {
            name: name.to_string(),
            source,
            laps,
            cost: LapCost::Run,
            rebuilds: false,
            by_name: false,
            asked: Mutex::new(Vec::new()),
        }
    }

    /// Say it needs the thing under test built again, as a measured collector does.
    pub fn needing_a_rebuild(mut self) -> FakeCollector {
        self.rebuilds = true;
        self
    }

    /// Say what a lap costs, in place of a run.
    pub fn costing(mut self, cost: LapCost) -> FakeCollector {
        self.cost = cost;
        self
    }

    /// Hand back only the laps of the tests it is asked for by name, as a real one does.
    pub fn answering_by_name(mut self) -> FakeCollector {
        self.by_name = true;
        self
    }

    pub fn asked(&self) -> Vec<(String, String, TestsToRecord)> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl TestCollector for FakeCollector {
    fn name(&self) -> &str {
        &self.name
    }

    fn source(&self) -> CollectorSource {
        self.source
    }

    fn needs_rebuild(&self) -> bool {
        self.rebuilds
    }

    fn lap_cost(&self) -> LapCost {
        self.cost
    }

    fn record(&self, check: &str, dir: &str, tests: &TestsToRecord) -> Result<Vec<Lap>, NotRecorded> {
        self.asked
            .lock()
            .expect("not poisoned")
            .push((check.to_string(), dir.to_string(), tests.clone()));
        Ok(match tests {
            TestsToRecord::Named(names) if self.by_name => self
                .laps
                .iter()
                .filter(|lap| names.contains(&lap.test))
                .cloned()
                .collect(),
            _ => self.laps.clone(),
        })
    }
}
