//! Requests' file, its read and its writes: a secret's value is in none of them but the one write
//! Bridge sends toward the Keychain.

use crate::{
    decode, encode, DeleteRequestEnvironment, Placed, RequestCollection, RequestPlace, RequestSaved, RequestToSend,
    RequestVariable, Requests, RequestsFile, RequestsNotRead, RunPlace, SaveRequestEnvironment,
    SaveRequestToCollection, SavedRequestNamed, SendRequest, VariableRead, VariableWrite,
};

const FILE: &str = r#"{"version":1,"collection":[{"id":"store","name":"Store API","headers":[{"name":"Accept","value":"application/json"}],"requests":[{"id":"orders","name":"List orders","method":"GET","url":"{{base}}/orders","headers":[{"name":"Authorization","value":"Bearer {{token}}"}]}]}],"environment":[{"name":"staging","variables":[{"name":"base","value":"https://api.example.test"},{"name":"token","secret":true}]}]}"#;

#[test]
fn a_requests_file_reads_and_writes_back_the_same() {
    let file = decode::<RequestsFile>("a requests file", FILE.as_bytes()).expect("reads");
    assert_eq!(file.collections[0].requests[0].name, "List orders");
    assert_eq!(
        file.environments[0].variables,
        vec![
            RequestVariable::Plain { name: "base".into(), value: "https://api.example.test".into() },
            RequestVariable::Secret { name: "token".into() },
        ]
    );
    assert_eq!(encode(&file).expect("plain data"), FILE);
}

#[test]
fn a_file_whose_secret_carries_a_value_is_refused() {
    let leaked = FILE.replace(r#""secret":true"#, r#""secret":true,"value":"hunter22""#);
    let why = decode::<RequestsFile>("a requests file", leaked.as_bytes()).expect_err("refused");
    assert!(why.why.contains("token"), "{}", why.why);
    assert!(!why.why.contains("hunter22"), "{}", why.why);
}

#[test]
fn a_plain_variable_with_no_value_is_refused_rather_than_read_as_empty() {
    let bare = FILE.replace(r#","value":"https://api.example.test""#, "");
    decode::<RequestsFile>("a requests file", bare.as_bytes()).expect_err("refused");
}

#[test]
fn a_secret_is_answered_by_whether_it_is_kept_and_never_by_its_value() {
    let read = vec![
        VariableRead::Plain { name: "base".into(), value: "https://api.example.test".into() },
        VariableRead::Secret { name: "token".into(), set: true },
    ];
    let spelled = encode(&read).expect("plain data");
    assert_eq!(
        spelled,
        r#"[{"name":"base","secret":false,"value":"https://api.example.test"},{"name":"token","secret":true,"set":true}]"#
    );
    assert_eq!(decode::<Vec<VariableRead>>("variables", spelled.as_bytes()).expect("reads"), read);
}

#[test]
fn a_placed_collection_is_the_collection_with_where_it_is_kept() {
    let placed = Placed {
        item: RequestCollection {
            id: "store".into(),
            name: "Store API".into(),
            headers: Vec::new(),
            requests: Vec::new(),
        },
        place: RequestPlace::Yours,
        hides: true,
    };
    let spelled = encode(&placed).expect("plain data");
    assert_eq!(
        spelled,
        r#"{"id":"store","name":"Store API","headers":[],"requests":[],"place":"yours","hides":true}"#
    );
    assert_eq!(decode::<Placed<RequestCollection>>("a collection", spelled.as_bytes()).expect("reads"), placed);
}

#[test]
fn a_file_that_would_not_read_is_named_rather_than_drawn_empty() {
    let read = Requests {
        repository: "/work/shop".into(),
        place: RunPlace::ThisMac,
        collections: Vec::new(),
        environments: Vec::new(),
        not_read: vec![RequestsNotRead { place: RequestPlace::Yours, why: "line 3: expected `=`".into() }],
    };
    let spelled = encode(&read).expect("plain data");
    assert_eq!(
        spelled,
        r#"{"repository":"/work/shop","place":"this_mac","collections":[],"environments":[],"not_read":[{"place":"yours","why":"line 3: expected `=`"}]}"#
    );
    assert_eq!(decode::<Requests>("requests", spelled.as_bytes()).expect("reads"), read);
}

const SAVE_ENVIRONMENT: &str = r#"{"repository":"/work/shop","environment":{"name":"staging","variables":[{"name":"base","secret":false,"value":"https://api.example.test"},{"name":"token","secret":true,"value":"hunter22"},{"name":"cookie","secret":true}]}}"#;

#[test]
fn a_saved_environment_carries_a_secret_toward_fleet_and_never_prints_it() {
    let save = decode::<SaveRequestEnvironment>("an environment", SAVE_ENVIRONMENT.as_bytes()).expect("reads");
    assert_eq!(
        save.environment.variables[1],
        VariableWrite::Secret { name: "token".into(), value: Some("hunter22".into()) }
    );
    // A secret sent with no value keeps the one already kept.
    assert_eq!(save.environment.variables[2], VariableWrite::Secret { name: "cookie".into(), value: None });
    let printed = format!("{save:?} {save:#?}");
    assert!(!printed.contains("hunter22"), "{printed}");
    assert!(printed.contains("token"), "{printed}");
}

#[test]
fn a_plain_variable_written_with_no_value_is_refused() {
    let bare = SAVE_ENVIRONMENT.replace(r#","value":"https://api.example.test""#, "");
    decode::<SaveRequestEnvironment>("an environment", bare.as_bytes()).expect_err("refused");
}

#[test]
fn an_environment_is_deleted_by_name_and_nothing_else() {
    let sent = r#"{"repository":"/work/shop","name":"staging"}"#;
    let delete = decode::<DeleteRequestEnvironment>("an environment to delete", sent.as_bytes()).expect("reads");
    assert_eq!(delete, DeleteRequestEnvironment { repository: Some("/work/shop".into()), session_id: None, name: "staging".into() });
    assert_eq!(encode(&delete).expect("plain data"), sent);
    let with_variables = r#"{"name":"staging","variables":[]}"#;
    decode::<DeleteRequestEnvironment>("an environment to delete", with_variables.as_bytes()).expect_err("refused");
}

#[test]
fn an_agent_saves_a_request_into_a_collection_by_name() {
    let sent = r#"{"collection":"Store API","request":{"name":"Make an order","method":"POST","url":"{{base}}/orders","headers":[],"body":"{}"}}"#;
    let save = decode::<SaveRequestToCollection>("a request to save", sent.as_bytes()).expect("reads");
    assert_eq!(save.collection, "Store API");
    assert_eq!(save.request.body.as_deref(), Some("{}"));
    assert_eq!(encode(&save).expect("plain data"), sent);
    assert_eq!(
        encode(&RequestSaved { collection_id: "store".into(), request_id: "make-an-order".into() }).expect("plain data"),
        r#"{"collection_id":"store","request_id":"make-an-order"}"#
    );
}

#[test]
fn a_send_names_a_saved_request() {
    let sent = r#"{"repository":"/work/shop","saved":{"collection":"store","request":"List orders"},"environment":"staging","headers":[]}"#;
    let send = decode::<SendRequest>("a send", sent.as_bytes()).expect("reads");
    assert_eq!(
        send.request,
        RequestToSend::Saved(SavedRequestNamed { collection: "store".into(), request: "List orders".into() })
    );
    assert_eq!(encode(&send).expect("plain data"), sent);
}

#[test]
fn a_send_carries_its_own_request_with_a_collection_s_headers() {
    let sent = r#"{"session_id":"s1","method":"POST","url":"{{base}}/orders","collection":"store","headers":[{"name":"Authorization","value":"Bearer {{token}}"}],"body":"{}"}"#;
    let send = decode::<SendRequest>("a send", sent.as_bytes()).expect("reads");
    assert_eq!(
        send.request,
        RequestToSend::Own { method: "POST".into(), url: "{{base}}/orders".into(), collection: Some("store".into()) }
    );
    assert_eq!(encode(&send).expect("plain data"), sent);
}

#[test]
fn a_send_naming_both_or_neither_is_refused() {
    let both = r#"{"saved":{"collection":"store","request":"orders"},"method":"GET","url":"https://example.test","headers":[]}"#;
    decode::<SendRequest>("a send", both.as_bytes()).expect_err("both refused");
    let neither = r#"{"environment":"staging","headers":[]}"#;
    decode::<SendRequest>("a send", neither.as_bytes()).expect_err("neither refused");
}

#[test]
fn an_agent_s_saved_request_is_a_row_in_its_thread() {
    let row = crate::SessionRow::Saved {
        id: "r9".into(),
        at: crate::Instant::carried("2026-10-21T09:00:00Z"),
        name: "Make an order".into(),
        collection: "Store API".into(),
        collection_id: "store".into(),
        request: "make-an-order".into(),
    };
    let spelled = encode(&row).expect("plain data");
    assert_eq!(
        spelled,
        r#"{"kind":"saved","id":"r9","at":"2026-10-21T09:00:00Z","name":"Make an order","collection":"Store API","collection_id":"store","request":"make-an-order"}"#
    );
    assert_eq!(decode::<crate::SessionRow>("a row", spelled.as_bytes()).expect("reads"), row);
    assert_eq!(row.id(), "r9");
}

#[test]
fn a_requested_row_carries_no_kept_header() {
    let kept = r#"{"kind":"requested","id":"r1","at":"2026-10-21T09:00:00Z","by":"agent","method":"GET","url":"https://api.example.test/orders","ms":12,"response":"","cookie_host":"api.example.test"}"#;
    let row = decode::<crate::SessionRow>("a row", kept.as_bytes()).expect("an unknown field does not break a parse");
    assert!(!encode(&row).expect("plain data").contains("cookie_host"));
}
