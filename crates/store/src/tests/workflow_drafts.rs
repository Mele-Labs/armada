//! The drafts kept to a workflow or a piece: begun by a change, added to in order by whoever made them, read back whole
//! across a reopen, and discarded with their changes.

use crate::tests::{open, TempDir};
use crate::{DraftKey, NewDraftChange, DRAFT_BY_AGENT, DRAFT_BY_YOU};

const AT: &str = "2026-11-13T09:00:00.000Z";
const LATER: &str = "2026-11-13T09:05:00.000Z";

fn key(source: &str) -> DraftKey {
    DraftKey { kind: "workflow".into(), source: source.into(), item_id: "feature".into(), ..Default::default() }
}

fn you<'a>(text: &'a str) -> NewDraftChange<'a> {
    NewDraftChange { made_by: DRAFT_BY_YOU, text, summary: None, change: None }
}

#[test]
fn a_draft_round_trips_with_its_base_and_its_changes_in_order_by_who_made_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.workflow_draft(&key("kit")).unwrap().is_none());
    store.put_draft_change(&key("kit"), "{published}", &you("{one}"), AT).unwrap();
    let agent = NewDraftChange { made_by: DRAFT_BY_AGENT, text: "{two}", summary: Some("split Refine in two"), change: Some(7) };
    let put = store.put_draft_change(&key("kit"), "{ignored: the draft began already}", &agent, LATER).unwrap();
    store.put_draft_change(&key("kit"), "", &you("{three}"), LATER).unwrap();
    drop(store);

    let kept = open(&dir).workflow_draft(&key("kit")).unwrap().expect("kept");
    assert_eq!(kept.base, "{published}", "it keeps what it began from");
    assert_eq!((kept.started_at.as_str(), kept.updated_at.as_str()), (AT, LATER));
    assert_eq!(kept.text(), "{three}", "its text is its last change's");
    let by: Vec<_> = kept.changes.iter().map(|one| (one.made_by.as_str(), one.text.as_str(), one.summary.as_deref(), one.change)).collect();
    assert_eq!(by, [("you", "{one}", None, None), ("agent", "{two}", Some("split Refine in two"), Some(7)), ("you", "{three}", None, None)]);
    assert!(kept.changes[0].id < kept.changes[1].id && kept.changes[1].id < kept.changes[2].id);
    assert_eq!(put.changes.len(), 2, "the answer to a put is the draft as it stood then");
}

#[test]
fn a_draft_is_one_places_and_one_things_and_a_repositorys_is_only_where_the_place_is_the_repository() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let repository = |manifest: &str| DraftKey { manifest_id: manifest.into(), ..key("repository") };
    store.put_draft_change(&repository("one"), "", &you("{one}"), AT).unwrap();
    store.put_draft_change(&repository("two"), "", &you("{two}"), AT).unwrap();
    store.put_draft_change(&key("kit"), "", &you("{kit}"), AT).unwrap();
    let project = |name: &str| DraftKey { project: name.into(), ..key("project") };
    store.put_draft_change(&project("storefront"), "", &you("{storefront}"), AT).unwrap();
    store.put_draft_change(&DraftKey { item_id: "bug".into(), ..key("kit") }, "", &you("{bug}"), AT).unwrap();
    let rule = DraftKey { kind: "rule".into(), scope: "within:web".into(), ..key("kit") };
    store.put_draft_change(&rule, "", &you("{rule}"), AT).unwrap();

    for (wanted, text) in [(repository("one"), "{one}"), (repository("two"), "{two}"), (key("kit"), "{kit}"), (project("storefront"), "{storefront}"), (rule, "{rule}")] {
        assert_eq!(store.workflow_draft(&wanted).unwrap().expect("kept").text(), text);
    }
    assert!(store.workflow_draft(&project("backoffice")).unwrap().is_none());
    assert!(store.workflow_draft(&key("armada")).unwrap().is_none());
}

#[test]
fn discarding_a_draft_deletes_its_changes_and_leaves_the_others_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.put_draft_change(&key("kit"), "{base}", &you("{one}"), AT).unwrap();
    store.put_draft_change(&key("kit"), "{base}", &you("{two}"), LATER).unwrap();
    store.put_draft_change(&key("armada"), "{carried}", &you("{armada}"), AT).unwrap();
    assert!(store.discard_draft(&key("kit")).unwrap());
    assert!(!store.discard_draft(&key("kit")).unwrap(), "there was none to discard");
    assert!(store.workflow_draft(&key("kit")).unwrap().is_none());
    let remaining: i64 = store.conn.query_row("SELECT count(*) FROM workflow_draft_changes", [], |row| row.get(0)).unwrap();
    assert_eq!(remaining, 1, "only the armada draft's change is left");
    let again = store.put_draft_change(&key("kit"), "{new base}", &you("{fresh}"), LATER).unwrap();
    assert_eq!((again.base.as_str(), again.changes.len()), ("{new base}", 1), "a draft begun after a discard begins from what is published then");
}
