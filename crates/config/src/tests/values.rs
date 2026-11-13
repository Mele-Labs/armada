//! The values a Project and Kit set, and what the layers make of them: each key at each layer, the
//! nearest winning, and the layer named.

use std::path::Path;

use core_model::rule::{ActKind, Autonomy, AutonomyDefaults};
use core_model::{AgentStarts, AutoMerge, DeliverySetting, Layer, Look, PrMode, ReviewMode, Reviews, TrackerNamed, Values, WalkReview};

use crate::{
    kit_values_text, parse_kit_values, parse_project, project_text, resolve, shipped_values, AutonomyLayers, Layers, ResolvedValues,
};

fn project_with(block: &str) -> String {
    format!("version: 2\nproject_id: shop\nname: Shop\nmembers: []\n{block}")
}

fn values_of(block: &str) -> Values {
    parse_project(Path::new("shop.yml"), &project_with(block)).expect("a Project that loads").values().clone()
}

fn refused(block: &str) -> String {
    parse_project(Path::new("shop.yml"), &project_with(block)).expect_err("a file that does not load").to_string()
}

#[test]
fn a_project_with_no_values_block_sets_nothing() {
    assert!(values_of("").is_empty());
    assert!(values_of("values: {}\n").is_empty());
}

#[test]
fn a_values_block_holds_only_the_keys_it_sets() {
    let read = values_of("values:\n  pr_mode: draft\n  look:\n    density: roomy\n");
    assert_eq!(
        read,
        Values { pr_mode: Some(PrMode::Draft), look: Look { density: Some("roomy".into()), ..Look::default() }, ..Values::default() }
    );
}

#[test]
fn every_key_reads_and_writes_back() {
    let all = Values {
        pr_mode: Some(PrMode::Ready),
        auto_merge: Some(AutoMerge::ChecksPass),
        review_model: Some("big-model".into()),
        look: Look { layout: Some("compact-deck".into()), theme: Some("paper".into()), density: Some("compact".into()) },
        tracker: Some(TrackerNamed { provider: Some("tickets".into()), keys: [("site".to_string(), "acme".to_string())].into() }),
        reviews: Some(Reviews { plans: ReviewMode::ReviewAndApprove, pull_requests: ReviewMode::Review, walks: WalkReview::Review }),
        agent_starts: Some(AgentStarts::AskMe),
        delivery: Some(DeliverySetting::KeepLocal),
    };
    let project = parse_project(Path::new("shop.yml"), &project_with("")).expect("a Project").with_values(all.clone());
    let written = project_text(&project).expect("text");
    let again = parse_project(Path::new("shop.yml"), &written).expect("what was written reads");
    assert_eq!(again.values(), &all);
}

#[test]
fn a_project_that_sets_nothing_writes_no_block() {
    let project = parse_project(Path::new("shop.yml"), &project_with("")).expect("a Project");
    assert!(!project_text(&project).expect("text").contains("values"));
}

#[test]
fn a_values_block_is_refused_for_every_key_that_is_wrong_and_names_each() {
    let said = refused("values:\n  pr_mode: soon\n  auto_merge: sometimes\n  review_model: ''\n  tracker: x\n  look:\n    density: huge\n    font: big\n");
    for fault in ["values.pr_mode", "values.auto_merge", "values.review_model", "values.tracker", "values.look.density", "values.look.font"] {
        assert!(said.contains(fault), "{fault} is named in: {said}");
    }
}

#[test]
fn a_project_values_block_with_a_tracker_keeps_its_provider_and_every_carried_key() {
    let read = values_of("values:\n  tracker:\n    provider: tickets\n    site: acme\n    board: ABC\n");
    let tracker = read.tracker.expect("a tracker");
    assert_eq!(tracker.provider.as_deref(), Some("tickets"));
    assert_eq!(tracker.keys.get("site").map(String::as_str), Some("acme"));
    assert_eq!(tracker.keys.get("board").map(String::as_str), Some("ABC"));
    assert!(values_of("values:\n  pr_mode: draft\n").tracker.is_none(), "a block that names none sets none");
}

#[test]
fn a_tracker_that_is_not_a_map_or_has_a_key_that_is_not_text_is_refused_at_its_path() {
    assert!(refused("values:\n  tracker: tickets\n").contains("values.tracker"));
    assert!(refused("values:\n  tracker:\n    provider: tickets\n    site: [a, b]\n").contains("values.tracker.site"));
}

#[test]
fn kits_values_have_no_tracker_and_refuse_the_key() {
    let said = parse_kit_values(Path::new("values.yml"), "version: 1\nvalues:\n  tracker:\n    provider: tickets\n").expect_err("refused").to_string();
    assert!(said.contains("values.tracker"), "named in: {said}");
}

#[test]
fn a_project_nearer_than_its_repository_names_the_tracker_and_says_so() {
    let repository = only(|v| v.tracker = Some(TrackerNamed { provider: Some("repo-board".into()), ..TrackerNamed::default() }));
    let project = only(|v| v.tracker = Some(TrackerNamed { provider: Some("shared-board".into()), ..TrackerNamed::default() }));
    let resolved = resolve(&layers(&[(Layer::Repository, repository.clone())]));
    assert_eq!((resolved.tracker.value.and_then(|one| one.provider), resolved.tracker.from), (Some("repo-board".into()), Layer::Repository));
    let resolved = resolve(&layers(&[(Layer::Repository, repository), (Layer::Project, project)]));
    assert_eq!((resolved.tracker.value.and_then(|one| one.provider), resolved.tracker.from), (Some("shared-board".into()), Layer::Project));
    assert_eq!(resolve(&layers(&[])).tracker.from, Layer::Default, "none named is the forge's own");
}

#[test]
fn a_values_block_that_is_not_a_map_is_refused() {
    assert!(refused("values: draft\n").contains("values"));
}

#[test]
fn kit_values_read_the_same_shape_and_an_absent_key_is_not_set() {
    let read = parse_kit_values(Path::new("values.yml"), "version: 1\nvalues:\n  auto_merge: always\n  look:\n    theme: paper\n").expect("Kit's values");
    assert_eq!(read.auto_merge, Some(AutoMerge::Always));
    assert_eq!(read.look.theme.as_deref(), Some("paper"));
    assert_eq!((read.pr_mode, read.review_model.clone()), (None, None));
    assert_eq!(parse_kit_values(Path::new("values.yml"), &kit_values_text(&read).expect("text")).expect("rereads"), read);
}

#[test]
fn kit_values_are_refused_with_their_keys() {
    let said = parse_kit_values(Path::new("values.yml"), "version: 3\nvalues:\n  pr_mode: soon\nmore: 1\n").expect_err("refused").to_string();
    for fault in ["version", "values.pr_mode", "more"] {
        assert!(said.contains(fault), "{fault} is named in: {said}");
    }
    assert!(parse_kit_values(Path::new("values.yml"), "values: {}\n").is_err(), "a file with no version is refused");
}

fn only(set: impl FnOnce(&mut Values)) -> Values {
    let mut values = Values::default();
    set(&mut values);
    values
}

/// A layer setting every key to a word of its own.
fn saying(mode: PrMode, merge: AutoMerge, word: &str) -> Values {
    Values {
        pr_mode: Some(mode),
        auto_merge: Some(merge),
        review_model: Some(format!("model-{word}")),
        look: Look { layout: Some(format!("layout-{word}")), theme: Some(format!("theme-{word}")), density: Some("compact".into()) },
        tracker: None,
        reviews: None,
        agent_starts: None,
        delivery: None,
    }
}

fn layers(of: &[(Layer, Values)]) -> Layers {
    let mut all = Layers { default: shipped_values(), ..Layers::default() };
    for (layer, values) in of {
        match layer {
            Layer::Default => all.default = values.clone(),
            Layer::Machine => all.machine = values.clone(),
            Layer::Kit => all.kit = values.clone(),
            Layer::Repository => all.repository = values.clone(),
            Layer::Project => all.project = values.clone(),
            Layer::Job => all.job = values.clone(),
        }
    }
    all
}

fn from_of(resolved: &ResolvedValues) -> [Layer; 6] {
    [
        resolved.pr_mode.from,
        resolved.auto_merge.from,
        resolved.review_model.from,
        resolved.layout.from,
        resolved.theme.from,
        resolved.density.from,
    ]
}

#[test]
fn with_nothing_set_every_key_is_the_default_and_names_it() {
    let resolved = resolve(&layers(&[]));
    assert_eq!(resolved.pr_mode.value, Some(PrMode::Ready));
    assert_eq!(resolved.auto_merge.value, Some(AutoMerge::Never));
    assert_eq!((resolved.review_model.value.clone(), resolved.layout.value.clone()), (None, None));
    assert_eq!(resolved.theme.value.as_deref(), Some("dark"));
    assert_eq!(resolved.density.value.as_deref(), Some("default"));
    assert_eq!(from_of(&resolved), [Layer::Default; 6]);
}

#[test]
fn every_key_is_read_from_every_layer() {
    for layer in [Layer::Machine, Layer::Kit, Layer::Repository, Layer::Project, Layer::Job] {
        let resolved = resolve(&layers(&[(layer, saying(PrMode::Draft, AutoMerge::Always, "x"))]));
        assert_eq!(resolved.pr_mode.value, Some(PrMode::Draft), "{layer:?}");
        assert_eq!(resolved.auto_merge.value, Some(AutoMerge::Always), "{layer:?}");
        assert_eq!(resolved.review_model.value.as_deref(), Some("model-x"), "{layer:?}");
        assert_eq!(resolved.layout.value.as_deref(), Some("layout-x"), "{layer:?}");
        assert_eq!(resolved.theme.value.as_deref(), Some("theme-x"), "{layer:?}");
        assert_eq!(resolved.density.value.as_deref(), Some("compact"), "{layer:?}");
        assert_eq!(from_of(&resolved), [layer; 6], "{layer:?} is named for every key");
    }
}

#[test]
fn the_nearest_layer_wins_in_the_order_default_machine_kit_repository_project_job() {
    let order = [Layer::Machine, Layer::Kit, Layer::Repository, Layer::Project, Layer::Job];
    for (at, nearer) in order.iter().enumerate() {
        // `nearer` and every layer before it set the key; the nearest of them is the answer.
        let sets: Vec<(Layer, Values)> =
            order[..=at].iter().map(|layer| (*layer, only(|v| v.review_model = Some(format!("{layer:?}"))))).collect();
        let resolved = resolve(&layers(&sets));
        assert_eq!(resolved.review_model.value, Some(format!("{nearer:?}")));
        assert_eq!(resolved.review_model.from, *nearer);
    }
}

#[test]
fn a_key_is_answered_by_the_layer_that_sets_it_and_not_by_one_that_sets_another() {
    let resolved = resolve(&layers(&[
        (Layer::Kit, only(|v| v.pr_mode = Some(PrMode::Draft))),
        (Layer::Project, only(|v| v.auto_merge = Some(AutoMerge::ChecksPass))),
        (Layer::Machine, only(|v| v.look.theme = Some("paper".into()))),
    ]));
    assert_eq!((resolved.pr_mode.value, resolved.pr_mode.from), (Some(PrMode::Draft), Layer::Kit));
    assert_eq!((resolved.auto_merge.value, resolved.auto_merge.from), (Some(AutoMerge::ChecksPass), Layer::Project));
    assert_eq!((resolved.theme.value.as_deref(), resolved.theme.from), (Some("paper"), Layer::Machine));
    assert_eq!(resolved.density.from, Layer::Default);
}

#[test]
fn a_repository_sits_under_the_project_and_over_kit() {
    let repository = only(|v| v.pr_mode = Some(PrMode::Draft));
    let kit = only(|v| v.pr_mode = Some(PrMode::Ready));
    let resolved = resolve(&layers(&[(Layer::Kit, kit.clone()), (Layer::Repository, repository.clone())]));
    assert_eq!((resolved.pr_mode.value, resolved.pr_mode.from), (Some(PrMode::Draft), Layer::Repository));
    let project = only(|v| v.pr_mode = Some(PrMode::Ready));
    let resolved = resolve(&layers(&[(Layer::Kit, kit), (Layer::Repository, repository), (Layer::Project, project)]));
    assert_eq!((resolved.pr_mode.value, resolved.pr_mode.from), (Some(PrMode::Ready), Layer::Project));
}

#[test]
fn a_projects_auto_merge_replaces_the_repositorys_and_is_not_folded_with_it() {
    let restrictive = only(|v| v.auto_merge = Some(AutoMerge::Never));
    let project = only(|v| v.auto_merge = Some(AutoMerge::Always));
    let resolved = resolve(&layers(&[(Layer::Repository, restrictive), (Layer::Project, project)]));
    assert_eq!((resolved.auto_merge.value, resolved.auto_merge.from), (Some(AutoMerge::Always), Layer::Project));
}

#[test]
fn a_repository_that_says_never_beats_kit_and_one_that_says_nothing_does_not() {
    let kit = only(|v| v.auto_merge = Some(AutoMerge::Always));
    let said = resolve(&layers(&[(Layer::Kit, kit.clone()), (Layer::Repository, only(|v| v.auto_merge = Some(AutoMerge::Never)))]));
    assert_eq!((said.auto_merge.value, said.auto_merge.from), (Some(AutoMerge::Never), Layer::Repository));
    let silent = resolve(&layers(&[(Layer::Kit, kit), (Layer::Repository, Values::default())]));
    assert_eq!((silent.auto_merge.value, silent.auto_merge.from), (Some(AutoMerge::Always), Layer::Kit));
}

#[test]
fn autonomy_is_read_for_each_kind_from_the_layers_it_already_has() {
    let kit = AutonomyDefaults::new().setting(ActKind::TrackerWrite, Autonomy::Acts).setting(ActKind::ForgeWrite, Autonomy::Acts);
    let project = AutonomyDefaults::new().setting(ActKind::ForgeWrite, Autonomy::Asks).setting(ActKind::Draft, Autonomy::Off);
    let repository = AutonomyDefaults::new().setting(ActKind::Draft, Autonomy::Asks);
    let mut all = layers(&[]);
    all.autonomy = AutonomyLayers { kit, project, repository };
    let resolved = resolve(&all);
    assert_eq!(resolved.autonomy.len(), ActKind::ALL.len(), "every kind of act is answered");
    let of = |kind: ActKind| resolved.autonomy.iter().find(|one| one.kind == kind).map(|one| (one.value, one.from)).expect("answered");
    assert_eq!(of(ActKind::TrackerWrite), (Autonomy::Acts, Layer::Kit));
    assert_eq!(of(ActKind::ForgeWrite), (Autonomy::Asks, Layer::Project));
    assert_eq!(of(ActKind::Draft), (Autonomy::Asks, Layer::Repository), "a repository's piece is over its Project's, as pieces always were");
    assert_eq!(of(ActKind::StartJob), (ActKind::StartJob.default_autonomy(), Layer::Default));
}

#[test]
fn reviews_read_a_kind_at_a_time_and_a_kind_left_out_is_off() {
    let read = values_of("values:\n  reviews:\n    plans: review_and_approve\n    walks: review\n").reviews.expect("set");
    assert_eq!(read, Reviews { plans: ReviewMode::ReviewAndApprove, pull_requests: ReviewMode::Off, walks: WalkReview::Review });
    assert_eq!(values_of("values:\n  pr_mode: draft\n").reviews, None, "nothing said is nothing set, and Off for every kind");
}

#[test]
fn a_walk_is_refused_review_and_approve_and_a_word_that_is_not_one_is_refused_for_every_kind() {
    let said = refused("values:\n  reviews:\n    walks: review_and_approve\n    plans: sometimes\n");
    for fault in ["values.reviews.walks", "values.reviews.plans"] {
        assert!(said.contains(fault), "{fault} is named in: {said}");
    }
}

#[test]
fn reviews_are_read_from_kit_and_the_nearest_layer_sets_all_three() {
    let kit = parse_kit_values(Path::new("values.yml"), "version: 1\nvalues:\n  reviews:\n    pull_requests: review\n").expect("Kit's values");
    assert_eq!(kit.reviews.map(|reviews| reviews.pull_requests), Some(ReviewMode::Review));
    let project = only(|v| v.reviews = Some(Reviews { plans: ReviewMode::Review, ..Reviews::default() }));
    let resolved = resolve(&layers(&[(Layer::Kit, kit), (Layer::Project, project)]));
    assert_eq!(resolved.reviews.from, Layer::Project);
    assert_eq!(resolved.reviews.value, Some(Reviews { plans: ReviewMode::Review, ..Reviews::default() }), "a project that sets reviews sets all three");
    assert_eq!(resolve(&layers(&[])).reviews.value, None, "no layer is Off");
}

#[test]
fn agent_starts_reads_a_word_refuses_any_other_and_ships_at_once() {
    assert_eq!(values_of("values:\n  agent_starts: ask_me\n").agent_starts, Some(AgentStarts::AskMe));
    assert_eq!(values_of("values:\n  agent_starts: at_once\n").agent_starts, Some(AgentStarts::AtOnce));
    assert_eq!(values_of("values:\n  pr_mode: draft\n").agent_starts, None, "a block that does not name it sets none");
    assert!(refused("values:\n  agent_starts: later\n").contains("values.agent_starts"));
    let kit = parse_kit_values(Path::new("values.yml"), "version: 1\nvalues:\n  agent_starts: ask_me\n").expect("Kit reads it");
    assert_eq!(kit.agent_starts, Some(AgentStarts::AskMe));
    let resolved = resolve(&layers(&[]));
    assert_eq!((resolved.agent_starts.value, resolved.agent_starts.from), (Some(AgentStarts::AtOnce), Layer::Default));
    let nearer = resolve(&layers(&[(Layer::Kit, only(|values| values.agent_starts = Some(AgentStarts::AskMe)))]));
    assert_eq!((nearer.agent_starts.value, nearer.agent_starts.from), (Some(AgentStarts::AskMe), Layer::Kit));
}

#[test]
fn delivery_reads_four_words_refuses_any_other_and_ships_none() {
    for (word, setting) in [
        ("auto_land", DeliverySetting::AutoLand),
        ("open_pr", DeliverySetting::OpenPr),
        ("keep_local", DeliverySetting::KeepLocal),
        ("ask", DeliverySetting::Ask),
    ] {
        assert_eq!(values_of(&format!("values:\n  delivery: {word}\n")).delivery, Some(setting), "{word}");
    }
    assert_eq!(values_of("values:\n  pr_mode: draft\n").delivery, None, "a block that does not name it sets none");
    assert!(refused("values:\n  delivery: whenever\n").contains("values.delivery"));
    let kit = parse_kit_values(Path::new("values.yml"), "version: 1\nvalues:\n  delivery: open_pr\n").expect("Kit reads it");
    assert_eq!(kit.delivery, Some(DeliverySetting::OpenPr));
    let shipped = resolve(&layers(&[]));
    assert_eq!((shipped.delivery.value, shipped.delivery.from), (None, Layer::Default), "unset at every layer is the question");
}

#[test]
fn a_projects_delivery_beats_kits_and_the_answer_names_the_layer() {
    let kit = only(|values| values.delivery = Some(DeliverySetting::KeepLocal));
    let project = only(|values| values.delivery = Some(DeliverySetting::AutoLand));
    let both = resolve(&layers(&[(Layer::Kit, kit.clone()), (Layer::Project, project)]));
    assert_eq!((both.delivery.value, both.delivery.from), (Some(DeliverySetting::AutoLand), Layer::Project));
    let kit_only = resolve(&layers(&[(Layer::Kit, kit)]));
    assert_eq!((kit_only.delivery.value, kit_only.delivery.from), (Some(DeliverySetting::KeepLocal), Layer::Kit));
}
