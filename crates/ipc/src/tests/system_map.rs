//! A system map read with a feature lit carries the parts it owns, those it calls and the links between; one from before
//! `lit` reads as unlit.

use crate::{decode, encode, LitFeature, PathStop, SystemLink, SystemMap};

fn level_one(lit: Option<LitFeature>) -> SystemMap {
    SystemMap { project: "p".into(), part: None, feature: None, parts: Vec::new(), links: Vec::new(), features: Vec::new(), stops: Vec::new(), librarian_at: None, lit }
}

#[test]
fn a_lit_feature_crosses_with_what_it_owns_and_what_it_calls() {
    let lit = LitFeature {
        slug: "board".into(),
        title: "Board".into(),
        owned: vec![PathStop { step: 1, part: "bridge".into(), files: vec!["board.rs".into()] }],
        called: vec!["fleet".into()],
        links: vec![SystemLink { from: "bridge".into(), to: "fleet".into(), label: Some("/board".into()), step: None }],
    };
    let json = encode(&level_one(Some(lit.clone()))).expect("plain data");
    assert!(json.contains(r#""lit":{"slug":"board","title":"Board","owned":[{"step":1,"part":"bridge","files":["board.rs"]}],"called":["fleet"],"links":[{"from":"bridge","to":"fleet","label":"/board"}]}"#), "{json}");
    assert_eq!(decode::<SystemMap>("system map", json.as_bytes()).expect("round-trips").lit, Some(lit));
}

#[test]
fn a_map_from_before_lit_reads_unlit_and_an_unlit_one_carries_no_lit() {
    let old = br#"{"project":"p","parts":[],"links":[],"features":[]}"#;
    assert_eq!(decode::<SystemMap>("system map", old).expect("reads"), level_one(None));
    assert!(!encode(&level_one(None)).expect("plain data").contains("lit"));
}
