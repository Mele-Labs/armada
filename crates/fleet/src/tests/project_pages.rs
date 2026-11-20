//! The project overview and Since you looked, written before the code: **the overview reads the first README sentence,
//! the code map with each room's area and the busiest features of the last 14 days, and Since you looked tags each feature
//! new, changed or needs_you, with no model.** Each test keeps a library in a real store and reads it as Fleet does.
//! `docs/concepts/feature-catalog.md`, *Project overview* and *Since you looked*.

use std::path::Path;

use api::{Features, Projects};
use ipc::{LookedTag, NewProject, ProjectMember};
use store::{feature_link_kind, KeptFeature, KeptFeatureLink};

use super::agent_starts::{rig, Rig};
use crate::feature_library::first_sentence;

const DAY_MS: i64 = 86_400_000;

fn put(root: &Path, path: &str, text: &str) {
    let at = root.join(path);
    std::fs::create_dir_all(at.parent().expect("a parent")).expect("made");
    std::fs::write(at, text).expect("written");
}

async fn project(rig: &Rig) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    rig.fleet.add_project(NewProject { name: "Shop".to_string(), members, place: None, values: None }).await.expect("added").projects.last().expect("the project").id.clone()
}

fn at(rig: &Rig, days_ago: i64) -> String {
    crate::clock::rfc3339_utc(rig.fleet.now().epoch_millis().expect("a clock") - days_ago * DAY_MS)
}

/// A feature of `area`, added `days_ago` days ago, in `state`.
async fn kept(rig: &Rig, project: &str, slug: &str, area: &str, paths: &[&str], state: &str, days_ago: i64) -> String {
    let feature = KeptFeature {
        id: format!("id-{slug}"),
        project: project.to_string(),
        area: area.to_string(),
        slug: slug.to_string(),
        title: format!("Title of {slug}"),
        summary: format!("Summary of {slug}."),
        state: state.to_string(),
        manifest_id: String::new(),
        entry_points: Vec::new(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        position: 0,
        added_at: at(rig, days_ago),
    };
    rig.fleet.store().lock().await.put_feature(&feature).expect("kept")
}

/// A pull request merged on the feature `days_ago` days ago, titled `title`.
async fn merged(rig: &Rig, feature: &str, number: u64, title: &str, days_ago: i64) {
    let link = KeptFeatureLink {
        feature_id: feature.to_string(),
        kind: feature_link_kind::MERGE.to_string(),
        target: number.to_string(),
        job: String::new(),
        title: title.to_string(),
        gist: String::new(),
        gist_by: String::new(),
        state: String::new(),
        via: "detector".to_string(),
        at: at(rig, days_ago),
    };
    rig.fleet.store().lock().await.put_feature_link(&link).expect("linked");
}

#[test]
fn the_first_sentence_is_the_first_prose_past_the_title_badges_and_markup() {
    let readme = "# Armada\n\n[![ci](https://x/y.svg)](https://x/y)\n![logo](logo.png)\n\nArmada is a **macOS app** that sends [coding agents](https://a.b/c) at `git` work, e.g. a bug.\nIt checks them. Then more.\n\n## Install\n";
    assert_eq!(first_sentence(readme).as_deref(), Some("Armada is a macOS app that sends coding agents at git work, e.g. a bug."), "markup off, `e.g.` is no end");
    assert_eq!(first_sentence("Armada\n======\n\nOne line here\nover two. Next.").as_deref(), Some("One line here over two."), "an underlined title is no prose, and lines join");
    assert_eq!(first_sentence("---\ntitle: x\n---\n\n```sh\nmake\n```\n\n- a list\n\nReal words? Yes.").as_deref(), Some("Real words?"), "front matter, code and lists are skipped");
    assert_eq!(first_sentence("# Only a title\n\n![x](y.png)\n"), None);
    assert!(first_sentence(&format!("{} end.", "word ".repeat(100))).is_some_and(|one| one.chars().count() <= 301 && one.ends_with('…')), "a sentence that runs on is cut");
}

#[tokio::test]
async fn the_overview_reads_the_readmes_first_sentence_with_its_file_and_the_map_with_each_rooms_area() {
    let rig = rig(None);
    let root = rig.fleet.repositories().first().expect("served").root().to_string();
    put(Path::new(&root), "README.md", "# Shop\n\nShop sells things to people. It is small.\n");
    put(Path::new(&root), "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    put(Path::new(&root), "crates/cart/Cargo.toml", "[package]\nname = \"cart\"\n");
    put(Path::new(&root), "crates/cart/src/lib.rs", "");
    put(Path::new(&root), "crates/cart/src/more.rs", "");
    put(Path::new(&root), "crates/pay/Cargo.toml", "[package]\nname = \"pay\"\n");
    put(Path::new(&root), "crates/pay/src/lib.rs", "");
    put(Path::new(&root), "crates/misc/Cargo.toml", "[package]\nname = \"misc\"\n");
    let project = project(&rig).await;
    kept(&rig, &project, "carts", "buying", &["crates/cart"], "confirmed", 30).await;
    kept(&rig, &project, "paying", "money", &["crates/pay/src"], "confirmed", 30).await;

    let overview = rig.fleet.get_project_overview(None, Some(project.clone())).await.expect("overview");
    let what = overview.what_it_is.expect("a README");
    assert_eq!((what.sentence.as_str(), what.file.as_str()), ("Shop sells things to people.", "README.md"));
    let rooms: Vec<(&str, Option<&str>)> = overview.rooms.iter().map(|room| (room.module.as_str(), room.area.as_deref())).collect();
    assert_eq!(rooms, [("cart", Some("buying")), ("misc", None), ("pay", Some("money"))], "every room, with the area owning most of it or none");
    assert!(overview.glossary.is_empty());
}

#[tokio::test]
async fn busiest_counts_only_the_last_fourteen_days_most_merges_first_and_five_at_most() {
    let rig = rig(None);
    let project = project(&rig).await;
    let mut ids = Vec::new();
    for slug in ["a", "b", "c", "d", "e", "f", "old"] {
        ids.push(kept(&rig, &project, slug, "x", &[], "confirmed", 60).await);
    }
    // `a` has three merges in the window and a fourth out of it; `b` two; `c` to `f` one each, `f` the newest.
    for (feature, number, days) in [(0, 1, 2), (0, 2, 3), (0, 3, 5), (0, 4, 20), (1, 5, 1), (1, 6, 9), (2, 7, 8), (3, 8, 7), (4, 9, 6), (5, 10, 0)] {
        merged(&rig, &ids[feature], number, "t", days).await;
    }
    merged(&rig, &ids[6], 11, "t", 15).await;

    let overview = rig.fleet.get_project_overview(None, Some(project)).await.expect("overview");
    let busiest: Vec<(&str, Vec<u64>)> = overview.busiest.iter().map(|one| (one.slug.as_str(), one.pulls.clone())).collect();
    assert_eq!(busiest, [("a", vec![1, 2, 3]), ("b", vec![5, 6]), ("f", vec![10]), ("e", vec![9]), ("d", vec![8])], "the old merge counts for nothing, ties go to the newest, and five is the cap");
}

#[tokio::test]
async fn since_you_looked_tags_each_feature_and_orders_needs_you_then_changed_by_newest_merge_then_new() {
    let rig = rig(None);
    let project = project(&rig).await;
    let since = at(&rig, 7);
    let old_a = kept(&rig, &project, "old-a", "x", &[], "confirmed", 60).await;
    let old_b = kept(&rig, &project, "old-b", "x", &[], "confirmed", 60).await;
    let still = kept(&rig, &project, "still", "x", &[], "confirmed", 60).await;
    kept(&rig, &project, "fresh", "x", &[], "confirmed", 2).await;
    kept(&rig, &project, "fresher", "x", &[], "seeded", 1).await;
    let pending = kept(&rig, &project, "asked-for", "pending", &[], "pending", 40).await;
    merged(&rig, &old_a, 20, "Add a thing", 5).await;
    merged(&rig, &old_a, 21, "Fix the thing", 4).await;
    merged(&rig, &old_b, 22, "Retitle it", 2).await;
    merged(&rig, &still, 23, "Long ago", 30).await;
    merged(&rig, &pending, 24, "Named by a job", 3).await;

    let looked = rig.fleet.get_since_you_looked(None, Some(project), since).await.expect("read");
    assert_eq!(looked.paragraph, "");
    let rows: Vec<(LookedTag, &str, &str, Vec<u64>)> = looked.features.iter().map(|one| (one.tag, one.slug.as_str(), one.what.as_str(), one.pulls.clone())).collect();
    assert_eq!(
        rows,
        [
            (LookedTag::NeedsYou, "asked-for", "Named by a job", vec![24]),
            (LookedTag::Changed, "old-b", "Retitle it", vec![22]),
            (LookedTag::Changed, "old-a", "Fix the thing; Add a thing", vec![21, 20]),
            (LookedTag::New, "fresher", "Summary of fresher.", vec![]),
            (LookedTag::New, "fresh", "Summary of fresh.", vec![]),
        ],
        "a feature nothing touched is left out, and a feature with no title to read says what it is"
    );
}

#[tokio::test]
async fn a_project_with_nothing_new_answers_empty_lists_and_a_time_that_is_none_is_refused() {
    let rig = rig(None);
    let project = project(&rig).await;
    let old = kept(&rig, &project, "quiet", "x", &[], "confirmed", 60).await;
    merged(&rig, &old, 30, "Long ago", 40).await;

    let looked = rig.fleet.get_since_you_looked(None, Some(project.clone()), at(&rig, 7)).await.expect("read");
    assert!(looked.features.is_empty());
    let overview = rig.fleet.get_project_overview(None, Some(project.clone())).await.expect("overview");
    assert_eq!((overview.what_it_is, overview.busiest, overview.glossary), (None, Vec::new(), Vec::new()), "no README, no merge in the window");

    let refused = rig.fleet.get_since_you_looked(None, Some(project), "yesterday".to_string()).await.expect_err("refused");
    assert_eq!((refused.status(), refused.error().code.as_str()), (422, "fleet.feature_since_unreadable"));
}
