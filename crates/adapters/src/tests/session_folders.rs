//! The folders a person's sessions name: read from the record's own lines, with whether each is
//! still there and whether it is a repository. Planted files, no process.

use std::fs::File;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use super::scratch::Scratch;
use crate::session_folders::{directory_name, list_folders, FolderGit};

/// A record with one line naming `cwd`, written under the projects directory.
fn kept(projects: &Path, dir: &str, session: &str, cwd: &str) {
    let line = format!("{{\"type\":\"user\",\"cwd\":\"{cwd}\",\"message\":{{\"content\":\"hi\"}}}}\n");
    let path = projects.join(dir).join(format!("{session}.jsonl"));
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("the directory");
    std::fs::write(path, line).expect("the record");
}

fn written_at(path: &Path, seconds: u64) {
    File::options()
        .write(true)
        .open(path)
        .expect("the record")
        .set_modified(UNIX_EPOCH + Duration::from_secs(seconds))
        .expect("its time");
}

fn text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[test]
fn a_folder_is_the_one_the_lines_name_and_not_a_guess_from_the_directory() {
    let scratch = Scratch::new();
    let app = scratch.folder("work/my-app");
    // The directory's name could be `work/my/app` as well; the line settles it.
    kept(&scratch.folder("projects"), &directory_name(&text(&app)), "a", &text(&app));

    let found = list_folders(&scratch.path().join("projects"));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, text(&app));
    assert!(found[0].exists);
}

#[test]
fn a_folder_that_is_gone_is_listed_as_gone_with_no_git_state() {
    let scratch = Scratch::new();
    let gone = scratch.path().join("deleted/project");
    kept(&scratch.folder("projects"), &directory_name(&text(&gone)), "a", &text(&gone));

    let found = list_folders(&scratch.path().join("projects"));

    assert_eq!(found.len(), 1);
    assert!(!found[0].exists);
    assert_eq!(found[0].git, FolderGit::Unknown);
}

#[test]
fn git_state_is_a_root_inside_one_or_not_a_repository() {
    let scratch = Scratch::new();
    let root = scratch.folder("repo");
    scratch.folder("repo/.git");
    let inside = scratch.folder("repo/apps/web");
    let plain = scratch.folder("notes");
    let projects = scratch.folder("projects");
    for folder in [&root, &inside, &plain] {
        kept(&projects, &directory_name(&text(folder)), "a", &text(folder));
    }

    let found = list_folders(&projects);
    let git = |folder: &Path| found.iter().find(|one| one.path == text(folder)).expect("listed").git.clone();

    assert_eq!(git(&root), FolderGit::Root);
    assert_eq!(git(&inside), FolderGit::Inside(text(&root)));
    assert_eq!(git(&plain), FolderGit::NotARepository);
    let one_inside = found.iter().find(|one| one.path == text(&inside)).expect("listed");
    assert_eq!(one_inside.root(), text(&root));
}

#[test]
fn folders_come_newest_session_first_with_the_count_of_sessions() {
    let scratch = Scratch::new();
    let older = scratch.folder("older");
    let newer = scratch.folder("newer");
    let projects = scratch.folder("projects");
    kept(&projects, &directory_name(&text(&older)), "a", &text(&older));
    kept(&projects, &directory_name(&text(&newer)), "a", &text(&newer));
    kept(&projects, &directory_name(&text(&newer)), "b", &text(&newer));
    written_at(&projects.join(directory_name(&text(&older))).join("a.jsonl"), 1_000);
    written_at(&projects.join(directory_name(&text(&newer))).join("a.jsonl"), 2_000);
    written_at(&projects.join(directory_name(&text(&newer))).join("b.jsonl"), 3_000);

    let found = list_folders(&projects);

    assert_eq!(found.iter().map(|one| one.path.clone()).collect::<Vec<_>>(), [text(&newer), text(&older)]);
    assert_eq!((found[0].newest_ms, found[0].sessions), (3_000_000, 2));
    assert_eq!((found[1].newest_ms, found[1].sessions), (1_000_000, 1));
}

#[test]
fn a_session_that_moved_is_filed_under_the_folder_its_directory_is_named_for() {
    let scratch = Scratch::new();
    let started = scratch.folder("started");
    let visited = scratch.folder("elsewhere");
    let projects = scratch.folder("projects");
    // The first line is a visit; the directory was named for where the session lives.
    let lines = format!(
        "{{\"type\":\"user\",\"cwd\":\"{}\"}}\n{{\"type\":\"user\",\"cwd\":\"{}\"}}\n",
        text(&visited),
        text(&started)
    );
    scratch.file(&format!("projects/{}/a.jsonl", directory_name(&text(&started))), &lines);

    let found = list_folders(&projects);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, text(&started));
}

#[test]
fn a_directory_with_no_folder_in_its_lines_or_no_records_is_left_out() {
    let scratch = Scratch::new();
    let projects = scratch.folder("projects");
    scratch.file("projects/-no-cwd/a.jsonl", "{\"type\":\"summary\"}\n");
    scratch.folder("projects/-empty");
    scratch.file("projects/-notes/readme.txt", "not a record");

    assert!(list_folders(&projects).is_empty());
    assert!(list_folders(&scratch.path().join("nowhere")).is_empty());
}

#[test]
fn a_huge_record_is_read_only_at_its_ends() {
    let scratch = Scratch::new();
    let app = scratch.folder("app");
    let moved = scratch.folder("moved");
    let projects = scratch.folder("projects");
    // The name, the folder at the start, a megabyte between with a folder no one should find, and the end.
    let junk = format!("{{\"type\":\"user\",\"cwd\":\"/in/the/middle\",\"pad\":\"{}\"}}\n", "x".repeat(900));
    let record = format!(
        "{{\"type\":\"user\",\"cwd\":\"{}\"}}\n{}{{\"type\":\"user\",\"cwd\":\"{}\"}}\n",
        text(&moved),
        junk.repeat(1200),
        text(&app)
    );
    scratch.file(&format!("projects/{}/a.jsonl", directory_name(&text(&app))), &record);

    let found = list_folders(&projects);

    assert_eq!(found[0].path, text(&app), "the end of the record names the folder the directory is for");
}
