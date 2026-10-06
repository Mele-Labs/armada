//! Kit's allowlist, `~/.armada/allowed-commands`: read from a file and enforced,
//! added to by an Always allow, taken out of by a person, and read back by
//! `get_kit_inventory`. `docs/concepts/kit.md`.

use std::path::PathBuf;

use api::PermissionAnswer;
use core_model::{Reach, WhenBlocked};
use ipc::{AlwaysAllowScope, CommandAnswer, KitAllowedSource, WhatWasRead};

use crate::daemon::Fleet;
use crate::kit_allowlist::{parse, FILE};
use crate::permitting::Answered;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, worktree_directory};
use crate::tests::permitting::{
    a_drone_that_reached_for, a_fleet_with, asked, heard, refused, started, the_fittings,
    until_waiting, Fixture,
};
use crate::tests::tmp::TempDir;

fn the_file(home: &TempDir) -> PathBuf {
    home.path().join("kit").join(FILE)
}

/// A person writing the file by hand.
fn written(home: &TempDir, text: &str) {
    let path = the_file(home);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("Kit's home");
    std::fs::write(path, text).expect("the file");
}

fn on_disk(home: &TempDir) -> String {
    std::fs::read_to_string(the_file(home)).unwrap_or_default()
}

/// A Fleet with a Job that refuses and holds, so nothing is allowed by the
/// Job's own setting and only what Kit lists can pass.
async fn refusing(home: &TempDir) -> (Fixture, core_model::JobId) {
    let fleet = a_fleet_with(home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::RefuseAndHold)
        .await
        .expect("a setting");
    (fleet, job)
}

/// **A command Kit lists runs, and one it does not is refused as before, and
/// the refusal is still recorded.** The listed command also runs with plain
/// arguments after it, and never with a chained command after it.
#[tokio::test]
async fn a_listed_command_runs_and_an_unlisted_one_is_refused_and_recorded() {
    let home = TempDir::new();
    let (fleet, job) = refusing(&home).await;
    written(&home, "grep -a -c\n");

    let listed = fleet
        .permission(&job, &asked("Bash", "grep -a -c arc-dispatch x.log", "c2"))
        .await;
    assert_eq!(listed, PermissionAnswer::Allow, "Kit lists it");

    let unlisted = fleet
        .permission(&job, &asked("Bash", "curl example.test", "c1"))
        .await;
    assert!(
        matches!(unlisted, PermissionAnswer::Deny(_)),
        "an unlisted command is refused as it was: {unlisted:?}"
    );
    assert!(
        refused(&heard(&fleet).await, "c1"),
        "and the refusal is on the record"
    );

    let chained = fleet
        .permission(
            &job,
            &asked("Bash", "grep -a -c x && curl example.test", "c3"),
        )
        .await;
    assert!(
        matches!(chained, PermissionAnswer::Deny(_)),
        "a listed command never carries a chained one: {chained:?}"
    );
}

/// **A line added while Fleet runs is in force, and one taken out is not.**
/// The file is read on the question, not held.
#[tokio::test]
async fn a_line_written_by_hand_is_read_without_a_restart() {
    let home = TempDir::new();
    let (fleet, job) = refusing(&home).await;
    let ask = || asked("Bash", "ls -la", "c1");
    assert!(matches!(
        fleet.permission(&job, &ask()).await,
        PermissionAnswer::Deny(_)
    ));

    written(&home, "# my own\nls -la\n");
    assert_eq!(
        fleet.permission(&job, &ask()).await,
        PermissionAnswer::Allow
    );

    written(&home, "# my own\n");
    assert!(matches!(
        fleet.permission(&job, &ask()).await,
        PermissionAnswer::Deny(_)
    ));
}

/// **Kit widens only what asking would have decided.** A command a Manifest
/// declares destructive stays withheld however Kit lists it.
#[tokio::test]
async fn a_destructive_command_stays_withheld_though_kit_lists_it() {
    let home = TempDir::new();
    let mut fittings = the_fittings(&home, a_drone_that_reached_for("c1"));
    fittings.starting().manifest = config::Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  publish:\n    run: \"npm publish\"\n    destructive: true\n",
    )
    .expect("a manifest that parses");
    let fleet = Fleet::assembled(fittings);
    let job = fleet
        .propose(a_proposal("publish the package"))
        .await
        .expect("proposed");
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.expect("dispatched");
    let job = job.id().clone();
    fleet
        .set_when_blocked(&job, WhenBlocked::RefuseAndHold)
        .await
        .expect("a setting");
    written(&home, "npm publish\nls\n");

    let answer = fleet
        .permission(&job, &asked("Bash", "npm publish", "c1"))
        .await;
    assert!(
        matches!(answer, PermissionAnswer::Deny(_)),
        "declared destructive, and Kit does not lift it: {answer:?}"
    );
    assert_eq!(
        fleet.permission(&job, &asked("Bash", "ls", "c2")).await,
        PermissionAnswer::Allow,
        "a plain command Kit lists runs"
    );
    assert!(
        fleet.withheld_everywhere("npm publish").is_some(),
        "and accepting a change for it would be refused"
    );
}

/// **Always allow in Kit appends once.** It is Kit's and not the repository's,
/// a later reach for the command asks nobody, and a second press leaves one line.
#[tokio::test]
async fn always_allow_in_kit_appends_once_and_the_next_reach_asks_nobody() {
    let home = TempDir::new();
    let (fleet, job) = refusing(&home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .expect("a setting");
    let asking = asked("Bash", "gh issue view 792", "c1");

    let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        fleet
            .answer_command(
                &job,
                "c1",
                Answered::naming(CommandAnswer::AlwaysAllow, None, Some("gh issue view"))
                    .kept_in(Some(AlwaysAllowScope::Kit)),
            )
            .await
    });

    answered.expect("one of the candidates the command offered");
    assert_eq!(answer, PermissionAnswer::Allow);
    assert_eq!(
        parse(&on_disk(&home)),
        vec![ipc::KitAllowedCommand {
            run: String::from("gh issue view"),
            from: KitAllowedSource::AlwaysAllow,
        }],
        "the chosen rule, and where it came from"
    );
    assert!(
        fleet.repository_allowed(&fleet.first()).await.is_empty(),
        "Kit's, not this repository's"
    );

    let again = fleet
        .allow_in_kit("gh issue view", KitAllowedSource::AlwaysAllow)
        .await
        .expect("a command already listed");
    assert!(!again, "pressing it again adds nothing");
    assert_eq!(parse(&on_disk(&home)).len(), 1, "{}", on_disk(&home));
    assert_eq!(
        fleet
            .permission(&job, &asked("Bash", "gh issue view 800", "c2"))
            .await,
        PermissionAnswer::Allow,
        "the next reach is answered from Kit"
    );
}

/// **A scope is read by Always allow alone**, and a rule the command never
/// offered is refused for Kit as it is for the repository.
#[test]
fn a_scope_rides_only_an_always_allow() {
    assert_eq!(
        Answered::naming(CommandAnswer::AllowForJob, None, None)
            .kept_in(Some(AlwaysAllowScope::Kit)),
        Answered::Allowed(Reach::Job, None)
    );
    assert_eq!(
        Answered::naming(CommandAnswer::AlwaysAllow, None, Some("gh")).kept_in(None),
        Answered::Allowed(Reach::Repository, Some(String::from("gh")))
    );
    assert_eq!(
        Answered::naming(CommandAnswer::AlwaysAllow, None, Some("gh"))
            .kept_in(Some(AlwaysAllowScope::Kit))
            .answer(),
        CommandAnswer::AlwaysAllow,
        "still the one offer Bridge matches on"
    );
}

/// **A command that chains another is never kept**, and neither is one a line
/// could not hold.
#[tokio::test]
async fn a_chained_command_is_not_kept() {
    let home = TempDir::new();
    let (fleet, _job) = refusing(&home).await;
    for bad in ["ls && rm -rf x", "ls | sh", "echo $(id)", "  ", "a\tb"] {
        let kept = fleet.allow_in_kit(bad, KitAllowedSource::AlwaysAllow).await;
        assert!(kept.is_err(), "{bad:?}: {kept:?}");
    }
    assert_eq!(on_disk(&home), "", "nothing was written");
}

/// **Lines are read as written**: notes and blanks skipped, a line Armada added
/// says where it came from, and one a person wrote needs nothing.
#[test]
fn the_file_reads_as_a_person_would_write_it() {
    let text = "# Commands.\n\nls -la\ngh issue view\talways allow\n\
                grep -a\tretro 01JOB-3\r\nrg\twhatever a person typed\n   \n";
    let rows: Vec<(String, KitAllowedSource)> = parse(text)
        .into_iter()
        .map(|row| (row.run, row.from))
        .collect();
    assert_eq!(
        rows,
        vec![
            (String::from("ls -la"), KitAllowedSource::ByHand),
            (String::from("gh issue view"), KitAllowedSource::AlwaysAllow),
            (
                String::from("grep -a"),
                KitAllowedSource::RetroItem {
                    lesson_id: String::from("01JOB-3")
                }
            ),
            (String::from("rg"), KitAllowedSource::ByHand),
        ]
    );
}

/// **Removing takes one line out and leaves the rest as a person left them**,
/// and a command the file does not hold is refused.
#[tokio::test]
async fn removing_takes_one_command_out_and_leaves_the_rest() {
    let home = TempDir::new();
    let (fleet, job) = refusing(&home).await;
    written(
        &home,
        "# mine\nls -la\ngh issue view\talways allow\ngrep -a\tretro 01JOB-3\n",
    );

    let left = fleet
        .remove_from_kit_allowlist("gh issue view")
        .await
        .expect("it was there");
    let names: Vec<&str> = left.commands.iter().map(|row| row.run.as_str()).collect();
    assert_eq!(names, ["ls -la", "grep -a"]);
    assert_eq!(
        on_disk(&home),
        "# mine\nls -la\ngrep -a\tretro 01JOB-3\n",
        "the note and the other lines are as they were"
    );
    assert!(matches!(
        fleet
            .permission(&job, &asked("Bash", "gh issue view 1", "c1"))
            .await,
        PermissionAnswer::Deny(_)
    ));

    let missing = fleet.remove_from_kit_allowlist("gh issue view").await;
    assert!(missing.is_err(), "nothing is spelled that now");
}

/// **`get_kit_inventory` reads the allowlist rather than saying not read yet**,
/// with where each command came from and nothing else. A file that is there
/// and will not read is `not_read`, never an empty list.
#[tokio::test]
async fn the_inventory_reads_the_allowlist_with_where_each_came_from() {
    let home = TempDir::new();
    let (fleet, _job) = refusing(&home).await;
    let row = |inventory: ipc::KitInventory| {
        inventory
            .kinds
            .into_iter()
            .find(|row| row.kind == "allowlist")
            .expect("the allowlist is a kind of Kit")
            .read
    };

    let WhatWasRead::Read { items, .. } = row(fleet.kit_inventory().await) else {
        panic!("an absent file is an empty list, and it was looked at");
    };
    assert!(items.is_empty());

    written(
        &home,
        "ls -la\ngh issue view\talways allow\ngrep -a\tretro 01JOB-3\n",
    );
    let WhatWasRead::Read { items, unreadable } = row(fleet.kit_inventory().await) else {
        panic!("the allowlist is read");
    };
    assert!(unreadable.is_empty());
    let seen: Vec<(&str, &str, Option<&str>)> = items
        .iter()
        .map(|item| {
            (
                item.name.as_str(),
                item.source.as_str(),
                item.says.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("ls -la", "written by hand", None),
            ("gh issue view", "always allow", None),
            ("grep -a", "retro item 01JOB-3", None),
        ],
        "the command text and where it came from, and nothing that could widen it"
    );

    std::fs::remove_file(the_file(&home)).expect("a file to replace");
    std::fs::create_dir(the_file(&home)).expect("a directory where the file is");
    assert!(
        matches!(
            row(fleet.kit_inventory().await),
            WhatWasRead::NotRead { .. }
        ),
        "a file that will not read is said so"
    );
}
