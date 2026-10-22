//! A collector that hands back fixed laps, for proving what Fleet does with
//! them: the source and commit it stamps, and which source answers when two
//! disagree.

use std::sync::Mutex;

use adapter_traits::{CollectorSource, Lap, LapCost, NotRecorded, TestCollector, TestsToRecord};

pub struct FakeCollector {
    name: String,
    source: CollectorSource,
    laps: Vec<Lap>,
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
            asked: Mutex::new(Vec::new()),
        }
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
        false
    }

    fn lap_cost(&self) -> LapCost {
        LapCost::Run
    }

    fn record(&self, check: &str, dir: &str, tests: &TestsToRecord) -> Result<Vec<Lap>, NotRecorded> {
        self.asked
            .lock()
            .expect("not poisoned")
            .push((check.to_string(), dir.to_string(), tests.clone()));
        Ok(self.laps.clone())
    }
}
