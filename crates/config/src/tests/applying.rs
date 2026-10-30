//! Which lifecycles apply to one repository: the narrower scope wins, then yours over the
//! repository's, and the one that loses is still listed.

use std::path::PathBuf;

use core_model::{Lifecycle, LifecycleId, ManifestId, PieceSource, Ulid};

use crate::piece::Written;
use crate::{Lifecycles, Rank, Reach, Workspaces};

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn id(written: &str) -> LifecycleId {
    LifecycleId::new(written).expect("an id")
}

/// A ticket whose `In review` column goes by `word`, scoped to `scope` where one is given.
fn ticket(scope: Option<&str>, word: &str) -> String {
    let scope = scope.map(|one| format!("scope: {one}\n")).unwrap_or_default();
    format!(
        "version: 1\nlifecycle_id: ticket\nname: Ticket\n{scope}states:\n  - {{ id: in_review, label: In review, read: {{ tracker: [{word}] }} }}\n  - {{ id: closed, label: Closed, read: {{ tracker: [Done] }} }}\nends: [closed]\n"
    )
}

fn in_kit(file: &str, text: String) -> Written {
    Written::in_kit(PathBuf::from("kit/lifecycles").join(file), text)
}

fn in_repository(root: &str, file: &str, text: String) -> Written {
    Written::in_repository(PathBuf::from(root).join(".armada/lifecycles").join(file), text)
}

/// The words the `ticket` that runs lists for In review: which file won, as a person would say it.
fn running_words(applying: &Lifecycles) -> String {
    let ticket = applying.get(&id("ticket")).expect("a ticket");
    format!("{:?}", ticket.states()[0].read())
}

fn my_store() -> Workspaces {
    let file = "version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n  - store-web\n  - store-api\n";
    Workspaces::of([Written::in_repository(PathBuf::from("store-web/.armada/workspaces/my-store.yml"), file.into())])
}

fn applying(to: &str, written: Vec<Written>) -> Lifecycles {
    Lifecycles::applying_to(&repository(to), &my_store(), written)
}

#[test]
fn a_repository_with_no_files_runs_the_four_armada_carries_and_nothing_else() {
    let none = applying("store-web", Vec::new());
    let mut ran: Vec<&str> = none.running().map(|one| one.id().as_str()).collect();
    ran.sort_unstable();
    assert_eq!(ran, ["deploy", "job", "pr", "ticket"]);
    assert!(none.left_out().is_empty());
    assert!(none.files().iter().all(|file| file.rank().source == PieceSource::Armada && file.path().is_none()));
    assert_eq!(none.get(&id("ticket")), Some(&Lifecycle::ticket()));
}

#[test]
fn a_lifecycle_scoped_to_a_workspace_applies_in_each_of_its_repositories_and_in_no_other() {
    let written = || vec![in_kit("tickets.yml", ticket(Some("my-store"), "Code Review"))];
    for member in ["store-web", "store-api"] {
        assert!(running_words(&applying(member, written())).contains("Code Review"), "{member}");
    }
    let outside = applying("back-office", written());
    assert_eq!(outside.get(&id("ticket")), Some(&Lifecycle::ticket()), "the Workspace does not reach it");
    assert_eq!(outside.files().len(), 4, "a lifecycle that applies nowhere here is not listed");
}

#[test]
fn a_lifecycle_scoped_to_one_repository_applies_there_alone() {
    let written = || vec![in_kit("tickets.yml", ticket(Some("store-api"), "Peer Review"))];
    assert!(running_words(&applying("store-api", written())).contains("Peer Review"));
    assert_eq!(applying("store-web", written()).get(&id("ticket")), Some(&Lifecycle::ticket()));
}

#[test]
fn the_narrower_scope_wins_whichever_place_kept_it() {
    // The repository's own file for the repository outranks Kit's for the Workspace, which outranks
    // Kit's for everywhere, which outranks what Armada carries.
    let applying = applying(
        "store-web",
        vec![
            in_kit("everywhere.yml", ticket(None, "Everywhere")),
            in_kit("workspace.yml", ticket(Some("my-store"), "Workspace")),
            in_repository("store-web", "repository.yml", ticket(Some("store-web"), "Repository")),
        ],
    );
    assert!(running_words(&applying).contains("Repository"));
    let mut tickets: Vec<(Reach, PieceSource)> = applying
        .files()
        .iter()
        .filter(|file| file.lifecycle().id() == &id("ticket"))
        .map(|file| (file.rank().reach, file.rank().source))
        .collect();
    assert_eq!(
        tickets.remove(0),
        (Reach::Repository, PieceSource::Repository),
        "the one that runs is listed first"
    );
    assert_eq!(
        tickets,
        [
            (Reach::Workspace, PieceSource::Kit),
            (Reach::Everywhere, PieceSource::Kit),
            (Reach::Everywhere, PieceSource::Armada)
        ]
    );
    let winner = Rank { reach: Reach::Repository, source: PieceSource::Repository, home: false };
    assert!(applying.files().iter().filter(|file| file.lifecycle().id() == &id("ticket")).skip(1).all(|file| file.overridden_by() == Some(winner)));
}

#[test]
fn at_one_scope_yours_wins_over_the_repositorys_and_the_one_it_hides_is_listed() {
    let applying = applying(
        "store-web",
        vec![
            in_repository("store-web", "tickets.yml", ticket(Some("my-store"), "Theirs")),
            in_kit("tickets.yml", ticket(Some("my-store"), "Mine")),
        ],
    );
    assert!(running_words(&applying).contains("Mine"));
    let [mine, theirs, _carried] = applying.files().iter().filter(|file| file.lifecycle().id() == &id("ticket")).collect::<Vec<_>>()[..] else {
        panic!("three tickets are listed")
    };
    assert_eq!((mine.hides(), mine.overridden_by()), (Some(PieceSource::Repository), None));
    assert_eq!(theirs.overridden_by(), Some(mine.rank()));
    assert!(theirs.path().is_some_and(|path| path.starts_with("store-web")), "it is still the repository's file");
}

#[test]
fn two_copies_that_are_the_same_hide_nothing() {
    let same = || ticket(Some("my-store"), "Code Review");
    let applying = applying(
        "store-web",
        vec![in_repository("store-web", "tickets.yml", same()), in_kit("tickets.yml", same())],
    );
    assert!(applying.files().iter().all(|file| file.hides().is_none()));
}

#[test]
fn a_person_can_replace_what_armada_carries_for_every_repository() {
    let applying = applying("back-office", vec![in_kit("tickets.yml", ticket(None, "Peer Review"))]);
    assert!(running_words(&applying).contains("Peer Review"));
    assert!(applying
        .files()
        .iter()
        .any(|file| file.rank().source == PieceSource::Armada && file.lifecycle().id() == &id("ticket") && file.overridden_by().is_some()));
}

#[test]
fn two_files_with_one_id_at_one_scope_in_one_place_are_both_left_out_and_the_next_answers() {
    let applying = applying(
        "store-web",
        vec![
            in_kit("a.yml", ticket(Some("my-store"), "One")),
            in_kit("b.yml", ticket(Some("my-store"), "Two")),
        ],
    );
    assert_eq!(applying.get(&id("ticket")), Some(&Lifecycle::ticket()), "neither is guessed to be the one");
    let [left] = applying.left_out() else { panic!("one entry: {:?}", applying.left_out()) };
    assert!(left.said().contains("kit/lifecycles/a.yml") && left.said().contains("kit/lifecycles/b.yml"), "{}", left.said());
}

#[test]
fn a_file_that_does_not_parse_is_left_out_for_every_repository_and_says_why() {
    for to in ["store-web", "back-office"] {
        let applying = applying(to, vec![in_kit("broken.yml", String::from("version: 1\nlifecycle_id: ticket\n"))]);
        let [left] = applying.left_out() else { panic!("{to}: one entry") };
        assert!(left.path().ends_with("broken.yml") && left.said().contains("name"), "{to}: {left:?}");
        assert_eq!(applying.get(&id("ticket")), Some(&Lifecycle::ticket()));
    }
}

/// A file read from the home of `my-store`, the repository the Workspace is committed to.
fn from_the_home(file: &str, text: String) -> Written {
    let workspace = core_model::ScopeId::new("my-store").expect("a scope");
    Written::in_home_of(workspace, PathBuf::from("store-web/.armada/lifecycles").join(file), text)
}

#[test]
fn a_member_takes_from_the_home_only_what_is_scoped_to_the_workspace() {
    let from_home = |scope: Option<&str>| applying("store-api", vec![from_the_home("tickets.yml", ticket(scope, "Home"))]);
    let shared = from_home(Some("my-store"));
    assert!(running_words(&shared).contains("Home"));
    for stays_the_homes in [None, Some("store-web"), Some("store-api"), Some("another-workspace")] {
        let applying = from_home(stays_the_homes);
        assert_eq!(applying.get(&id("ticket")), Some(&Lifecycle::ticket()), "{stays_the_homes:?}");
        assert_eq!(applying.files().len(), 4, "{stays_the_homes:?} is not listed, as nothing here applies");
        assert!(applying.left_out().is_empty(), "{stays_the_homes:?} is not a fault either");
    }
}

#[test]
fn among_committed_copies_at_one_scope_the_homes_is_the_one_a_member_sees() {
    let applying = applying(
        "store-api",
        vec![
            in_repository("store-api", "tickets.yml", ticket(Some("my-store"), "Own")),
            from_the_home("tickets.yml", ticket(Some("my-store"), "Home")),
        ],
    );
    assert!(running_words(&applying).contains("Home"));
    assert!(applying.left_out().is_empty(), "two committed copies at one scope are not a fault when one is the home's");
    let own = applying
        .files()
        .iter()
        .find(|file| file.path().is_some_and(|path| path.starts_with("store-api")))
        .expect("the member's own copy is still listed");
    assert_eq!(own.overridden_by().map(|rank| (rank.source, rank.home)), Some((PieceSource::Repository, true)));
}

#[test]
fn yours_in_kit_still_wins_over_the_homes() {
    let applying = applying(
        "store-api",
        vec![from_the_home("tickets.yml", ticket(Some("my-store"), "Home")), in_kit("tickets.yml", ticket(Some("my-store"), "Mine"))],
    );
    assert!(running_words(&applying).contains("Mine"));
    let home = applying.files().iter().find(|file| file.rank().home).expect("the home's copy is listed");
    assert!(home.overridden_by().is_some_and(|rank| rank.source == PieceSource::Kit));
}
