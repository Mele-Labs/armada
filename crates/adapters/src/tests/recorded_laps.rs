//! The two `gh` calls that fetch a commit's recorded laps, asserted on their argument lists and on
//! how their answers are told apart. **Nothing here starts `gh` or reaches a network.**

use std::cell::RefCell;
use std::path::Path;

use adapter_traits::{LapsOnForge, NotAsked};

use crate::recorded_laps::{
    artifact_of, download_arguments, fetch_through, list_arguments, run_in_listing, Ran, Run, WORKFLOW,
};

const COMMIT: &str = "4f2a9c1e07b35d68a1c4e9f02b7d5a3c8e6f1b24";

fn said(ok: bool, stdout: &str, said: &str) -> Result<Ran, String> {
    Ok(Ran { ok, stdout: stdout.to_string(), said: said.to_string() })
}

/// `gh`, as a script: the listing it prints, and what the download does. Every call is kept.
struct Gh {
    listing: Result<Ran, String>,
    /// The file the download writes into the folder it is given, or why it fails.
    download: Result<Option<&'static [u8]>, &'static str>,
    calls: RefCell<Vec<Vec<String>>>,
}

impl Gh {
    fn listing(listing: &str, download: Result<Option<&'static [u8]>, &'static str>) -> Gh {
        Gh { listing: said(true, listing, ""), download, calls: RefCell::new(Vec::new()) }
    }

    fn asked(&self, commit: &str) -> Result<LapsOnForge, NotAsked> {
        fetch_through(commit, &|args: &[String]| {
            self.calls.borrow_mut().push(args.to_vec());
            match args.get(1).map(String::as_str) {
                Some("list") => match &self.listing {
                    Ok(listed) => Ok(Ran { ok: listed.ok, stdout: listed.stdout.clone(), said: listed.said.clone() }),
                    Err(why) => Err(why.clone()),
                },
                _ => match &self.download {
                    Ok(file) => {
                        let into = &args[args.iter().position(|arg| arg == "--dir").expect("a folder") + 1];
                        if let Some(bytes) = file {
                            std::fs::write(Path::new(into).join("laps.json"), bytes).expect("written");
                        }
                        said(true, "", "")
                    }
                    Err(why) => said(false, "", why),
                },
            }
        })
    }

    fn calls(&self) -> Vec<Vec<String>> {
        self.calls.borrow().clone()
    }
}

#[test]
fn the_run_is_listed_by_workflow_commit_and_push_and_only_the_newest() {
    let args = list_arguments(COMMIT);
    assert_eq!(
        args,
        [
            "run", "list", "--workflow", WORKFLOW, "--commit", COMMIT, "--event", "push", "--limit", "1", "--json",
            "databaseId,status", "--jq", ".[] | \"\\(.databaseId) \\(.status)\""
        ]
    );
    assert_eq!(WORKFLOW, "checks.yml");
}

#[test]
fn the_artifact_is_downloaded_by_the_name_the_workflow_uploads_it_under() {
    let args = download_arguments("9001", COMMIT, Path::new("/tmp/into"));
    assert_eq!(args, ["run", "download", "9001", "--name", &format!("test-map-{COMMIT}"), "--dir", "/tmp/into"]);
    assert_eq!(artifact_of(COMMIT), format!("test-map-{COMMIT}"));
}

#[test]
fn a_listing_names_a_run_and_whether_it_finished() {
    assert_eq!(run_in_listing("123456 completed\n"), Some(Run { id: "123456".into(), finished: true }));
    assert_eq!(run_in_listing("123456 in_progress"), Some(Run { id: "123456".into(), finished: false }));
    assert_eq!(run_in_listing("123456 queued"), Some(Run { id: "123456".into(), finished: false }));
    assert_eq!(run_in_listing(""), None);
    assert_eq!(run_in_listing("\n  \n"), None);
    assert_eq!(run_in_listing("no runs found"), None, "words are not a run");
}

#[test]
fn a_commit_with_no_run_yet_is_not_yet_and_nothing_is_downloaded() {
    let gh = Gh::listing("", Ok(None));
    assert_eq!(gh.asked(COMMIT), Ok(LapsOnForge::NotYet));
    assert_eq!(gh.calls(), vec![list_arguments(COMMIT)]);
}

#[test]
fn an_artifact_is_handed_back_as_its_bytes_even_while_the_run_goes_on() {
    let gh = Gh::listing("77 in_progress\n", Ok(Some(b"{\"laps\":[]}")));
    assert_eq!(gh.asked(COMMIT), Ok(LapsOnForge::Found(b"{\"laps\":[]}".to_vec())));
    let calls = gh.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(&calls[1][..5], ["run", "download", "77", "--name", &format!("test-map-{COMMIT}")]);
    let folder = Path::new(&calls[1][6]);
    assert!(!folder.exists(), "the folder it was downloaded into is gone");
}

#[test]
fn a_run_still_going_without_the_artifact_is_not_yet() {
    let gh = Gh::listing("77 in_progress", Err("no valid artifacts found to download"));
    assert_eq!(gh.asked(COMMIT), Ok(LapsOnForge::NotYet));
}

#[test]
fn a_finished_run_without_the_artifact_is_never() {
    let gh = Gh::listing("77 completed", Err("no valid artifacts found to download"));
    assert_eq!(gh.asked(COMMIT), Ok(LapsOnForge::Never));
}

#[test]
fn a_finished_run_whose_download_failed_otherwise_is_a_fault_in_the_tools_words() {
    let gh = Gh::listing("77 completed", Err("HTTP 502: bad gateway"));
    assert_eq!(gh.asked(COMMIT), Err(NotAsked("HTTP 502: bad gateway".into())));
}

#[test]
fn a_forge_that_cannot_be_asked_is_a_fault_and_not_an_absent_artifact() {
    let gh = Gh {
        listing: said(false, "", "could not find any workflows named checks.yml"),
        download: Ok(None),
        calls: RefCell::new(Vec::new()),
    };
    assert_eq!(gh.asked(COMMIT), Err(NotAsked("could not find any workflows named checks.yml".into())));
    assert_eq!(gh.calls().len(), 1);

    let missing = Gh { listing: Err("gh would not start: not found".into()), download: Ok(None), calls: RefCell::new(Vec::new()) };
    assert_eq!(missing.asked(COMMIT), Err(NotAsked("gh would not start: not found".into())));
}
