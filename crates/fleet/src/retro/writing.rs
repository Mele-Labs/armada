//! The shared half of writing a retro: the two prompt texts both calls are
//! given, and reading a model's answer back as items held to the record.
//! `session_writing` has the call itself.

use std::collections::BTreeSet;

use ipc::RetroWritten;
use store::RetroLine;

/// Where a fix lands, in the words both calls are given: what ships as
/// `prompts.retroWhereFixesLand`.
pub(crate) const LANDS_IN: &str = "\
         Say separately where the fix for each item lands, as `lands_in`. It is exactly one \
         of three, and it is a different question from whose way it got in:\n\
         - `armada`: Armada itself, the app that ran the job. For example the gate measured \
         the job's change against the wrong base, ruled a check failed without confirming \
         it, or showed an agent running after it had ended.\n\
         - `kit`: the tools the agent was given: its skills, MCP servers, sub-agents, agent \
         files, plugins, commands, the list of commands it is allowed to run, and the list \
         of models. For example a command the agent was refused, or had to ask a person to \
         allow.\n\
         - `manifest`: the repository the job worked on: its `armada.yml` (its checks, \
         commands, places and when each runs), its tests and its code. For example a test \
         that waits a fixed number of seconds, or a check that runs every test on a change \
         to documentation alone.\n\
         An item names one place. Where a fix would land in two, write two items.\n\n";

/// The three texts of an item, the Kit change and how to write them, in the words both calls are given:
/// what ships as `prompts.retroHowToWrite`.
pub(crate) const TEXTS: &str = "\
         Each item has three texts:\n\
         - `title`: a headline of about eight words, with no period.\n\
         - `what`: one or two short sentences saying what happened and to whom.\n\
         - `fix`: one sentence naming what to change and where.\n\n\
         A `kit` item may also carry a `change`, and only when its fix is to allow a command \
         the record shows the agent was refused. Name that refusal by its `cite`, and Armada \
         reads the command from the record: \
         {\"kind\":\"allow_command\",\"refusal\":\"refusal:2\"}. Never write the command \
         yourself. Add `\"command\"` only to allow a shorter start of what was refused, such \
         as `grep -a -c`, and it must be the first words of what the refusal tried. Leave \
         `change` out of every other item.\n\n\
         How to write the three texts. A person reads them, and has rejected text that \
         sounds machine written:\n\
         - Short sentences with concrete facts: names, files, counts, durations.\n\
         - Plain verbs: is, has, ran, wrote. Not \"serves as\" or \"stands as\".\n\
         - No dashes of any kind. Use a period or a comma. A hyphen inside a word is fine.\n\
         - No \"not X but Y\", \"not just X\" or \"this is not about X\". Say what is.\n\
         - No lists of three for rhythm. Name the items that exist.\n\
         - No stock words: crucial, key, pivotal, robust, delve, landscape, highlight, \
         underscore, ensure, additionally, valuable, testament.\n\
         - No trailing phrases such as \"highlighting\", \"ensuring\" or \"reflecting\".\n\
         - No vague \"associated with\", \"linked to\" or \"related to\". Name the relation.\n\
         - No one line closer, and no sentence that repeats the one before it.\n\
         - `what` must not restate `title`. Its first sentence adds a fact the title lacks.\n\
         - No advice in `what`. The change goes in `fix`.\n\
         - No hedges such as \"could potentially\" or \"may arguably\".\n\
         - Active voice. Say who acted.\n\
         - `fix` is one imperative sentence, such as \"Fetch main before the gate measures.\"\n\
         - No filler: no opening that announces the point and no sentence about significance.\n\n";

/// Whether a text holds a dash: an em dash, an en dash, a spaced hyphen or a
/// double hyphen. **The owner's rule for what a retro says**, and a hyphen
/// inside a word is not one.
fn dashed(text: &str) -> bool {
    text.contains('\u{2014}')
        || text.contains('\u{2013}')
        || text.contains(" - ")
        || text.contains("--")
}

/// The items an answer names, each held to the record: an item citing nothing
/// the record holds is dropped, and so is one that lacks a title, what or fix,
/// one with a dash in any of them, and one that names no place its fix lands.
/// **The place is never defaulted**: a guess would list it under a place the
/// model did not choose.
pub(crate) fn read(
    said: &str,
    known: &BTreeSet<String>,
) -> Result<Vec<RetroLine>, String> {
    let from = said.find('{').ok_or("there is no JSON object in it")?;
    let to = said
        .rfind('}')
        .filter(|to| *to > from)
        .ok_or("there is no JSON object in it")?;
    let written: RetroWritten =
        ipc::decode("a retro", said[from..=to].as_bytes()).map_err(|why| why.to_string())?;
    Ok(written
        .items
        .into_iter()
        .filter_map(|item| {
            let evidence: Vec<String> = item
                .evidence
                .into_iter()
                .filter(|cited| known.contains(cited))
                .collect();
            let lands_in = item.lands_in?;
            let title = item.title.trim().to_string();
            let what = item.what.trim().to_string();
            let fix = item.fix.trim().to_string();
            let written = [&title, &what, &fix]
                .iter()
                .all(|text| !text.is_empty() && !dashed(text));
            (!evidence.is_empty() && written).then(|| RetroLine {
                change: None,
                whose: item.who.domain(),
                title: Some(title),
                said: what.clone(),
                what: Some(what),
                fix: Some(fix),
                evidence,
                lands_in: Some(lands_in.domain()),
            })
        })
        .collect())
}
