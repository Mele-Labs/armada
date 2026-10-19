//! `send_session_note` over a real repository holding the Usage page: the candidates are found by
//! `git grep` in the Session's checkout, the turn carries them with the state and the crop, and the
//! thread keeps only the person's words. The ranking itself is `acceptance::pinned_note`'s.

use std::path::Path;
use std::sync::Arc;

use api::HostedSessions;
use ipc::{
    NoteCrop, NoteElement, NotePlace, NoteState, NoteTake, NoteWalk, SendSessionNote, SessionId, SessionRow,
};

use super::session_host::{eventually, rig_fitted, Rig};

const PATH: &str = "packages/components/src/compositions/UsagePage/UsagePage.tsx";
const USAGE_PAGE: &str = include_str!("../../../acceptance/tests/bench/pinned_note/UsagePage.tsx.txt");
const SAID: &str = "this row should say which commands";

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
fn a_repository(rig: &Rig) {
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

fn note(id: &SessionId, label: &str, stamp: Option<&str>) -> SendSessionNote {
    SendSessionNote {
        session_id: id.clone(),
        said: format!("  {SAID}  "),
        address: "http://127.0.0.1:1/?walk=burnBySessionModelTool".to_string(),
        note_id: Some("n-1".to_string()),
        place: NotePlace {
            component: Some("ShareRows".to_string()),
            owners: vec!["UsagePage".to_string(), "Boundary".to_string()],
            selector: "li".to_string(),
            element: NoteElement { tag: "li".to_string(), text: label.to_string(), label: Some(label.to_string()) },
            nearby: vec!["Burn by Bash command".to_string()],
            stamp: stamp.map(str::to_string),
            location: "/?walk=burnBySessionModelTool".to_string(),
            screen: None,
            layer: None,
        },
        state: NoteState {
            selected: vec![r#"tab "Bash""#.to_string()],
            walk: Some(NoteWalk { name: "burnBySessionModelTool".to_string(), step: 6, of: 9 }),
            take: Some(NoteTake { letter: "B".to_string(), name: "Needs you first".to_string() }),
            scenario: Some("usage".to_string()),
        },
        crop: Some(NoteCrop { media_type: "image/png".to_string(), data: "aGVsbG8=".to_string() }),
    }
}

async fn turn_of(rig: &Rig) -> String {
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
        "this row should say which commands\\nPointed at: li \\\"Other commands, 16.1% of the burn\\\" (note n-1)",
        &format!("Candidates:\\n1. {PATH}:171 — "),
        r#"State: tab \"Bash\" selected; walk burnBySessionModelTool step 6 of 9; take B, Needs you first; scenario usage"#,
        "Page: http://127.0.0.1:1/?walk=burnBySessionModelTool",
        "Inside: Burn by Bash command; component chain ShareRows ← UsagePage ← Boundary",
        "Handoff: deliver the way the owner chose when Fleet asked how.",
        r#""data":"aGVsbG8=""#,
    ] {
        assert!(turn.contains(wanted), "the turn carries {wanted}: {turn}");
    }

    let rows = rig.rows(&id).await;
    assert!(
        matches!(&rows[..], [SessionRow::Message { text, .. }] if text == SAID),
        "the thread keeps the words alone: {rows:#?}"
    );
}

#[tokio::test]
async fn a_note_nothing_in_the_code_says_is_sent_whole_and_says_so() {
    let rig = rig_fitted(|_| {});
    a_repository(&rig);
    let id = rig.start().await;

    let sent = Arc::clone(&rig.fleet)
        .send_session_note(note(&id, "Zebra crossings, 4", Some(&format!("{PATH}:205"))))
        .await
        .expect("taken");

    assert!(sent.candidates.is_empty(), "{:#?}", sent.candidates);
    let turn = turn_of(&rig).await;
    assert!(turn.contains(&format!("No line on screen matched the code in {}", rig.root)), "{turn}");
    assert!(turn.contains(&format!("Stamp: the page's stamp {PATH}:205 was dropped")), "{turn}");
    assert!(turn.contains("Page: http://127.0.0.1:1/"), "the rest is still sent: {turn}");
}
