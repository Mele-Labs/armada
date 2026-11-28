//! A native walk played against the pictures of the look he approved: what `bridge-native --walk
//! <name> --check --against <dir> --out <dir>` writes to `<out>/report.json`, and Fleet reads.
//! `docs/concepts/session/showing-a-window.md`, *The look he approved*.

use serde::{Deserialize, Serialize};

/// Every step of one walk, set against the picture of the same step he approved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LookReport {
    pub walk: String,
    pub steps: Vec<LookStep>,
    /// Any step drifted, or had no picture to be set against.
    pub drifted: bool,
}

/// One step's picture, against the approved one of the same file name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LookStep {
    /// The step's number in the walk, from 1.
    pub step: u32,
    /// The picture's file name, the same in the approved folder and the new one.
    pub file: String,
    pub drifted: bool,
    /// The approved folder holds no picture of this name, or this run made none.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub missing: bool,
    /// The mean difference across the averaged grid, 0 to 1.
    pub mean: f32,
    /// The share of grid cells that differ past the cell threshold, 0 to 1.
    pub cells: f32,
    /// The two pictures side by side, under `--out`, where it drifted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side_by_side: Option<String>,
    /// Where they differ, under `--out`, where it drifted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difference: Option<String>,
}
