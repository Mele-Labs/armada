//! `setup.seed` as the Manifest declares it, and where its warm-up stands. #1064.

use serde::{Deserialize, Serialize};

/// `setup.seed` as the Manifest declares it, and where its warm-up stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredSeed {
    pub paths: Vec<String>,
    /// The Commands `setup.seed.warm` names, in order.
    pub warmed_by: Vec<String>,
    pub warmth: SeedWarmth,
}

/// Whether the seed at the current base commit may be cloned.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SeedWarmth {
    Warm {
        commit: String,
    },
    /// A warm-up is running. A worktree cut now starts cold.
    Warming {
        commit: String,
    },
    /// Not warm, and why, in a sentence.
    Cold {
        why: String,
    },
}
