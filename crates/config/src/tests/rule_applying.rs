//! Which rules apply to one repository: the same precedence as lifecycles', and a rule that names
//! what its lifecycles lack is set aside with the sentence.

use std::path::PathBuf;

use core_model::rule::RuleId;
use core_model::{Lifecycle, ManifestId, PieceSource, Ulid};

use crate::piece::Written;
use crate::{Rank, Reach, Rules, Workspaces};

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn id(written: &str) -> RuleId {
    RuleId::new(written).expect("a rule id")
}

/// A rule that moves tickets into `to` when planning starts, scoped to `scope` where one is given.
fn moves_tickets(scope: Option<&str>, to: &str) -> String {
    let scope = scope.map(|one| format!("scope: {one}\n")).unwrap_or_default();
    format!(
        "version: 1\nrule_id: plan_moves_tickets\nname: Move tickets\n{scope}when: {{ step_starts: plan }}\ndo:\n  - tracker_write: {{ move: {to}, on: {{ held: ticket }} }}\n"
    )
}

fn in_kit(file: &str, text: String) -> Written {
    Written::in_kit(PathBuf::from("kit/rules").join(file), text)
}

fn in_repository(root: &str, file: &str, text: String) -> Written {
    Written::in_repository(PathBuf::from(root).join(".armada/rules").join(file), text)
}

fn my_store() -> Workspaces {
    let file = "version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n  - store-web\n  - store-api\n";
    Workspaces::of([Written::in_repository(PathBuf::from("store-web/.armada/workspaces/my-store.yml"), file.into())])
}

fn applying(to: &str, written: Vec<Written>) -> Rules {
    let carried = Lifecycle::built_in();
    let lifecycles: Vec<&Lifecycle> = carried.iter().collect();
    Rules::applying_to(&repository(to), &my_store(), &lifecycles, written)
}

/// The state the rule that runs moves tickets to, as a person would say which file won.
fn moves_to(applying: &Rules) -> String {
    format!("{:?}", applying.get(&id("plan_moves_tickets")).expect("a rule").does())
}

#[test]
fn with_no_rule_file_nothing_applies() {
    let none = applying("store-web", Vec::new());
    assert!(none.files().is_empty() && none.left_out().is_empty());
    assert!(none.frozen().is_empty(), "so a Job started now is handed no rule, and the engine plans nothing");
}

#[test]
fn the_narrower_scope_wins_whichever_place_kept_it_and_the_losers_are_listed() {
    let applying = applying(
        "store-web",
        vec![
            in_kit("everywhere.yml", moves_tickets(None, "todo")),
            in_kit("workspace.yml", moves_tickets(Some("my-store"), "in_progress")),
            in_repository("store-web", "repository.yml", moves_tickets(Some("store-web"), "closed")),
        ],
    );
    assert!(moves_to(&applying).contains("closed"));
    let reach_and_place: Vec<(Reach, PieceSource)> =
        applying.files().iter().map(|file| (file.rank().reach, file.rank().source)).collect();
    assert_eq!(
        reach_and_place,
        [
            (Reach::Repository, PieceSource::Repository),
            (Reach::Workspace, PieceSource::Kit),
            (Reach::Everywhere, PieceSource::Kit)
        ]
    );
    let winner = Rank { reach: Reach::Repository, source: PieceSource::Repository, home: false };
    assert!(applying.files().iter().skip(1).all(|file| file.overridden_by() == Some(winner)));
    assert_eq!(applying.frozen().len(), 1, "one rule for each id is what a Job freezes");
}

#[test]
fn a_rule_scoped_to_a_workspace_applies_in_each_of_its_repositories_and_in_no_other() {
    let written = || vec![in_kit("moves.yml", moves_tickets(Some("my-store"), "in_progress"))];
    for member in ["store-web", "store-api"] {
        assert_eq!(applying(member, written()).files().len(), 1, "{member}");
    }
    assert!(applying("back-office", written()).files().is_empty(), "the Workspace does not reach it");
}

#[test]
fn at_one_scope_yours_wins_over_the_repositorys_and_the_one_it_hides_is_listed() {
    let applying = applying(
        "store-web",
        vec![
            in_repository("store-web", "moves.yml", moves_tickets(Some("my-store"), "closed")),
            in_kit("moves.yml", moves_tickets(Some("my-store"), "in_progress")),
        ],
    );
    assert!(moves_to(&applying).contains("in_progress"));
    let [mine, theirs] = applying.files() else { panic!("both are listed") };
    assert_eq!(mine.hides(), Some(PieceSource::Repository), "the mark Bridge shows");
    assert_eq!(theirs.overridden_by(), Some(mine.rank()));
}

#[test]
fn a_member_reads_the_rules_its_workspace_home_scopes_to_it() {
    let home = |scope: &str| {
        Written::in_home_of(
            core_model::ScopeId::new("my-store").expect("a scope"),
            PathBuf::from("store-web/.armada/rules/moves.yml"),
            moves_tickets(Some(scope), "in_progress"),
        )
    };
    assert_eq!(applying("store-api", vec![home("my-store")]).files().len(), 1);
    assert!(applying("store-api", vec![home("store-web")]).files().is_empty(), "the home's own stay the home's");
}

#[test]
fn a_rule_naming_a_state_its_lifecycle_lacks_is_set_aside_and_says_so() {
    let typo = "version: 1\nrule_id: typo\nname: Typo\nwhen: { enters: { lifecycle: pr, state: aproved } }\ndo:\n  - tell_agent: { wake: true }\n";
    let nowhere = "version: 1\nrule_id: nowhere\nname: Nowhere\nwhen: { enters: { lifecycle: release, state: shipped } }\ndo:\n  - tell_agent: { wake: true }\n";
    let applying = applying("store-web", vec![in_kit("typo.yml", typo.into()), in_kit("nowhere.yml", nowhere.into())]);
    assert!(applying.files().is_empty());
    let said: Vec<&str> = applying.left_out().iter().map(|left| left.said()).collect();
    assert!(said[0].contains("names the state `aproved`, which the `pr` lifecycle does not have"), "{said:?}");
    assert!(said[1].contains("names the `release` lifecycle, which does not apply here"), "{said:?}");
}

#[test]
fn a_file_that_does_not_load_is_left_out_for_every_repository_and_the_others_stand() {
    let applying = applying(
        "store-web",
        vec![in_kit("broken.yml", "version: 1\nname: Broken\n".into()), in_kit("moves.yml", moves_tickets(None, "closed"))],
    );
    assert_eq!(applying.files().len(), 1);
    assert_eq!(applying.left_out().len(), 1);
    assert!(applying.left_out()[0].said().contains("`rule_id` is required"));
    assert_eq!(applying.left_out()[0].source(), PieceSource::Kit);
}
