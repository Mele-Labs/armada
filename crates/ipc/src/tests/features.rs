//! The feature library's bodies, as they are written on the wire.

use crate::{decode, shape, ChangeFeature, FeatureChanged, FeatureSectionKind, FeaturesOfPath, FeaturesSeeded, FileAsk, HandOverFeatures, SeedFeatures, UpsertFeatures, WriteFeatureSections};

#[test]
fn the_two_bodies_say_their_shape() {
    assert_eq!(
        shape::of::<WriteFeatureSections>(),
        "{feature: string, sections?: [{section: test|runs|observe|debt, text: string}], glossary?: [{term: string, meaning: string, not?: [string]}], \
         gists?: [{target: string, gist: string}], project?: string, session_id?: string}"
    );
    assert_eq!(
        shape::of::<UpsertFeatures>(),
        "{project: string, areas?: [{slug: string, title: string}], features?: [{slug: string, area: string, title: string, summary: string, \
         state?: seeded|confirmed|pending, paths?: [string], entry_points?: [string], manifest_id?: string}]}"
    );
}

#[test]
fn the_seed_names_a_project_and_answers_what_it_kept_and_left_out() {
    assert_eq!(shape::of::<SeedFeatures>(), "{project: string}");
    assert_eq!(shape::of::<FeaturesSeeded>(), "{created: [string], areas: number, dropped?: [string]}");
}

#[test]
fn an_ask_filed_outside_a_job_says_its_shape() {
    assert_eq!(
        shape::of::<FileAsk>(),
        "{project: string, kind: bug|change_request|idea|fix, words: string, title?: string, features?: [string], note?: string}"
    );
}

#[test]
fn a_feature_is_merged_or_renamed_and_says_where_it_went() {
    assert_eq!(shape::of::<ChangeFeature>(), "{project: string, feature: string, into?: string, slug?: string, title?: string, area?: string}");
    assert_eq!(shape::of::<FeatureChanged>(), "{feature: string, merged_into?: string}");
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

#[test]
fn the_librarian_names_nine_parts_and_says_where_it_stands() {
    use crate::{FeatureLibrary, LibrarianPart, LibrarianRan, LibrarianState, RunLibrarian};
    assert_eq!(shape::of::<RunLibrarian>(), "{project: string}");
    assert_eq!(
        shape::of::<LibrarianRan>(),
        "{project: string, ran_at: string, ran: [floor_plans|links|decisions|health|drift|retire|glossary|since_you_looked|gists], read_through?: number, skipped_for_share?: [floor_plans|links|decisions|health|drift|retire|glossary|since_you_looked|gists], stopped?: boolean}"
    );
    let state = "{last_run?: string, read_through?: number, merged_since?: [number], steps?: [{id: string, title: string, detail: string, model?: boolean}], running?: {run: string, steps: [{step: string, state: running|done|skipped|failed, said?: string}]}}";
    assert_eq!(shape::of::<LibrarianState>(), state);
    assert!(shape::of::<FeatureLibrary>().contains(&format!("librarian?: {state}")));
    assert_eq!(shape::of::<crate::StopLibrarian>(), "{project: string}");
    assert_eq!(shape::of::<crate::LibrarianStopping>(), "{project: string, stopping?: boolean}");
    assert_eq!(shape::of::<crate::AnswerRetire>(), "{project: string, slug: string, retire: boolean}");
    assert_eq!(shape::of::<crate::RetireAnswered>(), "{project: string, slug: string, retired?: boolean}");
    assert_eq!(shape::of::<crate::RetireProposal>(), "{why: code_gone|unused, evidence: string}");
    let row = shape::of::<crate::FeatureRow>();
    assert!(row.contains("retire?: {why: code_gone|unused, evidence: string}, planned?: boolean}"), "{row}");
    assert_eq!(shape::of::<crate::LibrarianStepMoved>(), "{project: string, run: string, step: string, state: running|done|skipped|failed, said?: string}");
    assert_eq!(LibrarianPart::ALL.len(), 9);
    for part in LibrarianPart::ALL {
        assert_eq!(LibrarianPart::from_name(part.name()), Some(part), "{} reads back by its name", part.name());
    }
    let share: Vec<&str> = LibrarianPart::ALL.into_iter().filter(|one| one.spends_model_share()).map(LibrarianPart::name).collect();
    assert_eq!(share, ["drift", "since_you_looked", "gists"], "the model-written parts are the ones a night's share bounds");
}

#[test]
fn a_check_of_the_docs_says_its_shape_and_a_doc_carries_what_it_found() {
    let check = "{at: string, disagrees?: {doc_quote: string, code_at: string, code_quote: string}, dismissed?: boolean}";
    assert_eq!(shape::of::<crate::DocCheck>(), check);
    assert_eq!(shape::of::<crate::CheckDocs>(), "{project: string, slug: string}");
    assert_eq!(shape::of::<crate::DocsChecked>(), format!("{{project: string, slug: string, checked: [{{target: string, check: {check}}}], unchecked?: [string], calls: number}}"));
    assert_eq!(shape::of::<crate::DocFlag>(), "{project: string, slug: string, target: string}");
    assert_eq!(shape::of::<crate::DocJobStarted>(), "{project: string, slug: string, target: string, session_id: string, title: string, brief: string, send_failed?: string}");
    let read = shape::of::<crate::FeatureRead>();
    assert!(read.ends_with(&format!("drifted?: [string], check?: {check}}}")), "{read}");
}

#[test]
fn a_library_start_says_its_shape_and_a_draft_names_what_it_kept_and_passed_over() {
    assert_eq!(shape::of::<crate::LibraryStart>(), "{project: string, modules: [{name: string, slug: string, path: string, held?: boolean}], docs: [string]}");
    assert_eq!(shape::of::<crate::DraftModules>(), "{project: string}");
    assert_eq!(shape::of::<crate::ModulesDrafted>(), "{project: string, drafted: [string], held?: [string]}");
}

#[test]
fn a_sub_project_says_its_shape() {
    assert_eq!(
        shape::of::<crate::SubProject>(),
        "{id: string, project: string, name: string, goal: string, jobs: [string], features: [string], orchestrator?: string, started_at: string, ended_at?: string}"
    );
    assert_eq!(shape::of::<crate::StartSubProject>(), "{project: string, name: string, goal: string, jobs: [string]}");
    assert_eq!(shape::of::<crate::TurnIntoOrchestrator>(), "{project: string, session_id: string, goal: string}");
    assert_eq!(shape::of::<crate::UngroupSubProject>(), "{id: string}");
    assert_eq!(shape::of::<crate::GoalMet>(), "{sub_project: string, goal: string, landed?: [{number: number, title: string}]}");
}

#[test]
fn an_area_change_a_reject_and_a_catalog_rule_say_their_shape() {
    assert_eq!(shape::of::<crate::ChangeArea>(), "{project: string, area: string, change: {act: make|rename|split|merge, ...}, why?: string}");
    let lists = "features?: [string], areas?: [string], paths?: [string]";
    assert_eq!(shape::of::<crate::RejectInCatalog>(), format!("{{project: string, reject: not_this_feature|not_together, {lists}, why?: string}}"));
    let acts = "make_area|rename_area|split_area|merge_areas|merge_features|rename_feature|move_feature|not_this_feature|not_together";
    let rule = format!("{{id: string, text: string, act: {acts}, {lists}, at: string}}");
    assert_eq!(shape::of::<crate::CatalogRules>(), format!("{{project: string, rules: [{rule}]}}"));
    assert_eq!(shape::of::<crate::AreaChanged>(), "{project: string, area: string, rule?: string}");
    assert_eq!(shape::of::<crate::DropCatalogRule>(), "{project: string, rule: string}");
    assert!(shape::of::<crate::ChangeFeature>().ends_with("area?: string, why?: string}"));
    assert_eq!(shape::of::<crate::FeatureChanged>(), "{feature: string, merged_into?: string, rule?: string}");
}
