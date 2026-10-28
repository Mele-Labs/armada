//! Watch recipes are read from the base, not from what is on disk.

use crate::tests::repo::TempRepo;
use crate::watch_recipes_on_base;

#[test]
fn watch_recipes_are_the_bases_and_not_the_working_trees() {
    let repo = TempRepo::empty();
    let watches = repo.root().join(".armada/watches");
    std::fs::create_dir_all(&watches).expect("a folder");
    std::fs::write(watches.join("qa.yml"), "name: qa-deploy\n").expect("a file");
    repo.commit_everything("the first commit");
    std::fs::write(watches.join("qa.yml"), "name: edited\n").expect("an edit");

    let found = watch_recipes_on_base(repo.root(), None);

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "name: qa-deploy\n");
    assert!(found[0].0.ends_with(".armada/watches/qa.yml"));
}
