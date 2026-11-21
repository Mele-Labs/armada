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
    ArtifactPart { id: id.into(), content, asks, state: PartState::Open, revision: 1, drawn: format!("{id}-1"), added: at(minute), answer: None, question: None }
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
                takes: vec![take("bar", "A \u{b7} Bar under the title", bar()), take("rail", "B \u{b7} Rail (Job's pick)", rail()), take("dots", "C \u{b7} Dots, set out in a row of three, a title long enough to run past a whole line of the strip and be cut after the second", dots())],
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

/// The id and title of the Asides artifact: words for the owner, and two takes that are each a mock of several screens.
pub const ASIDES_ID: &str = "sketch:Asides";
pub const ASIDES_TITLE: &str = "Asides";

/// One screen of a take, as the HTML a `mock` node holds: a title bar, a list of rows and one action.
fn screen(id: &str, x: f64, y: f64, title: &str, rows: &[&str], action: &str) -> String {
    let rows: String = rows.iter().map(|row| format!("<li>{row}</li>")).collect();
    let html = format!("<div class=\\\"bar\\\">{title}</div><ul>{rows}</ul><div class=\\\"go\\\">{action}</div>");
    let css = "body{margin:0;font:14px sans-serif;background:#f4f1ea;color:#1c1b19}.bar{padding:14px 16px;font-weight:600;background:#e6e0d2}ul{list-style:none;margin:0;padding:8px 16px}li{padding:12px 0;border-bottom:1px solid #d8d1c0}.go{margin:16px;padding:10px;border-radius:8px;background:#2f6f5e;color:#fff;text-align:center}";
    format!(r#"{{"id":"{id}","kind":"mock","x":{x},"y":{y},"w":300,"title":"{title}","html":"{html}","css":"{css}"}}"#)
}

/// Take A: the thread, an aside opened from a message, and the aside kept beside it.
fn asides_beside() -> String {
    let nodes = [
        screen("a1", 0., 0., "Thread", &["The rail reads better", "Where does the aside go?", "Keep it short"], "Open an aside"),
        screen("a2", 380., 0., "Aside", &["Opened from the second message", "Its own short thread"], "Send"),
        screen("a3", 760., 0., "Kept", &["Aside on the second message", "Two replies"], "Back to the thread"),
    ];
    format!(r#"{{"nodes":[{}],"edges":[{{"id":"e1","from":"a1","to":"a2"}},{{"id":"e2","from":"a2","to":"a3"}}]}}"#, nodes.join(","))
}

/// Take B: the aside as a sheet over the thread, then folded to a line under the message.
fn asides_sheet() -> String {
    let nodes = [
        screen("b1", 0., 0., "Thread", &["The rail reads better", "Where does the aside go?", "Keep it short"], "Open an aside"),
        screen("b2", 380., 0., "Aside sheet", &["Opened over the thread", "Esc puts it away"], "Send"),
        screen("b3", 760., 0., "Folded", &["Where does the aside go?", "Aside: two replies"], "Open the aside"),
    ];
    format!(r#"{{"nodes":[{}],"edges":[{{"id":"e1","from":"b1","to":"b2"}},{{"id":"e2","from":"b2","to":"b3"}}]}}"#, nodes.join(","))
}

/// The Asides artifact: words that ask, and a vote between two takes.
pub fn asides() -> JobArtifact {
    let parts = vec![
        part("intro", PartContent::Words { text: "An aside is a short thread opened from one message. Two ways to draw it: beside the thread, or as a sheet over it.".into() }, true, 0),
        part(
            "way",
            PartContent::Takes {
                question: Some("Where does an aside open?".into()),
                form: VoteForm::Pick,
                takes: vec![take("beside", "A \u{b7} Beside the thread", asides_beside()), take("sheet", "B \u{b7} Sheet over the thread (Job's pick)", asides_sheet())],
                parts: Vec::new(),
            },
            true,
            1,
        ),
    ];
    let kept = |number| ArtifactRevision { number, at: at(number), by: ArtifactBy::Agent, form: ArtifactForm::Draft };
    JobArtifact {
        id: ASIDES_ID.into(),
        title: ASIDES_TITLE.into(),
        form: ArtifactForm::Draft,
        revisions: vec![kept(1)],
        state: ArtifactState::Asking,
        words: None,
        item: Some(ASIDES_ID.into()),
        in_plan: None,
        archived: None,
        for_line: Some(ArtifactFor { text: Some("picks where an aside opens, and the PR text that ships it.".into()), links: Vec::new() }),
        presses: Vec::new(),
        votes: Vec::new(),
        seen: None,
        parts,
    }
}

/// The Job and id of the `review-asks` takes: where the Features screen opens, three wireframes with the Job's pick second.
pub const FEATURES_JOB: &str = "mock-01";
pub const FEATURES_ID: &str = "sketch:Features place";
/// The Operator's read of the Features place vote, as its page would keep one: the reading and the take it would pick.
pub const FEATURES_OPERATOR_READ: (&str, &str) = ("Jobs and Usage open from the top bar, and Features is a screen of the same rank.", "In the top bar");

/// One box of a wireframe, grey unless it is the place the take puts Features.
fn wire_box(id: &str, x: u32, y: u32, w: u32, h: u32, title: &str, color: &str) -> String {
    format!(r#"{{"id":"{id}","kind":"box","x":{x},"y":{y},"w":{w},"h":{h},"title":"{title}","color":"{color}"}}"#)
}

fn wireframe(boxes: &[String]) -> String {
    format!(r#"{{"nodes":[{}],"edges":[]}}"#, boxes.join(","))
}

fn features_top_bar() -> String {
    wireframe(&[
        wire_box("jobs", 0, 0, 110, 40, "Jobs", "grey"),
        wire_box("features", 130, 0, 130, 40, "Features", "amber"),
        wire_box("usage", 280, 0, 110, 40, "Usage", "grey"),
        wire_box("list", 0, 80, 170, 200, "Feature list", "teal"),
        wire_box("page", 210, 80, 280, 200, "Feature page", "blue"),
    ])
}

fn features_sidebar() -> String {
    wireframe(&[
        wire_box("jobs", 0, 0, 130, 40, "Jobs", "grey"),
        wire_box("features", 0, 60, 130, 40, "Features", "amber"),
        wire_box("usage", 0, 120, 130, 40, "Usage", "grey"),
        wire_box("list", 170, 0, 140, 280, "Feature list", "teal"),
        wire_box("page", 350, 0, 160, 280, "Feature page", "blue"),
    ])
}

fn features_tab() -> String {
    wireframe(&[
        wire_box("jobs", 0, 0, 140, 280, "Job list", "grey"),
        wire_box("thread", 180, 0, 100, 40, "Thread", "grey"),
        wire_box("features", 300, 0, 120, 40, "Features", "amber"),
        wire_box("page", 180, 80, 320, 200, "This Job's features", "blue"),
    ])
}

/// The takes the `review-asks` takes ask stands for: one part, a pick of three places, with no words of its own.
pub fn features_place() -> JobArtifact {
    let takes = vec![
        take("sidebar", "In the sidebar", features_sidebar()),
        take("top-bar", "In the top bar (Job's pick)", features_top_bar()),
        take("tab", "As a tab in each Job", features_tab()),
    ];
    let parts = vec![part("place", PartContent::Takes { question: Some("Where should Features open?".into()), form: VoteForm::Pick, takes, parts: Vec::new() }, true, 0)];
    let kept = |number| ArtifactRevision { number, at: at(number), by: ArtifactBy::Agent, form: ArtifactForm::Draft };
    JobArtifact {
        id: FEATURES_ID.into(),
        title: "Features place".into(),
        form: ArtifactForm::Draft,
        revisions: vec![kept(1)],
        state: ArtifactState::Asking,
        words: None,
        item: Some("agent-place".into()),
        in_plan: None,
        archived: None,
        for_line: Some(ArtifactFor { text: Some("Features is a screen of its own, so it opens from where Jobs and Usage open.".into()), links: Vec::new() }),
        presses: Vec::new(),
        votes: Vec::new(),
        seen: None,
        parts,
    }
}

/// The Job whose artifacts of parts the mock serves: the review sheet's Job on `review-asks`, else the tool rows' Job.
pub fn holder() -> &'static str {
    if crate::mock::scenario_is("review-asks") {
        FEATURES_JOB
    } else {
        JOB
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

/// Whether the scenario is the folds': six artifacts kept among the thread's items, one answered, one sent back and redrawn,
/// one with no scene, one with a drawing, and a Job running a call.
pub fn folds() -> bool {
    crate::mock::scenario_is("artifact-folds")
}

/// The artifacts the scenario's Job holds when it opens.
pub fn initial() -> Option<Vec<JobArtifact>> {
    if crate::mock::scenario_is("review-asks") {
        return Some(vec![features_place()]);
    }
    if crate::mock::scenario_is("artifact-asides") {
        return Some(vec![asides()]);
    }
    match scenario() {
        Some(answered) => Some(vec![artifact(answered)]),
        None if folds() => Some(folded()),
        None => None,
    }
}

/// What he said of an older revision before this run, as the mock Fleet cannot say it: artifact, revision and his note on the send back.
pub fn earlier_verdicts() -> Vec<(&'static str, u32, &'static str)> {
    if folds() {
        vec![("draft:Merge line wording", 1, "Say which branch the line names, and keep it to one line.")]
    } else {
        Vec::new()
    }
}

/// One artifact that holds a single part, as the folds' scenario keeps them.
fn single(id: &str, title: &str, form: ArtifactForm, kept: &[u32], part: ArtifactPart, state: ArtifactState) -> JobArtifact {
    let revisions = kept.iter().enumerate().map(|(at, minute)| ArtifactRevision { number: at as u32 + 1, at: self::at(*minute), by: ArtifactBy::Agent, form }).collect();
    JobArtifact {
        id: id.into(),
        title: title.into(),
        form,
        revisions,
        state,
        words: None,
        item: (state == ArtifactState::Asking).then(|| id.into()),
        in_plan: None,
        archived: None,
        for_line: None,
        presses: Vec::new(),
        votes: Vec::new(),
        seen: None,
        parts: vec![part],
    }
}

fn words_part(id: &str, text: &str, revision: u32, minute: u32) -> ArtifactPart {
    ArtifactPart { revision, drawn: format!("{id}-{revision}"), ..part(id, PartContent::Words { text: text.into() }, true, minute) }
}

/// The folds' artifacts, oldest kept first, each at the minute its revision was kept: the thread's items stand between them.
fn folded() -> Vec<JobArtifact> {
    let mut names = single("draft:Branch names", "Branch names", ArtifactForm::Draft, &[5], words_part("names", "Name each branch for the Job that opened it: native/ and the topic.", 1, 5), ArtifactState::Approved);
    names.parts[0].state = PartState::Approved;
    names.parts[0].answer = Some(PartAnswer { words: Some("Good.".into()), at: Some(at(6)), ..PartAnswer::default() });
    let commit = single("draft:Commit words", "Commit words", ArtifactForm::Draft, &[12], words_part("commit", "A commit message says what the diff cannot: why the line moved, and what it fixes.", 1, 12), ArtifactState::Asking);
    let wording = single("draft:Merge line wording", "Merge line wording", ArtifactForm::Draft, &[15, 25], words_part("wording", "The merge line reads: Behind main by three commits.", 2, 25), ArtifactState::Asking);
    let title = single("draft:Page title", "Page title", ArtifactForm::Draft, &[36], words_part("title", "Name the page for what it answers: Which pages got a vote.", 1, 36), ArtifactState::Asking);
    let order = ArtifactPart {
        ..part(
            "order",
            PartContent::Takes {
                question: Some("Which goes out first?".into()),
                form: VoteForm::Pick,
                takes: ["A \u{b7} Dark mode first", "B \u{b7} Votes first (Job's pick)", "C \u{b7} Both together"]
                    .iter()
                    .enumerate()
                    .map(|(at, title)| PartTake { id: format!("t{at}"), title: (*title).into(), scene: None })
                    .collect(),
                parts: Vec::new(),
            },
            true,
            44,
        )
    };
    let rollout = single("sketch:Rollout order", "Rollout order", ArtifactForm::Sketch, &[44], order, ArtifactState::Asking);
    let flow = single("sketch:Step flow", "Step flow", ArtifactForm::Sketch, &[46], part("flow", PartContent::Drawing { scene: flow() }, true, 46), ArtifactState::Asking);
    vec![names, commit, wording, title, rollout, flow]
}

/// The Job sent a part back and drew it again: the part asks on its next revision, which the artifact keeps at `now`.
pub fn redraw(artifact: &mut JobArtifact, now: Instant) {
    let number = artifact.revisions.last().map_or(1, |last| last.number) + 1;
    for part in artifact.parts.iter_mut().filter(|part| part.state == PartState::SentBack) {
        part.revision += 1;
        part.drawn = format!("{}-{}", part.id, part.revision);
        part.state = PartState::Open;
        part.answer = None;
        if let PartContent::Words { text } = &mut part.content {
            *text = format!("Second pass. {text}");
        }
    }
    artifact.revisions.push(ArtifactRevision { number, at: now, by: ArtifactBy::Agent, form: artifact.form });
    artifact.state = ArtifactState::Asking;
    artifact.words = None;
    artifact.item = Some(artifact.id.clone());
}

/// The thread the folds' Job holds: what it said and did between the cards, at the minutes the cards were kept.
pub fn extend_thread(thread: &mut ipc::JobThread) {
    let rows = [
        r#"{"kind":"row","row":{"kind":"message","id":"f-1","at":"2026-11-14T10:01:00Z","from":{"kind":"agent"},"text":"Six questions came out of the page votes. I put each to you as its own card."}}"#,
        r#"{"kind":"calls","id":"f-2","names":["Read"],"rows":[{"kind":"tool","id":"f-2","at":"2026-11-14T10:08:00Z","text":"Read packages/components/src/PageVotes/page_votes.rs"}]}"#,
        r#"{"kind":"row","row":{"kind":"message","id":"f-3","at":"2026-11-14T10:20:00Z","from":{"kind":"agent"},"text":"The merge line wording needed one more pass, so it is back."}}"#,
        r#"{"kind":"calls","id":"f-4","names":["Edit"],"rows":[{"kind":"tool","id":"f-4","at":"2026-11-14T10:30:00Z","text":"Edit packages/components/src/MergeLine/MergeLine.tsx"}]}"#,
        r#"{"kind":"row","row":{"kind":"message","id":"f-5","at":"2026-11-14T10:41:00Z","from":{"kind":"agent"},"text":"Two more cards: the rollout order, and the step flow as a drawing."}}"#,
        r#"{"kind":"calls","id":"f-6","names":["Read"],"rows":[{"kind":"tool","id":"f-6","at":"2026-11-14T10:50:00Z","text":"Read crates/fleet/src/page_votes.rs"}]}"#,
    ];
    for row in rows {
        thread.items.push(ipc::decode::<ipc::ThreadItem>("a thread item", row.as_bytes()).expect("the mock's rows decode"));
    }
}
