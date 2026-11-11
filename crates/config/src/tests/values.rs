//! The values a Project and Kit set, and what the layers make of them: each key at each layer, the
//! nearest winning, and the layer named.

use std::path::Path;

use core_model::rule::{ActKind, Autonomy, AutonomyDefaults};
use core_model::{AutoMerge, Layer, Look, PrMode, Values};

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
