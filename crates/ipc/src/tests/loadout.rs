//! The Loadout on the wire: the mod's two facts as the contract spells them, and the summary's counts.
//! `docs/practices/protocol.md`, *A Session's Loadout*.

use crate::{
    decode, encode, Instant, LoadoutItem, LoadoutItemBody, LoadoutItemFormat, LoadoutKind, LoadoutSource, LoadoutState,
    LoadoutSummary, LoadoutUse, LoadoutUseKind, SessionFact,
};

#[test]
fn the_mods_equipped_and_used_facts_read_as_the_contract_spells_them() {
    let equipped: SessionFact = decode(
        "a fact",
        br#"{"kind":"equipped","loadout":{"items":[
            {"kind":"skill","name":"land","source":"repository"},
            {"kind":"mcp_server","name":"forge","source":"mcp","from":"forge","state":"connected"},
            {"kind":"tool","name":"mcp__forge__get_me","source":"mcp","from":"forge","state":"deferred","tokens":120},
            {"kind":"output_style","name":"Explanatory","source":"built_in"},
            {"kind":"rules","name":"policy","source":"managed"}]}}"#,
    )
    .expect("a list");
    let SessionFact::Equipped { loadout } = equipped else { panic!("a list") };
    assert_eq!(loadout.items[1].state, Some(LoadoutState::Connected));
    assert_eq!(loadout.items[2].tokens, Some(120));
    assert_eq!(loadout.items[3].source, LoadoutSource::BuiltIn);
    assert_eq!(loadout.items[4].source, LoadoutSource::Managed);

    let used: SessionFact = decode(
        "a fact",
        br#"{"kind":"used","use":{"turn_started_at":"2026-10-17T09:00:00.000Z","kind":"subagent","name":"rust-engineer","source":"kit","model":"opus","by":"a1"}}"#,
    )
    .expect("a use");
    let SessionFact::Used { used } = used else { panic!("a use") };
    assert_eq!((used.kind, used.model.as_deref(), used.by.as_deref()), (LoadoutUseKind::Subagent, Some("opus"), Some("a1")));
}

#[test]
fn a_use_leaves_out_what_it_lacks_and_crosses_under_use() {
    let used = LoadoutUse {
        turn_started_at: Instant::carried("2026-10-17T09:00:00.000Z"),
        kind: LoadoutUseKind::Skill,
        name: "land".into(),
        source: LoadoutSource::Repository,
        from: None,
        model: None,
        by: None,
    };
    let json = encode(&SessionFact::Used { used }).expect("encodes");
    assert_eq!(
        json,
        r#"{"kind":"used","use":{"turn_started_at":"2026-10-17T09:00:00.000Z","kind":"skill","name":"land","source":"repository"}}"#
    );
}

#[test]
fn the_summary_counts_what_is_known_apart_from_what_was_told() {
    let item = |kind| LoadoutItem { kind, name: "x".into(), source: LoadoutSource::Kit, from: None, state: None, tokens: None };
    let items: Vec<LoadoutItem> = [
        LoadoutKind::Skill,
        LoadoutKind::Subagent,
        LoadoutKind::McpServer,
        LoadoutKind::Tool,
        LoadoutKind::MemoryFile,
        LoadoutKind::OutputStyle,
        LoadoutKind::Rules,
        LoadoutKind::Injection,
    ]
    .into_iter()
    .map(item)
    .collect();
    let summary = LoadoutSummary::of(&items, None);
    assert_eq!((summary.knows, summary.told), (5, 3));
    assert_eq!(encode(&summary).expect("encodes"), r#"{"knows":5,"told":3}"#, "no turn is left out, not null");
}

#[test]
fn an_items_body_is_tagged_on_what_and_says_truncated_only_when_it_is() {
    let read = LoadoutItemBody::Read {
        path: Some("skills/land/SKILL.md".into()),
        format: LoadoutItemFormat::Markdown,
        body: "# Land".into(),
        truncated: false,
    };
    assert_eq!(
        encode(&read).expect("encodes"),
        r##"{"what":"read","path":"skills/land/SKILL.md","format":"markdown","body":"# Land"}"##
    );
    let cut = LoadoutItemBody::Read { path: None, format: LoadoutItemFormat::Text, body: String::new(), truncated: true };
    assert_eq!(encode(&cut).expect("encodes"), r#"{"what":"read","format":"text","body":"","truncated":true}"#);
    let not = LoadoutItemBody::NotRead { why: "A server is not a file.".into() };
    assert_eq!(encode(&not).expect("encodes"), r#"{"what":"not_read","why":"A server is not a file."}"#);
}
