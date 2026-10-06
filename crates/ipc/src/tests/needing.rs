//! A need rides on `declare_scope` and on a plan's tasks, in the declarer's
//! words, and a call that is not one is refused by name. #1059.

use core_model::PlanChange;

use crate::mcp::{read, DeclareScope, Incoming, NeedClaim, NotAnArgument, PlanCall};

fn called(tool: &str, arguments: &str) -> Incoming {
    let body = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    );
    read(body.as_bytes())
}

fn claim(path: &str, what: &str, took: Option<&str>) -> NeedClaim {
    NeedClaim {
        path: path.to_string(),
        what: what.to_string(),
        took: took.map(str::to_string),
    }
}

#[test]
fn a_scope_call_carries_the_needs_beside_its_paths() {
    let Incoming::Declare {
        declaration: DeclareScope {
            context_paths,
            needs,
        },
        ..
    } = called(
        "declare_scope",
        r#"{"context_paths":["crates/store"],"needs":[
            {"path":"crates/store/src/migrations.rs","what":"a new migration","took":"V95"},
            {"path":"protocol-version.toml","what":"a minor"}]}"#,
    )
    else {
        panic!("a declaration");
    };
    assert_eq!(context_paths, ["crates/store"]);
    assert_eq!(
        needs,
        [
            claim(
                "crates/store/src/migrations.rs",
                "a new migration",
                Some("V95")
            ),
            claim("protocol-version.toml", "a minor", None),
        ]
    );
}

#[test]
fn a_scope_call_without_needs_declares_none() {
    let Incoming::Declare { declaration, .. } = called("declare_scope", r#"{"context_paths":[]}"#)
    else {
        panic!("a declaration");
    };
    assert!(declaration.needs.is_empty());
}

#[test]
fn a_need_with_no_words_or_an_unknown_field_is_refused_by_name() {
    let blank = called(
        "declare_scope",
        r#"{"context_paths":[],"needs":[{"path":"a.rs","what":"  "}]}"#,
    );
    assert!(matches!(
        blank,
        Incoming::NotASubmission {
            why: NotAnArgument::Blank { field: "what" },
            ..
        }
    ));
    let extra = called(
        "declare_scope",
        r#"{"context_paths":[],"needs":[{"path":"a.rs","what":"x","kind":"migration"}]}"#,
    );
    assert!(matches!(
        extra,
        Incoming::NotASubmission {
            why: NotAnArgument::NotAField { .. },
            ..
        }
    ));
    let not_a_list = called("declare_scope", r#"{"context_paths":[],"needs":"a.rs"}"#);
    assert!(matches!(
        not_a_list,
        Incoming::NotASubmission {
            why: NotAnArgument::NotOptions { field: "needs" },
            ..
        }
    ));
}

#[test]
fn a_plans_tasks_carry_their_needs_with_the_call() {
    let Incoming::Plan {
        call: PlanCall { change, needs, .. },
        ..
    } = called(
        "record_plan",
        r#"{"approach":"Add a column","tasks":[
            {"title":"Migrate","note":"","scope":[],"expects":"",
             "needs":[{"path":"crates/store/src/migrations.rs","what":"a new migration"}]},
            {"title":"Cover","note":"","scope":[],"expects":""}]}"#,
    )
    else {
        panic!("a plan call");
    };
    assert!(matches!(change, PlanChange::Recorded { .. }));
    assert_eq!(
        needs,
        [claim(
            "crates/store/src/migrations.rs",
            "a new migration",
            None
        )]
    );
}
