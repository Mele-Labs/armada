//! The servers the agent CLI's files configure for a folder, read as the CLI reads them: spike 40.

use std::fs;
use std::path::{Path, PathBuf};

use crate::mcp_configured::{ConfigPlaces, McpConfig};

/// An empty folder of its own, with symlinks resolved, as a running job's folder is named.
fn folder(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("{name}-{}-{:?}", std::process::id(), std::thread::current().id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).unwrap();
    fs::canonicalize(&path).unwrap()
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn config(json: &str) -> McpConfig {
    ipc::decode("a server", json.as_bytes()).unwrap()
}

/// A repository at `home/repo` and a worktree of it at `home/repo/.armada/slots/slot-1`, as git
/// writes one.
fn worktree(home: &Path) -> (PathBuf, PathBuf) {
    let main = home.join("repo");
    let slot = main.join(".armada/slots/slot-1");
    write(&main.join(".git/worktrees/slot-1/commondir"), "../..\n");
    write(&slot.join(".git"), &format!("gitdir: {}\n", main.join(".git/worktrees/slot-1").display()));
    (main, slot)
}

#[test]
fn every_scope_is_read_and_a_name_found_twice_is_taken_as_the_cli_takes_it() {
    let home = folder("armada-mcp-configured-scopes");
    let (main, slot) = worktree(&home);
    write(
        &home.join(".claude.json"),
        &format!(
            r#"{{"numStartups":3,"mcpServers":{{"tracker-demo":{{"type":"stdio","command":"user"}},"notes-demo":{{"type":"http","url":"https://notes.example.test/mcp"}}}},
               "projects":{{"{}":{{"mcpServers":{{"tracker-demo":{{"type":"stdio","command":"local","args":["--port",4100],"env":{{"TOKEN":"x"}}}}}},
                                   "disabledMcpjsonServers":["turned-down"]}}}}}}"#,
            main.display()
        ),
    );
    write(
        &slot.join(".mcp.json"),
        r#"{"mcpServers":{"notes-demo":{"type":"stdio","command":"nearest"},"turned-down":{"type":"stdio","command":"no"}}}"#,
    );
    write(&main.join(".mcp.json"), r#"{"mcpServers":{"notes-demo":{"type":"stdio","command":"above"},"wiki-demo":{"type":"stdio","command":"above"}}}"#);

    let read = ConfigPlaces::of(&home, &slot).read().expect("read");

    let scopes: Vec<(&str, &str)> = read.iter().map(|(name, found)| (name.as_str(), found.scope)).collect();
    assert_eq!(scopes, [("notes-demo", "project"), ("tracker-demo", "local"), ("wiki-demo", "project")]);
    assert_eq!(
        read["tracker-demo"].config,
        config(r#"{"type":"stdio","command":"local","args":["--port",4100],"env":{"TOKEN":"x"}}"#),
        "local scope sits under the main checkout's key, and is kept whole"
    );
    assert_eq!(read["notes-demo"].config, config(r#"{"type":"stdio","command":"nearest"}"#));
    assert_eq!(format!("{:?}", read["tracker-demo"]), r#"Configured { scope: "local", config: McpConfig(..) }"#);
    fs::remove_dir_all(&home).ok();
}

#[test]
fn a_file_that_will_not_read_is_an_error_and_never_no_servers() {
    let home = folder("armada-mcp-configured-torn");
    let job = home.join("job");
    write(&home.join(".claude.json"), r#"{"mcpServers":{"tracker-demo":{"type":"stdio","command":"x"}}}"#);
    write(&job.join(".mcp.json"), r#"{"mcpServers":{"notes-demo":"#);

    let read = ConfigPlaces::of(&home, &job).read();

    assert!(read.as_ref().is_err_and(|why| why.contains(".mcp.json")), "{read:?}");
    fs::remove_dir_all(&home).ok();
}

#[test]
fn the_stamp_changes_when_a_file_is_written_and_only_then() {
    let home = folder("armada-mcp-configured-stamp");
    let job = home.join("job");
    fs::create_dir_all(&job).unwrap();
    let places = ConfigPlaces::of(&home, &job);
    let before = places.stamp();
    assert_eq!(places.stamp(), before);
    assert!(places.read().expect("no files is no servers").is_empty());

    write(&job.join(".mcp.json"), r#"{"mcpServers":{"tracker-demo":{"type":"stdio","command":"x"}}}"#);

    assert_ne!(places.stamp(), before);
    fs::remove_dir_all(&home).ok();
}
