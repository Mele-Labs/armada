//! Which test map collector serves which Check. `docs/concepts/test-map.md`.
//!
//! **The one place a Check's runner is turned into a collector**, so Fleet and the forge's
//! recording ask the same question and get the same answer, and Fleet names no runner.
//! A Check no collector reads here has no feeder, and its files fall to the Manifest's declaration.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::TestCollector;
use checks_runner::CheckWidth;
use core_model::{Narrowing, Runner};

use crate::{NextestCoverage, TypeScriptImports};

/// The runner a Check names for the import collector.
const IMPORTS_FOR: &str = "vitest";

/// The program a Check's command names for the coverage collector.
const COVERAGE_FOR: &str = "nextest";

/// A collector, and which Check and which directory it records.
#[derive(Clone)]
pub struct Feeder {
    check: String,
    collector: Arc<dyn TestCollector>,
    below: Option<String>,
}

impl Feeder {
    /// `collector` records `check` in the checkout it is handed, or below it at `below`.
    pub fn new(check: impl Into<String>, collector: Arc<dyn TestCollector>, below: Option<String>) -> Feeder {
        Feeder { check: check.into(), collector, below }
    }

    pub fn check(&self) -> &str {
        &self.check
    }

    pub fn collector(&self) -> &Arc<dyn TestCollector> {
        &self.collector
    }

    /// The `dir` to hand [`TestCollector::record`] for a checkout at `checkout`. **Collectors
    /// disagree about what it is**: coverage runs in the checkout itself, and the import
    /// reader is handed the Check's directory under the root it was built with.
    pub fn dir_in(&self, checkout: &str) -> String {
        self.below.clone().unwrap_or_else(|| checkout.to_string())
    }
}

/// The collector for the Check `check`, recording in the checkout at `root`, or nothing where
/// none reads it.
///
/// | The Check | Collector |
/// |---|---|
/// | Runs through the `vitest` runner | The import closure, of the runner's directory |
/// | Runs `nextest` and narrows `under` a directory | Coverage, `width` wide |
/// | Anything else | None |
///
/// **A Check with no `under` is not fed by coverage**: the owner's rule is that a Check
/// declaring none has no members to name a lap by.
pub fn collector_for(
    check: &str,
    run: &str,
    runner: Option<&Runner>,
    narrowing: Option<&Narrowing>,
    root: &Path,
    width: CheckWidth,
) -> Option<Feeder> {
    if let Some(runner) = runner.filter(|runner| runner.name() == IMPORTS_FOR) {
        let below = runner.dir().unwrap_or(".").to_string();
        return Some(Feeder::new(check, Arc::new(TypeScriptImports::new(root)), Some(below)));
    }
    let coverage = run.split_whitespace().any(|token| token == COVERAGE_FOR)
        && narrowing.is_some_and(|declared| declared.under().is_some());
    coverage.then(|| Feeder::new(check, Arc::new(NextestCoverage::for_check(narrowing, width)), None))
}
