//! What rides with a message to Helm, and what its box runs on: files and tags kept and read
//! back, the turn the session is told, the model and effort a person chose, and the commands `/`
//! lists. **The host is a recorder**, so what is asserted is the `Carry` Fleet hands it; the real
//! process host's reading of one is `helm::hosting`'s own.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::{Conversations, HelmSeen, HelmWatch, Refusal};
use base64::Engine as _;
use ipc::{
    AskHelm, HelmBoxFacts, HelmMessage, SessionTag, SessionUpload, TagKind, TuneHelm,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::helm::{Carried, Carry, Carrying, ConversationKey, Heard, Hosting};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// A host that records what it was asked to carry, answers at once, and says the commands it was
/// given as Helm's own do on an `init` line.
#[derive(Default)]
struct Recorder {
    carried: Mutex<Vec<Carry>>,
    commands: Mutex<Option<Vec<String>>>,
}

impl Hosting for Recorder {
    fn carry<'a>(&'a self, carry: Carry, heard: &'a dyn Heard) -> Carrying<'a> {
        self.carried.lock().expect("not poisoned").push(carry);
        if let Some(names) = self.commands.lock().expect("not poisoned").clone() {
            heard.commands(names);
        }
        Box::pin(async {
            Carried::Answered {
                session: String::from("a-session"),
            }
        }) as Pin<Box<dyn Future<Output = Carried> + Send>>
    }

    fn model(&self) -> String {
        String::from("a-model")
    }

    fn running(&self) -> Vec<u32> {
        Vec::new()
    }
}

/// A Fleet over `home` whose Helm is `host`. Called again on the same `home` is a restart.
fn hosted(home: &TempDir, host: &Arc<Recorder>) -> Arc<Hosted> {
    let host: Arc<dyn Hosting> = host.clone();
    Arc::new(
        Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[]))).hosting_helm_on(host),
    )
}

fn words(text: &str) -> AskHelm {
    AskHelm {
        text: text.to_string(),
        attachments: Vec::new(),
        mentions: Vec::new(),
        context: None,
    }
}

fn upload(name: &str, media_type: &str, bytes: &[u8]) -> SessionUpload {
    SessionUpload {
        name: name.to_string(),
        media_type: media_type.to_string(),
        data: base64::engine::general_purpose::STANDARD.encode(bytes),
    }
}

fn issue(number: u32, title: &str) -> SessionTag {
    SessionTag {
        kind: TagKind::Issue,
        id: number.to_string(),
        title: format!("#{number} {title}"),
        job: None,
        url: Some(format!("https://forge.invalid/o/r/issues/{number}")),
    }
}

/// Until the reply is in and its session kept.
async fn settled(fleet: &Hosted) {
    for _ in 0..500 {
        if !fleet.observe_helm(None).await.expect("observed").replying {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the reply never settled");
}

async fn ask(fleet: &Arc<Hosted>, asked: AskHelm) {
    Arc::clone(fleet)
        .ask_helm(asked, None)
        .await
        .expect("the message is taken");
    settled(fleet).await;
}

fn thread_file(fleet: &Hosted) -> PathBuf {
    let served = fleet.repositories().first().expect("a served repository");
    ConversationKey::of_repository(served.manifest().id()).thread_in(served.records_root())
}

async fn offered(live: &mut HelmWatch) -> Vec<HelmMessage> {
    let mut seen = Vec::new();
    while let Ok(Some(HelmSeen::Message(message))) =
        tokio::time::timeout(Duration::from_millis(50), live.next()).await
    {
        seen.push(message);
    }
    seen
}

#[tokio::test]
async fn files_and_tags_are_kept_on_the_row_read_back_and_told_to_the_session() {
    let home = TempDir::new();
    let host = Arc::new(Recorder::default());
    let fleet = hosted(&home, &host);
    let mut asked = words("what is this about?");
    asked.attachments = vec![
        upload("shot.png", "image/png", b"PNGBYTES"),
        upload("notes.txt", "text/plain", b"some notes"),
    ];
    asked.mentions = vec![issue(214, "The title")];
    ask(&fleet, asked).await;

    let history = fleet.observe_helm(None).await.expect("observed").history;
    let HelmMessage::Asked(row) = &history[0] else {
        panic!("the thread opens on what was asked: {history:?}");
    };
    assert_eq!(row.text, "what is this about?");
    assert_eq!(row.tags[0].id, "214");
    let files: Vec<_> = row.files.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(files, ["shot.png", "notes.txt"]);

    let picture = fleet
        .get_helm_file(None, row.files[0].id.clone())
        .await
        .expect("the picture is served");
    assert_eq!(picture.media_type, "image/png");
    assert_eq!(picture.bytes, b"PNGBYTES");
    let notes = fleet
        .get_helm_file(None, row.files[1].id.clone())
        .await
        .expect("the file is served");
    assert_eq!(notes.bytes, b"some notes");

    let carried = host.carried.lock().expect("not poisoned")[0].clone();
    assert!(
        carried
            .turn
            .contains(r#"The person named: issue #214 "The title" (https://forge.invalid/o/r/issues/214)."#),
        "{}",
        carried.turn
    );
    assert!(carried.turn.contains("what is this about?"), "{}", carried.turn);
    assert_eq!(carried.turn.matches("Attached file: ").count(), 1, "{}", carried.turn);
    assert!(carried.turn.contains("notes.txt"), "{}", carried.turn);
    assert_eq!(carried.pictures.len(), 1);
    assert_eq!(carried.pictures[0].0, "image/png");
    let uploads = carried.readable.expect("the uploads are readable");
    assert!(uploads.contains("helm"), "{uploads}");
}

#[tokio::test]
async fn a_file_that_was_never_sent_or_names_a_path_is_refused_as_a_sessions_is() {
    let home = TempDir::new();
    let host = Arc::new(Recorder::default());
    let fleet = hosted(&home, &host);
    let mut asked = words("x");
    asked.attachments = vec![upload("a.txt", "text/plain", b"a")];
    ask(&fleet, asked).await;
    for file in ["f-nothing", "../../etc/passwd", "", "."] {
        match fleet.get_helm_file(None, file.to_string()).await {
            Err(Refusal::Unacceptable(error)) => {
                assert_eq!(error.code, "fleet.session_attachment_refused", "{file}")
            }
            other => panic!("`{file}` is refused, not {:?}", other.map(|got| got.bytes)),
        }
    }
}

#[tokio::test]
async fn a_words_only_message_is_told_as_it_always_was() {
    let home = TempDir::new();
    let host = Arc::new(Recorder::default());
    let fleet = hosted(&home, &host);
    ask(&fleet, words("grackle")).await;
    ask(&fleet, words("thistle")).await;

    let carried = host.carried.lock().expect("not poisoned").clone();
    assert!(carried[0].resuming.is_none());
    assert!(
        carried[0].turn.ends_with("\n\ngrackle"),
        "the brief, then the words: {}",
        carried[0].turn
    );
    assert_eq!(carried[1].turn, "thistle", "a resumed turn is the words alone");
    assert_eq!(carried[1].resuming.as_deref(), Some("a-session"));
    assert!(carried[1].pictures.is_empty());
    assert_eq!((carried[1].model.clone(), carried[1].effort.clone()), (None, None));
    assert_eq!(carried[1].readable, None, "no uploads, no directory to read");

    let history = fleet.observe_helm(None).await.expect("observed").history;
    let HelmMessage::Asked(row) = &history[0] else {
        panic!("{history:?}");
    };
    let json = ipc::encode(&history[0]).expect("encodes");
    assert!(!json.contains("files") && !json.contains("tags"), "{json}");
    assert_eq!(row.text, "grackle");
}

#[tokio::test]
async fn a_message_with_nothing_in_it_is_refused_and_one_with_only_a_tag_or_a_file_is_taken() {
    let home = TempDir::new();
    let host = Arc::new(Recorder::default());
    let fleet = hosted(&home, &host);
    match Arc::clone(&fleet).ask_helm(words("  \n "), None).await {
        Err(Refusal::Unacceptable(error)) => assert_eq!(error.code, "fleet.helm_message_empty"),
        other => panic!("refused, not {other:?}"),
    }
    assert!(host.carried.lock().expect("not poisoned").is_empty());
    assert!(fleet.observe_helm(None).await.expect("observed").history.is_empty());

    let mut tagged = words("");
    tagged.mentions = vec![issue(9, "Nine")];
    ask(&fleet, tagged).await;
    let mut filed = words("");
    filed.attachments = vec![upload("a.txt", "text/plain", b"a")];
    ask(&fleet, filed).await;
    assert_eq!(host.carried.lock().expect("not poisoned").len(), 2);
}

#[tokio::test]
async fn an_attachment_that_is_not_base64_is_refused_with_the_sessions_code() {
    let home = TempDir::new();
    let host = Arc::new(Recorder::default());
    let fleet = hosted(&home, &host);
    let mut bad = words("x");
    bad.attachments = vec![SessionUpload {
        name: "a.png".to_string(),
        media_type: "image/png".to_string(),
        data: "not base64!".to_string(),
    }];
    match Arc::clone(&fleet).ask_helm(bad, None).await {
        Err(Refusal::Unacceptable(error)) => {
            assert_eq!(error.code, "fleet.session_attachment_refused")
        }
        other => panic!("refused, not {other:?}"),
    }
}

#[tokio::test]
async fn a_tuning_is_kept_passed_at_launch_survives_a_restart_and_start_fresh() {
    let home = TempDir::new();
    let host = Arc::new(Recorder::default());
    let fleet = hosted(&home, &host);

    let facts = fleet
        .tune_helm(
            TuneHelm {
                model: Some("a-chosen-model".to_string()),
                effort: Some("high".to_string()),
            },
            None,
        )
        .await
        .expect("tuned");
    assert_eq!(facts.model.as_deref(), Some("a-chosen-model"));
    assert_eq!(facts.effort.as_deref(), Some("high"));

    ask(&fleet, words("hello")).await;
    let carried = host.carried.lock().expect("not poisoned")[0].clone();
    assert_eq!(carried.model.as_deref(), Some("a-chosen-model"));
    assert_eq!(carried.effort.as_deref(), Some("high"));

    // A restart: a new Fleet over the same files reads it from the store.
    drop(fleet);
    let again = hosted(&home, &host);
    let opened = again.observe_helm(None).await.expect("observed");
    assert_eq!(opened.facts.model.as_deref(), Some("a-chosen-model"));
    assert_eq!(opened.facts.effort.as_deref(), Some("high"));

    again.start_helm_fresh(None).await.expect("started fresh");
    let after = again.observe_helm(None).await.expect("observed").facts;
    assert_eq!(after.effort.as_deref(), Some("high"), "starting fresh keeps the choice");
    ask(&again, words("again")).await;
    let carried = host.carried.lock().expect("not poisoned").last().cloned().expect("carried");
    assert_eq!(carried.model.as_deref(), Some("a-chosen-model"));
}

#[tokio::test]
async fn blank_means_the_machines_own_and_an_unknown_effort_is_refused() {
    let home = TempDir::new();
    let host = Arc::new(Recorder::default());
    let fleet = hosted(&home, &host);
    fleet
        .tune_helm(
            TuneHelm {
                model: Some("m".to_string()),
                effort: Some("low".to_string()),
            },
            None,
        )
        .await
        .expect("tuned");

    match fleet
        .tune_helm(
            TuneHelm {
                model: Some("other".to_string()),
                effort: Some("extreme".to_string()),
            },
            None,
        )
        .await
    {
        Err(Refusal::Unacceptable(error)) => assert_eq!(error.code, "fleet.unacceptable_effort"),
        other => panic!("refused, not {other:?}"),
    }
    let kept = fleet.observe_helm(None).await.expect("observed").facts;
    assert_eq!(kept.model.as_deref(), Some("m"), "a refused tune changes nothing");

    let cleared = fleet
        .tune_helm(
            TuneHelm {
                model: Some("  ".to_string()),
                effort: Some(String::new()),
            },
            None,
        )
        .await
        .expect("tuned");
    assert_eq!(cleared, HelmBoxFacts::default());
    ask(&fleet, words("hi")).await;
    let carried = host.carried.lock().expect("not poisoned")[0].clone();
    assert_eq!((carried.model, carried.effort), (None, None));
}

#[tokio::test]
async fn facts_reach_a_watcher_and_never_the_backfill_or_the_file() {
    let home = TempDir::new();
    let host = Arc::new(Recorder::default());
    let fleet = hosted(&home, &host);
    ask(&fleet, words("hello")).await;
    let mut live = fleet.observe_helm(None).await.expect("observed").live;

    fleet
        .tune_helm(
            TuneHelm {
                model: Some("m".to_string()),
                effort: None,
            },
            None,
        )
        .await
        .expect("tuned");
    let seen = offered(&mut live).await;
    assert!(
        seen.iter().any(|message| matches!(
            message,
            HelmMessage::Facts(facts) if facts.model.as_deref() == Some("m")
        )),
        "the watcher is told: {seen:?}"
    );

    let backfill = fleet.observe_helm(None).await.expect("observed").history;
    assert!(
        backfill.iter().all(|message| !matches!(message, HelmMessage::Facts(_))),
        "{backfill:?}"
    );
    let file = std::fs::read_to_string(thread_file(&fleet)).expect("the thread is written");
    assert!(!file.contains(r#""message":"facts""#), "{file}");
}

#[tokio::test]
async fn the_commands_helm_heard_are_served_and_offered_and_before_that_the_sessions_are() {
    let home = TempDir::new();
    let host = Arc::new(Recorder::default());
    let fleet = hosted(&home, &host);

    assert!(fleet.observe_helm(None).await.expect("observed").facts.commands.is_empty());
    fleet.hosts().heard_commands(vec!["/from-a-session".to_string()]);
    let before = fleet.observe_helm(None).await.expect("observed").facts;
    assert_eq!(before.commands, ["/from-a-session"], "until Helm has spoken");

    *host.commands.lock().expect("not poisoned") = Some(vec!["/help".to_string(), "/model".to_string()]);
    let mut live = fleet.observe_helm(None).await.expect("observed").live;
    ask(&fleet, words("hello")).await;
    let seen = offered(&mut live).await;
    assert!(
        seen.iter().any(|message| matches!(
            message,
            HelmMessage::Facts(facts) if facts.commands == ["/help", "/model"]
        )),
        "{seen:?}"
    );
    let after = fleet.observe_helm(None).await.expect("observed").facts;
    assert_eq!(after.commands, ["/help", "/model"], "Helm's own now");

    // The same list again is no change, and says nothing.
    ask(&fleet, words("again")).await;
    let seen = offered(&mut live).await;
    assert!(
        seen.iter().all(|message| !matches!(message, HelmMessage::Facts(_))),
        "{seen:?}"
    );
}
