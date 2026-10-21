//! Requests' file, its read and its writes: a secret's value is in none of them but the one write
//! Bridge sends toward the Keychain.

use crate::{
    decode, encode, Placed, RequestCollection, RequestPlace, RequestVariable, Requests,
    RequestsFile, RequestsNotRead, RunPlace, VariableRead,
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
