//! The builds Fleet has run are kept: one row for each commit `restart-commit` names, when the
//! restart wrote it, whether it was main or the preview, the main commit under it and the branches
//! it holds. What nothing recorded is asked of git once, and stays unknown if git cannot say.

use std::fs;
use std::time::{Duration, SystemTime};

use store::NewBuild;

use super::build_notices::{a_rig, commit_on, git, Rig};

fn written_at(rig: &Rig, seconds: u64) {
    let file = fs::File::options().write(true).open(rig.support().join("restart-commit")).unwrap();
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)).unwrap();
}

async fn builds(rig: &Rig) -> Vec<NewBuild> {
    rig.fleet.store().lock().await.builds().unwrap().into_iter().map(|one| one.build).collect()
}

#[tokio::test]
async fn a_main_build_is_kept_with_its_own_commit_as_the_main_commit_and_when_the_restart_wrote_it() {
    let rig = a_rig();
    rig.on_file(false);
    written_at(&rig, 1_700_000_000);

    rig.fleet.builds_told().await;

    let kept = builds(&rig).await;
    assert_eq!(
        kept,
        [NewBuild {
            commit_id: rig.build.clone(),
            started_at: "2023-11-14T22:13:20.000Z".into(),
            source: Some("main".into()),
            main_commit: Some(rig.build.clone()),
            branches: vec![],
            looked: true,
        }]
    );
}

#[tokio::test]
async fn a_preview_is_kept_with_what_scripts_preview_recorded_it_holds_and_only_for_its_own_commit() {
    let rig = a_rig();
    let base = git(rig.home.path(), &["rev-parse", "main"]);
    rig.on_file(true);
    fs::write(rig.support().join("preview-held"), format!("{}\n{base}\narmada/work\tabc\narmada/other\tdef\n", rig.build)).unwrap();

    rig.fleet.builds_told().await;

    let kept = builds(&rig).await;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].source.as_deref(), Some("preview"));
    assert_eq!(kept[0].main_commit.as_deref(), Some(base.as_str()));
    assert_eq!(kept[0].branches, ["armada/work", "armada/other"]);

    // Another build, for which the file was not written: its contents are not those.
    let next = commit_on(rig.home.path(), "armada/next", &rig.build, "next");
    rig.on_file_at(&next, true);
    rig.fleet.builds_told().await;
    let kept = builds(&rig).await;
    assert_eq!(kept.len(), 2);
    assert_eq!((kept[1].commit_id.as_str(), kept[1].main_commit.clone(), kept[1].branches.len()), (next.as_str(), None, 0));
}

#[tokio::test]
async fn a_restart_onto_the_commit_already_running_is_not_another_build() {
    let rig = a_rig();
    rig.on_file(false);
    rig.fleet.builds_told().await;
    rig.restarts().await;
    rig.on_file(false);
    rig.fleet.builds_told().await;
    assert_eq!(builds(&rig).await.len(), 1);
}

#[tokio::test]
async fn a_build_nothing_recorded_is_read_from_git_and_one_git_cannot_say_stays_unknown() {
    let rig = a_rig();
    let root = rig.home.path();
    let base = git(root, &["rev-parse", "main"]);
    git(root, &["checkout", "-q", "-B", "preview", &base]);
    git(root, &["merge", "-q", "--no-ff", "-m", "Merge branch 'armada/other' into preview", "armada/other"]);
    let preview = git(root, &["rev-parse", "HEAD"]);
    {
        let mut store = rig.fleet.store().lock().await;
        for (commit, at) in [(&preview, "2026-10-14T10:00:00.000Z"), (&rig.build, "2026-10-14T11:00:00.000Z")] {
            let build = NewBuild { commit_id: commit.clone(), started_at: at.into(), ..Default::default() };
            store.keep_build(&build).unwrap();
        }
    }

    rig.fleet.builds_looked_at().await;

    let kept = builds(&rig).await;
    assert_eq!(kept[0].source.as_deref(), Some("preview"));
    assert_eq!(kept[0].main_commit.as_deref(), Some(base.as_str()));
    assert_eq!(kept[0].branches, ["armada/other"]);
    assert!(kept[0].looked);
    assert_eq!((kept[1].source.clone(), kept[1].main_commit.clone(), kept[1].looked), (None, None, false), "a branch's own commit is neither");
}
