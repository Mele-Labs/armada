//! The files a teaching writes, as one type for the file and the wire: `found` and `where` are one
//! string each, and a spelling nobody sanctioned does not decode.

use crate::{decode, encode, AddressFound, Does, FleetRule, ModKind, ModSummary, RuleScope, WordsAddress};

#[test]
fn the_new_kinds_are_spelled_as_their_files_are_named_and_taught_on_is_left_out_when_absent() {
    for (kind, word) in [(ModKind::Words, "words"), (ModKind::Fleet, "fleet"), (ModKind::Tweak, "tweak")] {
        assert_eq!(encode(&kind).expect("plain data"), format!("\"{word}\""));
    }
    let row = ModSummary {
        name: "jira".into(),
        kind: Some(ModKind::Words),
        version: None,
        description: None,
        taught_on: Some("ARM-123 in a message".into()),
        enabled: true,
        valid: true,
        reason: None,
        changed_at: None,
        tried: None,
    };
    let json = encode(&row).expect("plain data");
    assert!(json.contains(r#""taught_on":"ARM-123 in a message""#), "{json}");
    let bare = encode(&ModSummary { taught_on: None, ..row }).expect("plain data");
    assert!(!bare.contains("taught_on"), "{bare}");
}

#[test]
fn found_is_one_string_that_says_where_an_address_came_from() {
    for (found, spelled) in [
        (AddressFound::Settings, "settings"),
        (AddressFound::Person, "person"),
        (AddressFound::PullRequest(2795), "pull_request #2795"),
    ] {
        let address = WordsAddress { repository: "armada".into(), template: "https://x.example/{match}".into(), found };
        let json = encode(&address).expect("plain data");
        assert!(json.contains(&format!("\"found\":\"{spelled}\"")), "{json}");
        assert_eq!(decode::<WordsAddress>("an address", json.as_bytes()).expect("round-trips"), address);
    }
    for bad in ["pull_request #0", "pull_request 12", "guessed", ""] {
        let body = format!(r#"{{"repository":"a","template":"https://x.example/{{match}}","found":"{bad}"}}"#);
        decode::<WordsAddress>("an address", body.as_bytes()).expect_err(bad);
    }
}

#[test]
fn a_rule_says_where_in_one_string_and_a_place_that_does_not_parse_is_refused() {
    let rule = FleetRule { notice: "behind_base".into(), does: Does::Batch, scope: RuleScope::Repo("armada".into()) };
    let json = encode(&rule).expect("plain data");
    assert_eq!(json, r#"{"notice":"behind_base","does":"batch","where":"repo armada"}"#);
    assert_eq!(decode::<FleetRule>("a rule", json.as_bytes()).expect("round-trips"), rule);
    for place in ["job 84ed9507", "everywhere"] {
        let body = format!(r#"{{"notice":"behind_base","does":"call","where":"{place}"}}"#);
        assert!(decode::<FleetRule>("a rule", body.as_bytes()).is_ok(), "{place}");
    }
    for bad in ["job", "job a b", "repo", "nowhere", "Everywhere", "job ../x"] {
        let body = format!(r#"{{"notice":"behind_base","does":"call","where":"{bad}"}}"#);
        decode::<FleetRule>("a rule", body.as_bytes()).expect_err(bad);
    }
}
