//! An act in the owner's words, the one short line he reads in Needs you: `Move ARM-41 to In
//! progress`, `Start the web deploy watch`. **Said from the act and the instance it is done to**,
//! with a state by the label its lifecycle gives it and a pull request as `#812`.

use core_model::rule::{Act, ForgeWrite, TrackerWrite, Words};
use core_model::{InstanceRef, Lifecycle};

/// The longest a prompt or a message is quoted in a line.
const QUOTED: usize = 60;

/// `act` done to `on`, in the owner's words. `lifecycles` are the ones the Job reads against, for
/// the label of a state.
pub fn said(act: &Act, on: Option<&InstanceRef>, lifecycles: &[Lifecycle]) -> String {
    let thing = on.map(|on| named(on)).unwrap_or_default();
    match act {
        Act::Tracker { write: TrackerWrite::Assign { to }, .. } => format!("Assign {thing} to {to}", to = to.as_str()),
        Act::Tracker { write: TrackerWrite::Move { to }, .. } => {
            let label = on
                .and_then(|on| lifecycles.iter().find(|one| one.id() == on.lifecycle()))
                .and_then(|lifecycle| lifecycle.state(to))
                .map(|state| state.label().as_str().to_string())
                .unwrap_or_else(|| to.as_str().to_string());
            format!("Move {thing} to {label}")
        }
        Act::Tracker { write: TrackerWrite::Close, .. } => format!("Close {thing}"),
        Act::Forge { write: ForgeWrite::RequestReviewers { from }, .. } => {
            let from: Vec<&str> = from.iter().map(Words::as_str).collect();
            format!("Request review on {thing} from {}", from.join(", "))
        }
        Act::Forge { write: ForgeWrite::MarkReady, .. } => format!("Mark {thing} ready"),
        Act::Forge { write: ForgeWrite::Merge, .. } => format!("Merge {thing}"),
        Act::Forge { write: ForgeWrite::AutoMerge, .. } => format!("Turn on auto-merge for {thing}"),
        Act::Draft { template, .. } => format!("Write the {} message on {thing}", template.as_str()),
        Act::StartJob { prompt, .. } => format!("Start a Job: {}", quoted(prompt)),
        Act::StartWatch { deploy: Some(deploy), .. } => format!("Start the {} deploy watch", deploy.as_str()),
        Act::StartWatch { recipe, .. } => format!("Start the {} watch", recipe.as_str()),
        Act::Tell { message } => format!("Tell the agent: {}", quoted(message)),
        Act::Wake => String::from("Wake the agent"),
        Act::RunCommand { command } => format!("Run {}", command.as_str()),
        Act::RunSkill { skill } => format!("Run the {} skill", skill.as_str()),
        Act::RunDrone { prompt } => format!("Start a Drone: {}", quoted(prompt)),
    }
}

/// An act that did not go through, in the same words and what went wrong.
pub fn failed(said: &str, why: &str) -> String {
    format!("{said} failed: {}", why.trim())
}

/// A pull request is `#812`, and anything else goes by its key.
fn named(on: &InstanceRef) -> String {
    match on.lifecycle().as_str() {
        "pr" => format!("#{}", on.key()),
        _ => on.key().to_string(),
    }
}

/// The first line, cut where it runs long.
fn quoted(words: &Words) -> String {
    let first = words.as_str().lines().next().unwrap_or_default().trim();
    match first.char_indices().nth(QUOTED) {
        Some((at, _)) => format!("{}...", first[..at].trim_end()),
        None => first.to_string(),
    }
}
