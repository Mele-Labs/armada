//! `send_session_note` over a real repository holding the Usage page: the candidates are found by
//! `git grep` in the Session's checkout, the turn carries them with the state and the crop, and the
//! thread keeps only the person's words. The ranking itself is `acceptance::pinned_note`'s.
//! `send_session_notes` sends a walk's notes as one turn, numbered, each with its own lines.

use std::path::Path;
use std::sync::Arc;

use api::{HostedSessions, Refusal};
use ipc::{
    NoteCrop, NoteElement, NotePlace, NoteState, NoteTake, NoteWalk, SendSessionNote, SendSessionNotes, SessionId,
    SessionNote, SessionRow,
};

use super::session_host::{eventually, rig_fitted, Rig};

pub(super) const PATH: &str = "packages/components/src/compositions/UsagePage/UsagePage.tsx";
const USAGE_PAGE: &str = include_str!("../../../acceptance/tests/bench/pinned_note/UsagePage.tsx.txt");
pub(super) const SAID: &str = "this row should say which commands";

fn git(repo: &Path, args: &[&str]) {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(run.status.success(), "git {args:?}: {run:?}");
}

/// The rig's root as a repository holding the page, and a test that says the row's words whole on
/// an `<li>`, which is set aside while the page's own file matches.
pub(super) fn a_repository(rig: &Rig) {
    let root = Path::new(&rig.root);
    git(root, &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    let page = root.join(PATH);
    std::fs::create_dir_all(page.parent().unwrap()).unwrap();
    std::fs::write(&page, USAGE_PAGE).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(
        root.join("tests/usage.tsx"),
        "function ShareRows() {\n  return <li aria-label=\"Other commands\">of the burn</li>;\n}\n",
    )
    .unwrap();
    git(root, &["add", PATH, "tests/usage.tsx"]);
    git(root, &["commit", "--quiet", "-m", "the page"]);
}

pub(super) fn note(id: &SessionId, label: &str, stamp: Option<&str>) -> SendSessionNote {
    SendSessionNote { session_id: id.clone(), note: SessionNote {
        said: format!("  {SAID}  "),
        address: "http://127.0.0.1:1/?walk=burnBySessionModelTool".to_string(),
        note_id: Some("n-1".to_string()),
        place: NotePlace {
            component: Some("ShareRows".to_string()),
            owners: vec!["UsagePage".to_string(), "Boundary".to_string(), "TheShell".to_string()],
            selector: "li".to_string(),
            element: NoteElement { tag: "li".to_string(), text: label.to_string(), label: Some(label.to_string()) },
            nearby: vec!["Burn by Bash command".to_string()],
            stamp: stamp.map(str::to_string),
            location: "/?walk=burnBySessionModelTool".to_string(),
            screen: None,
            layer: None,
        },
        state: NoteState {
            selected: vec![r#"tab "Bash""#.to_string(), r#"button "JobsFilterlGo to Jobsl""#.to_string()],
            walk: Some(NoteWalk { name: "burnBySessionModelTool".to_string(), step: 6, of: 9 }),
            take: Some(NoteTake { letter: "B".to_string(), name: "Needs you first".to_string() }),
            scenario: Some("usage".to_string()),
        },
        crop: Some(NoteCrop { media_type: "image/png".to_string(), data: "aGVsbG8=".to_string() }),
        job: Vec::new(),
    } }
}

pub(super) async fn turn_of(rig: &Rig) -> String {
    eventually(|| async { rig.stand_in.starts().len() == 1 && !rig.stand_in.process(0).sent.lock().unwrap().is_empty() }).await;
    rig.stand_in.process(0).sent.lock().unwrap()[0].clone()
}

#[tokio::test]
async fn a_note_hands_the_session_its_lines_and_the_thread_keeps_only_the_words() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;

    let sent = Arc::clone(&rig.fleet)
        .send_session_note(note(&id, "Other commands, 16.1% of the burn", None))
        .await
        .expect("taken");

    let first = sent.candidates.first().expect("a line matched");
    assert_eq!((first.file.as_str(), first.line), (PATH, 171), "{:#?}", sent.candidates);
    assert!(sent.candidates.iter().all(|one| !one.file.starts_with("tests/")), "a test is set aside: {:#?}", sent.candidates);

    let turn = turn_of(&rig).await;
    for wanted in [
        "this row should say which commands\\n\\nWhere he pointed, to find the code; not the request:\\nPointed at: li \\\"Other commands, 16.1% of the burn\\\" (note n-1)\\nScreenshot: image 1 of this message, the whole window, ring 1 marks what he pointed at",
        &format!("Candidates:\\n1. {PATH}:171 — "),
        r#"State: tab \"Bash\" selected; walk burnBySessionModelTool step 6 of 9; take B, Needs you first; scenario usage"#,
        "Page: http://127.0.0.1:1/?walk=burnBySessionModelTool",
        "Inside: Burn by Bash command; component chain ShareRows ← UsagePage ← Boundary",
        r#""data":"aGVsbG8=""#,
    ] {
        assert!(turn.contains(wanted), "the turn carries {wanted}: {turn}");
    }
    for unwanted in ["JobsFilterl", "TheShell", "Handoff:"] {
        assert!(!turn.contains(unwanted), "a pressed button, a far component and a handoff line are left out, {unwanted}: {turn}");
    }

    let rows = rig.rows(&id).await;
    assert!(
        matches!(&rows[..], [SessionRow::Message { text, .. }] if text == SAID),
        "the thread keeps the words alone: {rows:#?}"
    );
}

#[tokio::test]
async fn a_notes_job_lines_come_after_his_words_as_background_and_nothing_follows_them() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;
    let mut sent = note(&id, "Other commands, 16.1% of the burn", None);
    sent.note.job = vec!["Job: Fix the flaky store test (01OPEN)".to_string(), "State: working".to_string()];

    Arc::clone(&rig.fleet).send_session_note(sent).await.expect("taken");

    let turn = turn_of(&rig).await;
    let at = |wanted: &str| turn.find(wanted).unwrap_or_else(|| panic!("the turn carries {wanted}: {turn}"));
    let order = [
        at("this row should say which commands"),
        at("Background, not the task: the Job he had open when he pinned this\\nJob: Fix the flaky store test (01OPEN)\\nState: working"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "his words, then the Job: {turn}");
    assert!(!turn.contains("Handoff:"), "no handoff line follows the Job: {turn}");
}

#[tokio::test]
async fn a_note_nothing_in_the_code_says_is_sent_whole_without_a_line_for_it() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;

    let sent = Arc::clone(&rig.fleet)
        .send_session_note(note(&id, "Zebra crossings, 4", Some(&format!("{PATH}:205"))))
        .await
        .expect("taken");

    assert!(sent.candidates.is_empty(), "{:#?}", sent.candidates);
    let turn = turn_of(&rig).await;
    assert!(!turn.contains("Candidates:") && !turn.contains("No line on screen matched"), "no match is no line: {turn}");
    assert!(turn.contains(&format!("Stamp: the page's stamp {PATH}:205 was dropped")), "{turn}");
    assert!(turn.contains("Page: http://127.0.0.1:1/"), "the rest is still sent: {turn}");
}

/// A note of a walk: its words, the label it pointed at, its page and its crop's bytes.
fn walk_note(id: &SessionId, said: &str, label: &str, address: &str, crop: &str) -> SessionNote {
    let mut one = note(id, label, None).note;
    one.said = said.to_string();
    one.address = address.to_string();
    one.note_id = Some(format!("n-{crop}"));
    one.crop = Some(NoteCrop { media_type: "image/png".to_string(), data: crop.to_string() });
    one
}

#[tokio::test]
async fn three_notes_reach_the_session_as_one_turn_numbered_with_three_crops_and_no_handoff() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;

    let sent = Arc::clone(&rig.fleet)
        .send_session_notes(SendSessionNotes {
            session_id: id.clone(),
            notes: vec![
                walk_note(&id, "  too wide  ", "Other commands, 16.1% of the burn", "http://127.0.0.1:1/?take=A", "b25l"),
                walk_note(&id, "wrong colour", "Zebra crossings, 4", "http://127.0.0.1:2/?take=B", "dHdv"),
                walk_note(&id, "say which", "Other commands, 16.1% of the burn", "file:///tmp/page.html", "dGhyZWU="),
            ],
        })
        .await
        .expect("taken");

    assert_eq!(sent.candidates.len(), 3, "one list a note: {:#?}", sent.candidates);
    assert_eq!(sent.candidates[0].first().map(|one| one.line), Some(171), "{:#?}", sent.candidates);
    assert!(sent.candidates[1].is_empty(), "{:#?}", sent.candidates[1]);

    let turn = turn_of(&rig).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let turns = rig.stand_in.process(0).sent.lock().unwrap().clone();
    assert_eq!(turns.len(), 1, "one turn: {turns:#?}");
    let at = |wanted: &str| turn.find(wanted).unwrap_or_else(|| panic!("the turn carries {wanted}: {turn}"));
    let order = [
        at("1. too wide\\n   Where he pointed, to find the code; not the request:\\n   Pointed at: li \\\"Other commands, 16.1% of the burn\\\" (note n-b25l)\\n   Screenshot: image 1 of this message, the whole window, ring 1 marks what he pointed at"),
        at("   Page: http://127.0.0.1:1/?take=A"),
        at("2. wrong colour\\n   Where he pointed, to find the code; not the request:\\n   Pointed at: li \\\"Zebra crossings, 4\\\" (note n-dHdv)\\n   Screenshot: image 2 of this message, the whole window, ring 2 marks what he pointed at"),
        at("   Page: http://127.0.0.1:2/?take=B"),
        at("3. say which\\n"),
        at("   Screenshot: image 3 of this message, the whole window, ring 3 marks what he pointed at"),
        at("   Page: file:///tmp/page.html"),
        at(r#""data":"b25l""#),
        at(r#""data":"dHdv""#),
        at(r#""data":"dGhyZWU=""#),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "in note order: {turn}");
    assert!(!turn.contains("Handoff:"), "no handoff line: {turn}");
    assert_eq!(turn.matches(r#""type":"image""#).count(), 3, "three pictures: {turn}");

    let rows = rig.rows(&id).await;
    assert!(
        matches!(&rows[..], [SessionRow::Message { text, .. }] if text == "1. too wide\n2. wrong colour\n3. say which"),
        "the thread keeps each note's words, numbered: {rows:#?}"
    );
}

#[tokio::test]
async fn a_note_without_a_crop_has_no_screenshot_line_and_the_next_picture_skips_it() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;
    let mut bare = walk_note(&id, "no picture", "Zebra crossings, 4", "http://127.0.0.1:2/?take=B", "dHdv");
    bare.crop = None;

    Arc::clone(&rig.fleet)
        .send_session_notes(SendSessionNotes {
            session_id: id.clone(),
            notes: vec![
                walk_note(&id, "first", "Other commands, 16.1% of the burn", "http://127.0.0.1:1/?take=A", "b25l"),
                bare,
                walk_note(&id, "third", "Other commands, 16.1% of the burn", "file:///tmp/page.html", "dGhyZWU="),
            ],
        })
        .await
        .expect("taken");

    let turn = turn_of(&rig).await;
    assert!(turn.contains("ring 1 marks"), "{turn}");
    assert!(turn.contains("image 2 of this message, the whole window, ring 3 marks"), "{turn}");
    assert_eq!(turn.matches("Screenshot:").count(), 2, "none for the note without a crop: {turn}");
    assert_eq!(turn.matches(r#""type":"image""#).count(), 2, "two pictures: {turn}");
}

#[tokio::test]
async fn no_notes_are_refused_and_send_nothing() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;

    let refused = Arc::clone(&rig.fleet)
        .send_session_notes(SendSessionNotes { session_id: id.clone(), notes: Vec::new() })
        .await;

    let Err(Refusal::Unacceptable(why)) = refused else { panic!("an empty batch is refused: {refused:?}") };
    assert_eq!(why.code, "fleet.session_notes_empty");
    assert!(rig.stand_in.starts().is_empty(), "no turn is taken");
    assert!(rig.rows(&id).await.is_empty(), "the thread is untouched");
}
