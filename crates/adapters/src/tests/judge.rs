//! What a Judge call is, as a value.
//!
//! No process, the same as the harness cases beside these: whether a call
//! carries a session, a toolset or a directory is a question about a rendering.

use std::num::NonZeroU8;

use adapter_traits::{Ask, Environment, Model, ModelClient, Reading, SpawnConfigRefused};

use crate::HeadlessAgent;

fn ask() -> Ask {
    Ask::put(
        Model::named("the-cheap-model").expect("a model name"),
        "Does the fix address the cause the note names?",
        Environment::nothing()
            .and("PATH", "/usr/bin:/bin")
            .expect("a legal variable"),
    )
    .expect("a legal ask")
}

/// The same ask, able to read a checkout for six turns.
fn reading() -> Ask {
    ask().reading(
        Reading::checkout("/repos/armada/", NonZeroU8::new(6).expect("six"))
            .expect("an absolute checkout"),
    )
}

fn arg_after(args: &[String], flag: &str) -> Option<String> {
    let at = args.iter().position(|arg| arg == flag)?;
    args.get(at + 1).cloned()
}

/// The one-shot properties of an ask given nothing to read, each as a flag on
/// the list: the proposer, generated copy and a link lookup. A call that could
/// take a second turn could go looking, and nothing asked this way has
/// anywhere to look.
#[test]
fn a_judge_call_takes_one_turn_and_holds_no_tool() {
    let call = HeadlessAgent::on_path().render(&ask());
    let args = call.args();
    assert_eq!(arg_after(args, "--max-turns").as_deref(), Some("1"));
    assert_eq!(arg_after(args, "--allowedTools").as_deref(), Some(""));
    assert!(args.iter().any(|arg| arg == "--strict-mcp-config"));
    // Strict with no configuration beside it is the empty set. A Judge does not
    // even hold the Evidence tool a Drone always has.
    assert!(!args.iter().any(|arg| arg == "--mcp-config"));
}

/// `--allowedTools ""` denies each use and leaves the toolset standing; a
/// single denied use still spends `--max-turns 1` and the call exits 1. Only
/// `--tools ""` disables the toolset itself, and both renders of an ask with no
/// reading carry it. `#1047`.
#[test]
fn both_renders_carry_no_tools_at_all() {
    let plain = HeadlessAgent::on_path().render(&ask());
    let watched = HeadlessAgent::on_path().render_watched(&ask());
    assert_eq!(arg_after(plain.args(), "--tools").as_deref(), Some(""));
    assert_eq!(arg_after(watched.args(), "--tools").as_deref(), Some(""));
}

/// It is not a session. A Drone's stdin carries one JSON object per line for
/// the life of the Job; this one carries a question and then closes.
#[test]
fn a_judge_call_is_not_a_session() {
    let call = HeadlessAgent::on_path().render(&ask());
    assert!(!call.args().iter().any(|arg| arg == "--input-format"));
    assert!(!call
        .args()
        .iter()
        .any(|arg| arg == "--replay-user-messages"));
}

/// Nothing readable in argv, for the reason a Drone's prompt is not there: `ps`
/// prints a same-uid child's argument list. A criterion quotes the work.
#[test]
fn the_question_is_on_stdin_and_not_on_the_argument_list() {
    let call = HeadlessAgent::on_path().render(&ask());
    assert_eq!(
        call.question(),
        "Does the fix address the cause the note names?"
    );
    assert!(
        !call.args().iter().any(|arg| arg.contains("the note names")),
        "{:?}",
        call.args()
    );
}

/// **There is no field for a worktree**, so a Judge cannot be pointed at one,
/// and an ask that reads nothing names no directory at all. The environment is
/// the caller's and the adapter cannot substitute another.
#[test]
fn a_judge_call_names_no_directory_and_carries_the_environment_it_was_given() {
    let call = HeadlessAgent::on_path().render(&ask());
    assert_eq!(call.directory(), None);
    assert_eq!(call.environment().names(), vec!["PATH"]);
    assert!(
        !call.args().iter().any(|arg| arg.contains("worktree")),
        "{:?}",
        call.args()
    );
}

/// The per-step dial is what arrives; the default is what it moves away from.
#[test]
fn the_model_on_the_list_is_the_one_the_ask_named() {
    let call = HeadlessAgent::on_path().render(&ask());
    assert_eq!(
        arg_after(call.args(), "--model").as_deref(),
        Some("the-cheap-model")
    );
    assert!(HeadlessAgent::models().contains(&HeadlessAgent::judge_model()));
}

/// The default is `crates/config/settings.toml`'s decision, and this is where
/// the two are held to each other.
///
/// **The derivation is what needs pinning, not the string.** `judge_model` does
/// not write the value down — it takes the last entry of the roster, on the
/// assumption that the roster runs strongest first. That assumption is about a
/// list this module does not own, so a reordered or extended roster would
/// redecide a setting without anybody deciding anything. Here it costs a red
/// test instead, which is the point at which somebody either updates the row or
/// puts the order back.
#[test]
fn the_default_judge_model_is_the_value_the_settings_row_carries() {
    assert_eq!(HeadlessAgent::judge_model(), "haiku");
    // Cheapest, not merely present: it is the end of the roster rather than
    // somewhere in the middle of it.
    assert_eq!(
        HeadlessAgent::models().last().copied(),
        Some(HeadlessAgent::judge_model())
    );
    // And below the model a Job itself gets. A Judge that cost what the work it
    // checks costs would be a second Drone rather than a veto.
    assert_ne!(HeadlessAgent::judge_model(), HeadlessAgent::default_model());
}

/// A flag is read again by a model on the roster and stronger than the one that
/// raised it, or the second reading is the first look asked twice.
#[test]
fn a_flag_is_read_again_on_a_stronger_model_than_raised_it() {
    assert_eq!(HeadlessAgent::second_opinion_model(), "sonnet");
    let roster = HeadlessAgent::models();
    let at = |model| roster.iter().position(|named| *named == model);
    assert!(
        at(HeadlessAgent::second_opinion_model()) < at(HeadlessAgent::judge_model()),
        "the roster runs strongest first"
    );
}

/// A retro is written on the middle tier, a member of the roster and stronger
/// than the Judge's cheap one, as `retro-model` decided on 4 Oct 2026.
#[test]
fn a_retro_is_written_on_a_roster_model_stronger_than_the_judges() {
    assert_eq!(HeadlessAgent::retro_model(), "sonnet");
    let roster = HeadlessAgent::models();
    let at = |model| roster.iter().position(|named| *named == model);
    assert!(
        at(HeadlessAgent::retro_model()).is_some(),
        "a member of the roster"
    );
    assert!(
        at(HeadlessAgent::retro_model()) < at(HeadlessAgent::judge_model()),
        "the roster runs strongest first"
    );
}

/// Decided 2 Oct 2026: **a Judge reads the repository, and nothing more.** The
/// three read tools are the whole toolset and the whole allowlist; every
/// built-in that writes, runs a command or reaches the network is denied by
/// name, and so is Fleet's own directory, where the Drone's worktree is. Both
/// renders, because a watched call is not a call with a longer leash.
#[test]
fn a_reading_judge_holds_the_read_tools_and_nothing_that_writes() {
    let agent = HeadlessAgent::on_path();
    for call in [agent.render(&reading()), agent.render_watched(&reading())] {
        let args = call.args();
        let reads = Some("Read,Grep,Glob".to_string());
        assert_eq!(arg_after(args, "--tools"), reads);
        assert_eq!(arg_after(args, "--allowedTools"), reads);
        assert!(args.iter().any(|arg| arg == "--restricted"), "{args:?}");
        assert!(args.iter().any(|arg| arg == "--strict-mcp-config"));
        assert!(!args.iter().any(|arg| arg == "--mcp-config"));

        let denied = arg_after(args, "--disallowedTools").expect("a deny list");
        let denied: Vec<&str> = denied.split(',').collect();
        for not_a_read in [
            "Bash",
            "Edit",
            "Write",
            "NotebookEdit",
            "Task",
            "WebFetch",
            "WebSearch",
            "Read(./.armada/**)",
        ] {
            assert!(denied.contains(&not_a_read), "`{not_a_read}`: {denied:?}");
        }
        for read in ["Read", "Grep", "Glob"] {
            assert!(!denied.contains(&read), "a Judge reads: {read}");
        }
    }
}

/// The cap is the reading's, and it is the only `--max-turns` on the list.
#[test]
fn a_reading_judge_takes_the_turns_its_reading_names() {
    let call = HeadlessAgent::on_path().render(&reading());
    assert_eq!(arg_after(call.args(), "--max-turns").as_deref(), Some("6"));
    assert_eq!(
        call.args()
            .iter()
            .filter(|arg| *arg == "--max-turns")
            .count(),
        1
    );
}

/// The call starts in the checkout it was given, copied off the ask, and the
/// question still goes in on stdin rather than the list.
#[test]
fn a_reading_judge_starts_in_the_checkout_its_ask_names() {
    let call = HeadlessAgent::on_path().render(&reading());
    assert_eq!(call.directory(), Some("/repos/armada"));
    assert!(
        !call.args().iter().any(|arg| arg.contains("/repos/armada")),
        "the directory is where it starts, not an argument: {:?}",
        call.args()
    );
    assert!(!call.args().iter().any(|arg| arg.contains("the note names")));
}

/// A relative checkout would be read wherever the call happened to start.
#[test]
fn a_relative_checkout_is_refused() {
    assert_eq!(
        Reading::checkout("repos/armada", NonZeroU8::new(6).expect("six")),
        Err(SpawnConfigRefused::CheckoutNotAbsolute {
            given: "repos/armada".to_string()
        })
    );
}
