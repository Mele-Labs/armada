//! Mock data for an artifact of parts: what the mock Fleet serves in `JobArtifact.parts` and how it takes an answer.
//!
//! `open` holds one part of each kind in the order a Job adds them: words that ask, takes of three with the Job's
//! pick on the second, a drawing whose steps stand at different places, buttons and a page. `answered` is the same
//! artifact with every ask answered. `apply` is the mock's `answer_part`: it refuses what Fleet refuses, in the
//! sentences Fleet uses, so a refusal on a part's row can be walked.

use std::collections::BTreeMap;

use ipc::{
    AnswerPart, ArtifactBy, ArtifactFor, ArtifactForm, ArtifactPart, ArtifactRevision, ArtifactState, Instant, JobArtifact, PageAct, PagePress, PartAction, PartAnswer, PartContent, PartHow,
    PartState, PartTake, PressDid, VoteForm,
};

/// The Job the mock artifacts belong to, in the tool-rows scenario.
pub const JOB: &str = "tool-edit";
/// The artifact's id and title.
pub const ID: &str = "sketch:Step bar for Jobs";
pub const TITLE: &str = "Step bar for Jobs";
/// The page the page part holds, which the mock Fleet serves as its design page.
pub const PAGE: &str = "design";

fn at(minute: u32) -> Instant {
    Instant::carried(format!("2026-11-14T10:{minute:02}:00Z"))
}

/// A take's picture as a scene's JSON text: the boxes at their places, which the scene settles apart.
fn bar() -> String {
    r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"w":120,"h":40,"title":"Plan","color":"blue"},{"id":"b","kind":"box","x":160,"y":0,"w":120,"h":40,"title":"Build","color":"blue"},{"id":"c","kind":"box","x":320,"y":0,"w":120,"h":40,"title":"Land","color":"blue"}],"edges":[]}"#.into()
}

fn rail() -> String {
    r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"w":120,"h":40,"title":"Plan","color":"teal"},{"id":"b","kind":"box","x":0,"y":80,"w":120,"h":40,"title":"Build","color":"teal"},{"id":"c","kind":"box","x":0,"y":160,"w":120,"h":40,"title":"Land","color":"teal"}],"edges":[]}"#.into()
}

fn dots() -> String {
    r#"{"nodes":[{"id":"a","kind":"box","shape":"circle","x":0,"y":0,"w":48,"h":48,"color":"amber"},{"id":"b","kind":"box","shape":"circle","x":96,"y":0,"w":48,"h":48,"color":"amber"},{"id":"c","kind":"box","shape":"circle","x":192,"y":0,"w":48,"h":48,"color":"amber"}],"edges":[]}"#.into()
}

/// The drawing part's scene: four steps, each at a different place.
fn flow() -> String {
    r#"{"nodes":[{"id":"plan","kind":"box","x":0,"y":0,"w":130,"h":44,"title":"Plan","color":"green","state":"done"},{"id":"build","kind":"box","x":200,"y":0,"w":130,"h":44,"title":"Build","color":"blue","state":"running"},{"id":"walk","kind":"box","x":400,"y":0,"w":130,"h":44,"title":"Walk","color":"amber","state":"waiting"},{"id":"land","kind":"box","x":600,"y":0,"w":130,"h":44,"title":"Land","color":"grey","state":"checking"}],"edges":[{"id":"e1","from":"plan","to":"build"},{"id":"e2","from":"build","to":"walk"},{"id":"e3","from":"walk","to":"land"}]}"#.into()
}

fn part(id: &str, content: PartContent, asks: bool, minute: u32) -> ArtifactPart {
    ArtifactPart { id: id.into(), content, asks, state: PartState::Open, revision: 1, drawn: format!("{id}-1"), added: at(minute), answer: None }
}

fn take(id: &str, title: &str, scene: String) -> PartTake {
    PartTake { id: id.into(), title: title.into(), scene: Some(scene) }
}

/// The parts, in the order they were added. `answered` has every ask answered, as he would have.
fn parts(answered: bool) -> Vec<ArtifactPart> {
    let mut parts = vec![
        part("intro", PartContent::Words { text: "Each Job shows its workflow as steps under its title. Two ways to draw them, and a third that drops the words.".into() }, true, 0),
        part(
            "bar",
            PartContent::Takes {
                question: Some("Which step bar reads better?".into()),
                form: VoteForm::Pick,
                takes: vec![take("bar", "A \u{b7} Bar under the title", bar()), take("rail", "B \u{b7} Rail (Job's pick)", rail()), take("dots", "C \u{b7} Dots", dots())],
                parts: Vec::new(),
            },
            true,
            1,
        ),
        part("flow", PartContent::Drawing { scene: flow() }, answered, 2),
        part(
            "try",
            PartContent::Buttons {
                actions: vec![PartAction {
                    id: "try".into(),
                    act: PageAct::Message,
                    label: "Try it".into(),
                    item: Some("Try the rail on this Job now".into()),
                    brief: None,
                    repository: None,
                    ask: None,
                    option: None,
                }],
            },
            false,
            3,
        ),
        ArtifactPart { revision: 2, drawn: "2".into(), ..part(PAGE, PartContent::Page { path: PAGE.into() }, false, 4) },
    ];
    if answered {
        let done = |part: &mut ArtifactPart, state: PartState, answer: PartAnswer| {
            part.state = state;
            part.answer = Some(PartAnswer { at: Some(at(30)), ..answer });
        };
        done(&mut parts[0], PartState::Approved, PartAnswer { words: Some("Good, keep the title short.".into()), ..PartAnswer::default() });
        done(&mut parts[1], PartState::Picked, PartAnswer { take: Some("rail".into()), words: Some("Keep the step names.".into()), ..PartAnswer::default() });
        done(&mut parts[2], PartState::Approved, PartAnswer::default());
    }
    parts
}

/// The artifact as `get_job_artifacts` serves it, asking or settled.
pub fn artifact(answered: bool) -> JobArtifact {
    let parts = parts(answered);
    let kept = |number| ArtifactRevision { number, at: at(number), by: ArtifactBy::Agent, form: ArtifactForm::Draft };
    let press = PagePress { revision: 1, action: "try".into(), item: Some("Try the rail on this Job now".into()), act: PageAct::Message, at: at(31), did: PressDid::Sent };
    JobArtifact {
        id: ID.into(),
        title: TITLE.into(),
        form: ArtifactForm::Draft,
        revisions: vec![kept(1), kept(2)],
        state: if answered { ArtifactState::Approved } else { ArtifactState::Asking },
        words: None,
        item: (!answered).then(|| ID.into()),
        in_plan: None,
        archived: None,
        for_line: Some(ArtifactFor { text: Some("picks the step bar, and the PR text that ships it.".into()), links: Vec::new() }),
        presses: if answered { vec![press] } else { Vec::new() },
        votes: Vec::new(),
        seen: None,
        parts,
    }
}

/// What the artifact of parts reads as once its asks are in: asking while any is open, then sent back where any part
/// was, else approved. Fleet's own rule, which the mock repeats for the artifact it keeps.
pub fn state_of(parts: &[ArtifactPart]) -> ArtifactState {
    if parts.iter().any(ArtifactPart::is_open_ask) {
        return ArtifactState::Asking;
    }
    let answered: Vec<&ArtifactPart> = parts.iter().filter(|part| part.asks && !matches!(part.state, PartState::Open | PartState::Withdrawn)).collect();
    match (answered.is_empty(), answered.iter().any(|part| part.state == PartState::SentBack)) {
        (true, _) => ArtifactState::Reading,
        (false, true) => ArtifactState::SentBack,
        (false, false) => ArtifactState::Approved,
    }
}

/// The mock's `answer_part`: the part answered in place, or Fleet's refusal in Fleet's words.
pub fn apply(artifact: &mut JobArtifact, said: &AnswerPart) -> Result<(), String> {
    let ids: Vec<String> = artifact.parts.iter().map(|part| part.id.clone()).collect();
    let Some(part) = artifact.parts.iter_mut().find(|part| part.id == said.part) else {
        return Err(format!("`{}` holds no part `{}`. Parts it holds: {}", artifact.title, said.part, ids.join(", ")));
    };
    if !part.asks {
        return Err(format!("`{}` was not put to him: a part that only informs, or acts when pressed, has nothing to answer", part.id));
    }
    if part.state != PartState::Open {
        return Err(format!("`{}` was {} already", part.id, part.state.word().replace('_', " ")));
    }
    if part.drawn != said.drawn {
        return Err(format!("`{}` was put again since he read it: read the new one", part.id));
    }
    let words = said.words.as_deref().map(str::trim).filter(|words| !words.is_empty()).map(str::to_string);
    if said.how == PartHow::SentBack && words.is_none() {
        return Err("a part is sent back with a note saying what to change".into());
    }
    let takes = matches!(part.content, PartContent::Takes { .. });
    if (said.how == PartHow::Picked) != takes {
        return Err(if takes {
            "a takes part is answered by a vote, never an Approve or a note: send a pick, with how `picked` and the take he chose".into()
        } else {
            "only a takes part is answered with a pick. Words, a drawing and a page are approved or sent back".into()
        });
    }
    if let PartContent::Takes { takes, .. } = &part.content {
        let Some(chosen) = said.take.as_deref() else { return Err("a pick names the take: `take`".into()) };
        if !takes.iter().any(|one| one.id == chosen) {
            return Err(format!("`{}` holds no take `{chosen}`. Takes it holds: {}", part.id, takes.iter().map(|one| one.id.as_str()).collect::<Vec<_>>().join(", ")));
        }
    }
    part.state = match said.how {
        PartHow::Approved => PartState::Approved,
        PartHow::SentBack => PartState::SentBack,
        PartHow::Picked => PartState::Picked,
    };
    part.answer = Some(PartAnswer { words, take: said.take.clone(), notes: BTreeMap::new(), at: Some(at(40)) });
    artifact.state = state_of(&artifact.parts);
    artifact.item = (artifact.state == ArtifactState::Asking).then(|| artifact.id.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(part: &str, drawn: &str, how: PartHow, take: Option<&str>, words: Option<&str>) -> AnswerPart {
        AnswerPart { session_id: ipc::SessionId::carried(JOB), artifact: ID.into(), part: part.into(), drawn: drawn.into(), how, take: take.map(str::to_string), words: words.map(str::to_string), notes: BTreeMap::new() }
    }

    #[test]
    fn the_open_artifact_holds_one_part_of_each_kind_in_the_order_they_were_added() {
        let kept = artifact(false);
        let kinds: Vec<&str> = kept.parts.iter().map(|part| part.content.kind().word()).collect();
        assert_eq!(kinds, ["words", "takes", "drawing", "buttons", "page"]);
        assert!(kept.parts.windows(2).all(|pair| pair[0].added.as_str() < pair[1].added.as_str()));
        assert_eq!(kept.state, ArtifactState::Asking);
        assert_eq!(kept.parts.iter().filter(|part| part.is_open_ask()).count(), 2);
    }

    #[test]
    fn the_answered_artifact_has_no_ask_left_and_reads_approved() {
        let kept = artifact(true);
        assert!(kept.parts.iter().all(|part| !part.is_open_ask()));
        assert_eq!(kept.state, ArtifactState::Approved);
        assert_eq!(kept.item, None);
        assert_eq!(kept.presses.len(), 1, "the button was pressed");
    }

    #[test]
    fn the_artifact_settles_only_once_the_last_ask_is_answered() {
        let mut kept = artifact(false);
        apply(&mut kept, &said("bar", "bar-1", PartHow::Picked, Some("rail"), None)).unwrap();
        assert_eq!(kept.state, ArtifactState::Asking, "the words still ask");
        apply(&mut kept, &said("intro", "intro-1", PartHow::Approved, None, None)).unwrap();
        assert_eq!(kept.state, ArtifactState::Approved);
        assert_eq!(kept.item, None);
    }

    #[test]
    fn a_part_sent_back_settles_the_artifact_sent_back() {
        let mut kept = artifact(false);
        apply(&mut kept, &said("bar", "bar-1", PartHow::Picked, Some("bar"), None)).unwrap();
        apply(&mut kept, &said("intro", "intro-1", PartHow::SentBack, None, Some("Shorter."))).unwrap();
        assert_eq!(kept.state, ArtifactState::SentBack);
    }

    #[test]
    fn fleets_refusals_are_the_sentences_the_mock_gives() {
        let mut kept = artifact(false);
        let refused = |kept: &mut JobArtifact, said: AnswerPart| apply(kept, &said).unwrap_err();
        assert!(refused(&mut kept, said("bar", "bar-1", PartHow::Approved, None, None)).contains("send a pick"), "takes are a vote, never an Approve");
        assert!(refused(&mut kept, said("bar", "bar-1", PartHow::SentBack, None, Some("No."))).contains("send a pick"));
        assert!(refused(&mut kept, said("intro", "intro-1", PartHow::Picked, Some("rail"), None)).contains("only a takes part"));
        assert!(refused(&mut kept, said("intro", "intro-1", PartHow::SentBack, None, None)).contains("with a note"));
        assert!(refused(&mut kept, said("intro", "intro-0", PartHow::Approved, None, None)).contains("put again"));
        assert!(refused(&mut kept, said("try", "try-1", PartHow::Approved, None, None)).contains("not put to him"));
        assert!(refused(&mut kept, said("bar", "bar-1", PartHow::Picked, Some("nope"), None)).contains("holds no take"));
        assert!(refused(&mut kept, said("nope", "x", PartHow::Approved, None, None)).contains("holds no part"));
        assert_eq!(kept, artifact(false), "a refusal writes nothing");
    }

    #[test]
    fn every_take_and_the_drawing_read_as_scenes() {
        let kept = artifact(false);
        for part in &kept.parts {
            let scenes: Vec<&str> = match &part.content {
                PartContent::Takes { takes, .. } => takes.iter().filter_map(|one| one.scene.as_deref()).collect(),
                PartContent::Drawing { scene } => vec![scene.as_str()],
                _ => Vec::new(),
            };
            for text in scenes {
                let raw: ipc::RawScene = serde_json::from_str(text).expect("the scene is JSON");
                assert!(raw.scene().is_some(), "{} reads as a scene", part.id);
            }
        }
    }
}

/// Which artifact of parts the scenario named serves: the open one, or the one with every ask answered.
pub fn scenario() -> Option<bool> {
    if crate::mock::scenario_is("artifact-parts") {
        Some(false)
    } else if crate::mock::scenario_is("artifact-parts-answered") {
        Some(true)
    } else {
        None
    }
}
