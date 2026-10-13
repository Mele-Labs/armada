//! What a Job's Commander is told when its session starts, and how each turn
//! after it is worded. `#2166`.
//!
//! The wording is `docs/contracts/agent-prompt.md` section 5e, transcribed. A
//! change to it belongs in the contract first. What is here ships; a person may
//! override it in settings.json as `prompts.commander`, which must keep `{job}`.
//!
//! **The brief says what the launch already enforces, and why.** The launch
//! cuts every write tool (`adapters::Commanding`) and the door narrows its acts
//! to its own Job (`api::CommanderReach`); a model told only that it may not is
//! still told, so it stops reaching for them rather than retrying each.

use config::settings::PROMPT_COMMANDER;

use super::conversation::Input;
use crate::prompts::{fill, Prompts};

const OPENING: &str = "\
You are the Commander of one Job, in Armada. A Job is work that Drones do in a \
worktree of its own, one step after another, and Checks and a Judge decide \
whether each step is done. Your part is to get this Job through: you read what \
it has done and what is in its way, and you steer its Drones, its plan and its \
caps. This Job is {job}.";

const THE_WORKTREE: &str = "\
THE WORKTREE

Your working directory is this Job's worktree, as the Drones left it. Read it \
and search it. You have no tool that edits a file or runs a command, and you do \
not ask for one: the Drones do the work, and you do not do a step yourself.";

const EACH_TURN: &str = "\
EACH TURN

A turn begins because Fleet saw trouble on this Job, because the owner wrote \
to you, or because he answered your card, and the turn says which. Start by calling get_events_since with the \
upto cursor your last call answered with, or 0 on your first turn. Then read \
the Job, with get_job and the other reads, before you act on it. Everything a \
tool returns stays in this conversation for the rest of it.";

const MAY_DO: &str = "\
WHAT YOU MAY DO

On this Job and no other: raise its turn cap or its cost cap, up to three times \
what its workflow allows; redirect a Drone with redirect_drone or \
redirect_one_drone; restart a step or a task; send a repair Drone with \
send_repair_drone where the Job's worktree or environment is broken, naming \
what broke: a merge conflict, dependencies, a missing binary or the worktree \
itself; and change the plan with add_task, edit_task, drop_task and move_plan. \
A repair Drone works on this Job's own branch, and the Job's Checks and Judge \
rule on its work as on any step's. Each act is recorded as yours, and the owner \
sees it on the Job. A call that names another Job is refused.";

const MAY_NOT: &str = "\
WHAT YOU MAY NOT DO

Pass a gate, attest that work is complete, merge, push, or edit a Check or the \
Manifest. Those are the owner's and you hold no tool for them.";

const WHAT_YOU_SAY: &str = "\
WHAT YOU SAY

Nothing you say advances the Job. The Checks and the Judge decide whether work \
is done, and a sentence from you that it is done is evidence of nothing. Say \
what you read, what you changed and why, and cite the step, the Drone or the \
Check each rests on. Never say that work is finished or passing. When the owner \
wrote to you, answer what he asked first.";

const WHEN_STUCK: &str = "\
WHEN YOU CANNOT FIX IT

Where the fix is out of your reach, or you have been woken three times on one \
problem and the Job has not moved, stop acting and send the owner one card with \
ask_the_owner. It takes what is wrong, what is out of your reach, one question, \
and two to four options he picks from. A reply in this conversation is not a \
card, and he may not read it. One card is out at a time, so while it is, send \
no other and say nothing more about it. Fleet wakes you with his pick when he \
makes it, and the turn says which option. Fleet sends a card itself once it has \
woken you three times on one problem. Do not raise a cap, restart a step or \
redirect a Drone a fourth time on the same problem.";

/// A Commander's brief as it ships, `{job}` unfilled.
pub(crate) fn shipped() -> String {
    [
        OPENING,
        THE_WORKTREE,
        EACH_TURN,
        MAY_DO,
        MAY_NOT,
        WHAT_YOU_SAY,
        WHEN_STUCK,
    ]
    .join("\n\n")
}

/// The brief, with the Job named.
pub(crate) fn worded(prompts: &Prompts, job: &str) -> String {
    fill(prompts.get(PROMPT_COMMANDER), &[("job", job)])
}

/// One turn's words: why it was wanted, **each reason once**. The owner's
/// words are quoted as his, in the order they came; a wake is Fleet's line.
/// Wakes that said the same thing while a turn ran are one line, and that is
/// all a coalesced wake is.
pub(crate) fn turn_of(inputs: &[Input]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for input in inputs {
        let line = match input {
            Input::Owner(text) => format!("The owner wrote to you: {}", text.trim()),
            Input::Wake(why) => format!("Fleet woke you: {}", why.trim()),
        };
        if !lines.contains(&line) {
            lines.push(line);
        }
    }
    lines.join("\n\n")
}

/// A turn as its session is told it: after the brief on a new session, alone
/// on one that already heard the brief.
pub(crate) fn told(
    prompts: &Prompts,
    job: &str,
    inputs: &[Input],
    resuming: bool,
) -> String {
    let turn = turn_of(inputs);
    match resuming {
        true => turn,
        false => format!("{}\n\n{turn}", worded(prompts, job)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unwrapped(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// **The contract's drafted wording, whole**: section 5e, with `{job}`
    /// filled.
    #[test]
    fn a_commander_is_told_the_contracts_brief() {
        let told = worded(&Prompts::shipped(), "14 \"bound the reader\"");
        let contract = include_str!("../../../../docs/contracts/agent-prompt.md");
        let section = contract
            .split("# 5e. The Commander brief")
            .nth(1)
            .expect("section 5e");
        let drafted = section
            .split("```\n")
            .nth(1)
            .expect("the drafted block")
            .trim_end()
            .replace("{job}", "14 \"bound the reader\"");
        assert_eq!(unwrapped(&told), unwrapped(&drafted));
    }

    /// The brief is told once, and a resumed session hears the turn alone.
    #[test]
    fn a_resumed_session_is_told_the_turn_and_not_the_brief_again() {
        let inputs = [Input::Owner("why".into())];
        let first = told(&Prompts::shipped(), "14", &inputs, false);
        let again = told(&Prompts::shipped(), "14", &inputs, true);
        assert!(first.starts_with("You are the Commander of one Job"));
        assert_eq!(again, "The owner wrote to you: why");
    }

    /// **It never claims work done, and says so as a rule.** The Checks and the
    /// Judge decide, and the brief is where a Commander is told it is not one.
    #[test]
    fn the_brief_says_nothing_it_says_advances_a_job_and_names_the_loop_bound() {
        let brief = shipped();
        assert!(brief.contains("Nothing you say advances the Job"));
        assert!(brief.contains("Never say that work is finished or passing"));
        assert!(brief.contains("woken three times on one problem"));
        assert!(brief.contains("a fourth time"));
        assert!(
            brief.contains("send the owner one card with ask_the_owner"),
            "a Commander that cannot fix it sends a card and does not just say so"
        );
    }

    /// Every tool the brief names is one the door lets a Commander call, so
    /// the brief and `daemon/commander.rs` cannot drift to a tool that is
    /// refused.
    #[test]
    fn every_act_the_brief_names_is_on_the_allowlist() {
        for tool in [
            "redirect_drone",
            "redirect_one_drone",
            "add_task",
            "edit_task",
            "drop_task",
            "move_plan",
            "ask_the_owner",
            "send_repair_drone",
        ] {
            assert!(shipped().contains(tool), "the brief names {tool}");
            assert!(api::COMMANDER_ALLOWED.contains(&tool), "{tool}");
        }
    }
}
