//! The messages between a sealed page and its host, as `PageToHost` and `HostToPage` in
//! `packages/protocol/src/job-artifacts.ts` spell them: every message carries `armada: 1`, and one that
//! carries another number does not parse.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::{
    decode, encode, HostToPage, Instant, PageAct, PageMessage, PagePress, PageToHost, PressDid, SessionId, VoteCast, VoteResult,
};

const MIRROR: &str = include_str!("../../../../packages/protocol/src/job-artifacts.ts");

fn a_press() -> PagePress {
    PagePress {
        revision: 2,
        action: "go".to_string(),
        item: Some("row-1".to_string()),
        act: PageAct::Start,
        at: Instant::carried("2026-11-14T09:00:00.000Z"),
        did: PressDid::Started { job: SessionId::carried("job-2") },
    }
}

fn a_cast() -> VoteCast {
    let result = VoteResult::Pick { take: "a".to_string(), notes: BTreeMap::new() };
    VoteCast { revision: 2, vote: "take".to_string(), form: result.form(), result, at: Instant::carried("2026-11-14T09:00:00.000Z") }
}

fn to_host() -> Vec<PageToHost> {
    vec![
        PageToHost::Press { artifact: "/r/a.html".to_string(), revision: 2, action: "go".to_string(), words: Some("now".to_string()) },
        PageToHost::Press { artifact: "/r/a.html".to_string(), revision: 2, action: "go".to_string(), words: None },
        PageToHost::Vote {
            artifact: "/r/a.html".to_string(),
            revision: 2,
            vote: "take".to_string(),
            result: VoteResult::Pick { take: "a".to_string(), notes: BTreeMap::from([("b".to_string(), "no".to_string())]) },
        },
    ]
}

fn to_page() -> Vec<HostToPage> {
    vec![
        HostToPage::Pressed { press: a_press() },
        HostToPage::Voted { cast: a_cast() },
        HostToPage::Refused { action: Some("go".to_string()), vote: None, code: "fleet.page_action_unknown".to_string(), message: "No such action.".to_string() },
        HostToPage::Refused { action: None, vote: Some("take".to_string()), code: "fleet.page_vote_unknown".to_string(), message: "No such vote.".to_string() },
    ]
}

#[test]
fn a_message_to_the_host_writes_armada_1_and_reads_back_whole() {
    for message in to_host() {
        let sent = PageMessage::new(message.clone());
        let bytes = encode(&sent).unwrap().into_bytes();
        let spelled: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(spelled["armada"], json!(1), "{spelled}");
        let back: PageMessage<PageToHost> = decode("a page's message", &bytes).unwrap();
        assert_eq!(back.message, message);
    }
}

#[test]
fn a_message_to_the_page_writes_armada_1_and_reads_back_whole() {
    for message in to_page() {
        let sent = PageMessage::new(message.clone());
        let bytes = encode(&sent).unwrap().into_bytes();
        let spelled: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(spelled["armada"], json!(1), "{spelled}");
        let back: PageMessage<HostToPage> = decode("a host's message", &bytes).unwrap();
        assert_eq!(back.message, message);
    }
}

#[test]
fn the_kit_s_own_spelling_reads_and_an_absent_key_leaves_none() {
    let press: PageMessage<PageToHost> = decode(
        "a page's message",
        br#"{"armada":1,"kind":"press","artifact":"/r/a.html","revision":2,"action":"go"}"#,
    )
    .unwrap();
    assert_eq!(
        press.message,
        PageToHost::Press { artifact: "/r/a.html".to_string(), revision: 2, action: "go".to_string(), words: None }
    );
    assert_eq!(
        serde_json::to_value(PageMessage::new(press.message)).unwrap(),
        json!({"armada":1,"kind":"press","artifact":"/r/a.html","revision":2,"action":"go"})
    );
    let refused = serde_json::to_value(PageMessage::new(to_page().remove(2))).unwrap();
    assert_eq!(
        refused,
        json!({"armada":1,"kind":"refused","action":"go","code":"fleet.page_action_unknown","message":"No such action."})
    );
}

#[test]
fn a_message_that_says_another_armada_is_refused_in_both_directions() {
    for other in ["2", "0", "\"1\"", "null"] {
        let press = format!(r#"{{"armada":{other},"kind":"press","artifact":"/r/a.html","revision":2,"action":"go"}}"#);
        assert!(decode::<PageMessage<PageToHost>>("a page's message", press.as_bytes()).is_err(), "{press}");
        let refused = format!(r#"{{"armada":{other},"kind":"refused","code":"c","message":"m"}}"#);
        assert!(decode::<PageMessage<HostToPage>>("a host's message", refused.as_bytes()).is_err(), "{refused}");
    }
    let none = br#"{"kind":"press","artifact":"/r/a.html","revision":2,"action":"go"}"#;
    assert!(decode::<PageMessage<PageToHost>>("a page's message", none).is_err());
    let said = decode::<PageMessage<PageToHost>>(
        "a page's message",
        br#"{"armada":2,"kind":"press","artifact":"/r/a.html","revision":2,"action":"go"}"#,
    )
    .unwrap_err()
    .to_string();
    assert!(said.contains("`armada` is 2"), "{said}");
}

#[test]
fn a_kind_the_page_has_no_word_for_is_refused() {
    let bytes = br#"{"armada":1,"kind":"explode"}"#;
    assert!(decode::<PageMessage<PageToHost>>("a page's message", bytes).is_err());
    assert!(decode::<PageMessage<HostToPage>>("a host's message", bytes).is_err());
}

/// The mirror is written by hand, so it is read here: the keys and the `kind` words Rust sends are the ones
/// `PageToHost` and `HostToPage` declare in `job-artifacts.ts`, and nothing else is declared there.
#[test]
fn the_typescript_mirror_spells_every_key_and_kind_rust_sends_and_no_other() {
    let start = MIRROR.find("export type PageToHost").expect("PageToHost is mirrored");
    let end = MIRROR[start..].find("/** What a link on a For line").expect("a type follows") + start;
    let mirrored = &MIRROR[start..end];

    let mut sent: Vec<Value> = to_host().into_iter().map(|m| serde_json::to_value(PageMessage::new(m)).unwrap()).collect();
    sent.extend(to_page().into_iter().map(|m| serde_json::to_value(PageMessage::new(m)).unwrap()));
    let mut keys: Vec<String> = sent.iter().flat_map(|m| m.as_object().unwrap().keys().cloned()).collect();
    keys.sort();
    keys.dedup();
    // `armada`, `kind`, then every field of the six messages. `result` is a `VoteResult`, named in the file.
    assert_eq!(
        keys,
        ["action", "armada", "artifact", "cast", "code", "kind", "message", "press", "result", "revision", "vote", "words"]
    );
    for key in &keys {
        assert!(
            mirrored.contains(&format!(" {key}: ")) || mirrored.contains(&format!(" {key}?: ")),
            "job-artifacts.ts does not declare `{key}` on a page message"
        );
    }

    let kinds: Vec<String> = sent.iter().map(|m| m["kind"].as_str().unwrap().to_string()).collect();
    for kind in ["press", "vote", "pressed", "voted", "refused"] {
        assert!(kinds.iter().any(|k| k == kind), "no sample for `{kind}`");
        assert!(mirrored.contains(&format!("kind: \"{kind}\"")), "job-artifacts.ts does not spell the kind `{kind}`");
    }
    assert_eq!(mirrored.matches("kind: \"").count(), 5, "a kind is spelled there that Rust does not send");
    assert_eq!(mirrored.matches("armada: 1;").count(), 5, "every message carries `armada: 1`");
    // The optional ones are optional there, and the required ones are not.
    for optional in ["words", "action", "vote"] {
        assert!(mirrored.contains(&format!(" {optional}?: string")), "`{optional}` is optional");
    }
}
