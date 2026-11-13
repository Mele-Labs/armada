//! A preference a person saves survives a restart, and a name outside the
//! closed set is refused by name on the wire code `fleet.unknown_preference`.

use api::{Refusal, Settings};
use ipc::{AnswerWith, LookQuestions, LookResults, Preferences, ReadResults, SavePreference, TakeIn};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

fn saving(name: &str, value: bool) -> SavePreference {
    SavePreference {
        name: name.to_string(),
        value,
        text: None,
    }
}

#[tokio::test]
async fn nothing_saved_reads_as_the_shipped_default() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(
        fleet.get_preferences().await.expect("reads"),
        Preferences::default()
    );
}

#[tokio::test]
async fn a_saved_preference_survives_a_restart() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let now = fleet
        .save_preferences(saving("where_things_are_open", true))
        .await
        .expect("saved");
    assert!(now.where_things_are_open);
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(
        fleet.get_preferences().await.expect("reads"),
        Preferences {
            where_things_are_open: true,
            ..Default::default()
        }
    );
}

#[tokio::test]
async fn an_unknown_name_is_refused_by_name_and_saves_nothing() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));

    let refused = fleet
        .save_preferences(saving("where_things_are_purple", true))
        .await
        .expect_err("not a preference this build reads");
    match refused {
        Refusal::Unacceptable(error) => {
            assert_eq!(error.code, "fleet.unknown_preference");
            assert!(error.message.contains("where_things_are_purple"));
        }
        other => panic!("expected Unacceptable, got {other:?}"),
    }
    assert_eq!(
        fleet.get_preferences().await.expect("reads"),
        Preferences::default(),
        "nothing was saved"
    );
}

#[tokio::test]
async fn the_owners_key_bindings_are_a_preference_that_survives_a_restart() {
    // As settings.json gives JSON back: compact, its keys in order.
    const MINE: &str = r#"{"bindings":{"kill":["q"]},"version":1}"#;
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let save = |text: Option<&str>| SavePreference { name: "key_bindings".into(), value: false, text: text.map(str::to_string) };
    assert_eq!(fleet.save_preferences(save(Some(MINE))).await.expect("saved").key_bindings, MINE);
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.get_preferences().await.expect("reads").key_bindings, MINE);

    let refused = fleet.save_preferences(save(Some(r#"{"version":1,"bindings":{"kill":"q"}}"#))).await.expect_err("not a list");
    assert_eq!((refused.status(), refused.error().code.clone()), (422, "fleet.unacceptable_key_bindings".to_string()));
    assert_eq!(fleet.get_preferences().await.expect("reads").key_bindings, MINE, "nothing moved");

    assert_eq!(fleet.save_preferences(save(Some(""))).await.expect("empty takes them back").key_bindings, "");
}

#[tokio::test]
async fn the_owners_time_format_is_a_preference_that_survives_a_restart() {
    // As settings.json gives JSON back: compact, its keys in order.
    const MINE: &str = r#"{"date":true,"hour12":false,"seconds":true}"#;
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let save = |text: Option<&str>| SavePreference { name: "time_format".into(), value: false, text: text.map(str::to_string) };
    assert_eq!(fleet.save_preferences(save(Some(MINE))).await.expect("saved").time_format, MINE);
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.get_preferences().await.expect("reads").time_format, MINE);
}

#[tokio::test]
async fn a_time_format_that_is_not_three_switches_is_refused_and_saves_nothing() {
    const MINE: &str = r#"{"date":false,"hour12":true,"seconds":false}"#;
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let save = |text: &str| SavePreference { name: "time_format".into(), value: false, text: Some(text.to_string()) };
    fleet.save_preferences(save(MINE)).await.expect("saved");

    for bad in [
        "hour12",
        r#"["hour12"]"#,
        r#"{"hour12":true,"seconds":true}"#,
        r#"{"hour12":"yes","seconds":true,"date":false}"#,
        r#"{"hour12":true,"seconds":true,"date":false,"zone":true}"#,
    ] {
        let refused = fleet.save_preferences(save(bad)).await.expect_err(bad);
        assert_eq!((refused.status(), refused.error().code.clone()), (422, "fleet.unacceptable_time_format".to_string()), "{bad}");
        assert_eq!(fleet.get_preferences().await.expect("reads").time_format, MINE, "nothing moved by {bad}");
    }
}

#[tokio::test]
async fn an_empty_time_format_takes_the_choice_back() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let save = |text: &str| SavePreference { name: "time_format".into(), value: false, text: Some(text.to_string()) };
    fleet.save_preferences(save(r#"{"date":true,"hour12":true,"seconds":true}"#)).await.expect("saved");
    assert_eq!(fleet.save_preferences(save("")).await.expect("empty takes it back").time_format, "");
    assert_eq!(fleet.get_preferences().await.expect("reads"), Preferences::default());
}

fn saying(name: &str, text: &str) -> SavePreference {
    SavePreference { name: name.to_string(), value: false, text: Some(text.to_string()) }
}

#[tokio::test]
async fn how_he_works_with_jobs_reads_as_its_defaults_until_he_chooses() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let now = fleet.get_preferences().await.expect("reads");
    assert_eq!(
        (now.take_in, now.answer_with, now.read_results, now.look_results, now.look_questions),
        (TakeIn::Read, AnswerWith::Type, ReadResults::NoteThenPage, LookResults::PictureEvery, LookQuestions::PicturesMix)
    );
    assert_eq!(
        (now.first_run_answered, now.listen_end_summary, now.listen_needs_you, now.listen_progress, now.listen_operator, now.buttons_words),
        (false, true, true, false, false, true)
    );
    assert_eq!(now.listen_voice, "");
}

#[tokio::test]
async fn each_choice_of_how_he_works_survives_a_restart() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    for (name, word) in [
        ("take_in", "listen"),
        ("answer_with", "buttons"),
        ("read_results", "thread_only"),
        ("look_results", "walk_visible"),
        ("look_questions", "pictures_whole"),
        ("listen_voice", "Samantha"),
    ] {
        fleet.save_preferences(saying(name, word)).await.unwrap_or_else(|why| panic!("{name} {word}: {why:?}"));
    }
    for name in ["first_run_answered", "listen_progress", "listen_operator"] {
        fleet.save_preferences(saving(name, true)).await.expect("saved");
    }
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(
        fleet.get_preferences().await.expect("reads"),
        Preferences {
            take_in: TakeIn::Listen,
            answer_with: AnswerWith::Buttons,
            first_run_answered: true,
            read_results: ReadResults::ThreadOnly,
            look_results: LookResults::WalkVisible,
            look_questions: LookQuestions::PicturesWhole,
            listen_progress: true,
            listen_operator: true,
            listen_voice: "Samantha".into(),
            ..Preferences::default()
        }
    );
}

#[tokio::test]
async fn a_switch_that_defaults_on_stays_off_once_saved_off() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    for name in ["listen_end_summary", "listen_needs_you", "buttons_words"] {
        let now = fleet.save_preferences(saving(name, false)).await.expect("saved");
        assert!(!(now.listen_end_summary && now.listen_needs_you && now.buttons_words), "{name} stayed on");
    }
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let now = fleet.get_preferences().await.expect("reads");
    assert_eq!((now.listen_end_summary, now.listen_needs_you, now.buttons_words), (false, false, false));

    let now = fleet.save_preferences(saving("buttons_words", true)).await.expect("saved");
    assert!(now.buttons_words && !now.listen_end_summary, "one switch moved and the others stayed");
}

#[tokio::test]
async fn a_word_outside_a_closed_set_is_refused_by_name_and_saves_nothing() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    fleet.save_preferences(saying("take_in", "look")).await.expect("saved");
    for (name, word) in [("take_in", "smell"), ("take_in", ""), ("answer_with", "read"), ("read_results", "page"), ("look_results", "walk"), ("look_questions", "pictures")] {
        let refused = fleet.save_preferences(saying(name, word)).await.expect_err("not a word of the set");
        assert_eq!((refused.status(), refused.error().code.clone()), (422, "fleet.unacceptable_choice".to_string()), "{name} {word}");
        assert!(refused.error().message.contains(name) && refused.error().message.contains(&format!("`{word}`")), "{}", refused.error().message);
    }
    assert_eq!(fleet.get_preferences().await.expect("reads").take_in, TakeIn::Look, "nothing moved");
}

#[tokio::test]
async fn a_blank_listen_voice_takes_the_choice_back() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.save_preferences(saying("listen_voice", "Daniel")).await.expect("saved").listen_voice, "Daniel");
    assert_eq!(fleet.save_preferences(saying("listen_voice", "  ")).await.expect("saved").listen_voice, "");
}

/// `ipc` spells the closed sets as enums and `config` lists the same words for the file's own check. They are
/// two spellings of one set, so they are held to each other here.
#[test]
fn the_registry_and_the_wire_spell_each_closed_set_the_same() {
    use config::settings as keys;
    assert_eq!(TakeIn::WORDS, keys::TAKE_INS);
    assert_eq!(AnswerWith::WORDS, keys::ANSWER_WITHS);
    assert_eq!(ReadResults::WORDS, keys::READ_RESULTS_SET);
    assert_eq!(LookResults::WORDS, keys::LOOK_RESULTS_SET);
    assert_eq!(LookQuestions::WORDS, keys::LOOK_QUESTIONS_SET);
}
