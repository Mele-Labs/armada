//! The feature library's bodies, as they are written on the wire.

use crate::{decode, shape, FeatureSectionKind, FeaturesOfPath, HandOverFeatures, UpsertFeatures, WriteFeatureSections};

#[test]
fn the_two_bodies_say_their_shape() {
    assert_eq!(
        shape::of::<WriteFeatureSections>(),
        "{feature: string, sections: [{section: test|runs|observe|debt, text: string}], project?: string, session_id?: string}"
    );
    assert_eq!(
        shape::of::<UpsertFeatures>(),
        "{project: string, areas?: [{slug: string, title: string}], features?: [{slug: string, area: string, title: string, summary: string, \
         state?: seeded|confirmed, paths?: [string], entry_points?: [string], manifest_id?: string}]}"
    );
}

#[test]
fn a_section_is_written_by_its_name_and_a_wrong_name_is_refused() {
    let write: WriteFeatureSections =
        decode("sections", br#"{"feature":"voting-on-takes","sections":[{"section":"debt","text":"One vote kind is untested."}]}"#).expect("decoded");
    assert_eq!(write.sections[0].section, FeatureSectionKind::Debt);
    assert!(decode::<WriteFeatureSections>("sections", br#"{"feature":"x","sections":[{"section":"tests","text":"t"}]}"#).is_err());
}

#[test]
fn a_path_answers_each_feature_with_how_it_belongs() {
    let found: FeaturesOfPath = decode(
        "features of a path",
        br#"{"path":"a.rs","features":[{"project":"p1","slug":"s","title":"S","area":"A","how":"prefix","prefix":"a"},{"project":"p1","slug":"t","title":"T","area":"A","how":"filed"}]}"#,
    )
    .expect("decoded");
    assert_eq!(found.features.len(), 2);
    assert!(found.features[1].prefix.is_none());
}

#[test]
fn the_hand_over_body_says_its_shape() {
    assert_eq!(shape::of::<HandOverFeatures>(), "{session_id: string, agent_id?: string, paths?: [string], brief?: string}");
}

#[test]
fn the_commit_check_body_says_its_shape() {
    assert_eq!(shape::of::<crate::CheckCommitFeature>(), "{session_id: string, command: string, files?: [string]}");
}

#[test]
fn an_act_names_its_feature_as_one_slug_or_a_list() {
    use crate::FeatureNames;
    let one: FeatureNames = decode("a feature", br#""voting-on-takes""#).expect("decoded");
    assert_eq!(one.slugs(), ["voting-on-takes"]);
    let many: FeatureNames = decode("features", br#"[" Voting-On-Takes","page-actions, voting-on-takes",""]"#).expect("decoded");
    assert_eq!(many.slugs(), ["voting-on-takes", "page-actions"], "trimmed, lower case, split at a comma and each once");
    assert_eq!(many.as_detail().as_deref(), Some("voting-on-takes,page-actions"));
    assert_eq!(FeatureNames::One(" ".into()).as_detail(), None);
    let shown: crate::ShowWindow = decode("a window", br#"{"url":"http://localhost:5173/","feature":["a"]}"#).expect("decoded");
    assert_eq!(shown.feature.map(|names| names.slugs()), Some(vec!["a".to_string()]));
}
