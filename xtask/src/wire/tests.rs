use super::emit::{render, Registry};
use super::{lex, parse};

/// The TypeScript for the closure of `roots` over one source file named `x`.
fn ts(src: &str, roots: &[&str]) -> Result<String, String> {
    let items = parse::items(lex::lex(src)?, "x")?;
    let reg = Registry { items };
    let closure = reg.closure(roots)?;
    let files = render(&reg, &closure)?;
    Ok(files.into_iter().map(|(_, text)| text).collect())
}

fn body(text: &str) -> String {
    text.lines().skip(2).collect::<Vec<_>>().join("\n")
}

#[test]
fn a_field_is_optional_where_serde_can_leave_it_out() {
    let out = ts(
        r#"
        #[derive(Serialize, Deserialize)]
        pub struct Row {
            pub id: String,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub note: Option<String>,
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            pub tags: Vec<String>,
            #[serde(default)]
            pub band: u32,
            pub stated: Option<u32>,
        }
        "#,
        &["Row"],
    )
    .unwrap();
    assert_eq!(
        body(&out),
        "export type Row = {\n  id: string;\n  note?: string;\n  tags?: string[];\n  band: number;\n  stated: number | null;\n};"
    );
}

#[test]
fn rename_all_and_rename_name_the_keys_the_wire_has() {
    let out = ts(
        r#"
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct A {
            pub first_name: String,
            #[serde(rename = "last-name")]
            pub last: String,
        }
        "#,
        &["A"],
    )
    .unwrap();
    assert!(out.contains("  firstName: string;"));
    assert!(out.contains("  \"last-name\": string;"));
}

#[test]
fn an_internally_tagged_enum_carries_its_tag_in_each_variant() {
    let out = ts(
        r#"
        #[derive(Serialize)]
        #[serde(tag = "kind", rename_all = "snake_case")]
        pub enum Said {
            /// Pressed.
            Press { artifact: String },
            NotRead,
            Wrapped(Inner),
        }
        #[derive(Serialize)]
        pub struct Inner { pub n: u8 }
        "#,
        &["Said"],
    )
    .unwrap();
    assert!(out.contains("  /** Pressed. */\n  | {\n      kind: \"press\";\n      artifact: string;\n    }"), "{out}");
    assert!(out.contains("  | {\n      kind: \"not_read\";\n    }"), "{out}");
    assert!(out.contains("  | {\n      kind: \"wrapped\";\n    } & Inner"), "{out}");
}

#[test]
fn an_externally_tagged_enum_is_a_string_or_a_one_key_object() {
    let out = ts(
        r#"
        #[derive(Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum Answer {
            AllowOnce,
            Chosen(Vec<String>),
            Said { words: String },
        }
        "#,
        &["Answer"],
    )
    .unwrap();
    assert!(out.contains("| \"allow_once\""), "{out}");
    assert!(out.contains("chosen: string[];"), "{out}");
    assert!(out.contains("said: {\n"), "{out}");
}

#[test]
fn a_flattened_field_is_an_intersection_and_a_generic_keeps_its_parameter() {
    let out = ts(
        r#"
        #[derive(Serialize)]
        pub struct Wrapped<T> {
            pub armada: u8,
            #[serde(flatten)]
            pub message: T,
        }
        "#,
        &["Wrapped"],
    )
    .unwrap();
    assert!(out.contains("export type Wrapped<T> = {\n  armada: number;\n} & T;"), "{out}");
}

#[test]
fn the_macros_that_spell_a_string_are_strings() {
    let out = ts(
        r#"
        wire_enum! {
            /// What it is.
            Kind, core_model::Kind, "a kind"
        }
        #[derive(Serialize)]
        #[serde(transparent)]
        pub struct Id(String);
        #[derive(Serialize)]
        pub struct Row { pub kind: Kind, pub id: Id }
        "#,
        &["Row"],
    )
    .unwrap();
    assert!(out.contains("export type Kind = string;"), "{out}");
    assert!(out.contains("export type Id = string;"), "{out}");
    assert!(out.contains("/** What it is. */"), "{out}");
}

#[test]
fn a_map_a_tuple_and_a_json_value_have_their_own_spellings() {
    let out = ts(
        r#"
        #[derive(Serialize)]
        pub struct Row {
            pub by: BTreeMap<String, Vec<u32>>,
            pub pair: (String, bool),
            pub raw: serde_json::Value,
            pub maybe: Vec<Option<String>>,
        }
        "#,
        &["Row"],
    )
    .unwrap();
    assert!(out.contains("by: Record<string, number[]>;"), "{out}");
    assert!(out.contains("pair: [string, boolean];"), "{out}");
    assert!(out.contains("raw: unknown;"), "{out}");
    assert!(out.contains("maybe: (string | null)[];"), "{out}");
}

#[test]
fn what_it_cannot_read_it_names() {
    let missing = ts("#[derive(Serialize)] pub struct A { pub b: Elsewhere }", &["A"]).unwrap_err();
    assert!(missing.contains("`Elsewhere`"), "{missing}");
    let by_hand = ts("pub struct A { pub n: u8 }", &["A"]).unwrap_err();
    assert!(by_hand.contains("no `Serialize` derive"), "{by_hand}");
    let rule = ts("#[derive(Serialize)] #[serde(rename_all = \"Title Case\")] pub struct A { pub n_x: u8 }", &["A"]).unwrap_err();
    assert!(rule.contains("Title Case"), "{rule}");
    let root = ts("", &["Nothing"]).unwrap_err();
    assert!(root.contains("`Nothing`"), "{root}");
}

#[test]
fn code_that_is_not_a_definition_is_skipped_and_a_test_module_is_not_read() {
    let out = ts(
        r#"
        use std::fmt;
        const N: Foo = Foo { a: 1 };
        fn is_false(v: &bool) -> bool { !*v }
        impl A { pub fn new() -> A where X: Y { A { n: 1 } } }
        #[cfg(test)]
        mod tests { struct Hidden; }
        #[derive(Serialize)]
        pub struct A { pub n: u8, #[serde(skip)] pub hidden: Hidden }
        "#,
        &["A"],
    )
    .unwrap();
    assert!(out.contains("  n: number;"), "{out}");
    assert!(!out.contains("hidden"), "{out}");
}

/// What is checked in is what `crates/ipc` generates now: the check `verify-foundations` runs, run here.
#[test]
fn the_checked_in_wire_types_are_current() {
    let report = super::pockets_wire_types_are_generated_from_ipc(&crate::repo_root());
    let failed: Vec<String> = report
        .findings
        .iter()
        .filter_map(|f| match f {
            crate::Finding::Fail(why) => Some(why.clone()),
            crate::Finding::Warn(_) => None,
        })
        .collect();
    assert!(failed.is_empty(), "{failed:#?}");
}
