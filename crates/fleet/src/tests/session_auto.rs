//! What a hosted session's `auto` runs and what it asks about. The owner's
//! three classes and nothing else: a call that is none of them must not reach
//! a person. Since 23.71, `docs/spikes/029-*`.

use ipc::AskingToRun;

use crate::helm::{because_in_a_session, because_in_a_session_owning, Because, Own};

const SLOT: &str = "/repo/.armada/slots/slot-3";

#[derive(serde::Serialize)]
struct Ask<'a, T: serde::Serialize> {
    tool_name: &'a str,
    input: T,
}

#[derive(serde::Serialize)]
struct Command<'a> {
    command: &'a str,
}

#[derive(serde::Serialize)]
struct Path<'a> {
    file_path: &'a str,
}

#[derive(serde::Serialize)]
struct Pattern<'a> {
    pattern: &'a str,
}

fn call<T: serde::Serialize>(tool: &str, input: T) -> AskingToRun {
    let body = ipc::encode(&Ask {
        tool_name: tool,
        input,
    })
    .expect("an ask");
    ipc::decode("an ask", body.as_bytes()).expect("an ask")
}

fn bash(line: &str) -> Option<Because> {
    because_in_a_session(&call("Bash", Command { command: line }), &[SLOT])
}

fn edit(tool: &str, path: &str) -> Option<Because> {
    because_in_a_session(&call(tool, Path { file_path: path }), &[SLOT])
}

#[test]
fn a_read_a_search_and_the_ordinary_shell_run_unasked() {
    let read = call(
        "Read",
        Path {
            file_path: "/repo/src/lib.rs",
        },
    );
    assert_eq!(because_in_a_session(&read, &[SLOT]), None);
    let grep = call("Grep", Pattern { pattern: "fn main" });
    assert_eq!(because_in_a_session(&grep, &[SLOT]), None);
    for line in [
        "git status",
        "git log --oneline -5",
        "cargo build",
        "cargo build 2>&1 | tail -5",
        "cargo build 2>/dev/null",
        "ls -la >/dev/null 2>&1",
        "cd crates/fleet && cargo test --lib",
        "pnpm --dir apps/desktop exec vitest run",
        "git checkout -b sessions/next",
        "git stash list",
        "git add -A && git commit -m \"Say why\"",
        "git commit -m \"$(cat <<'EOF'\nRemove the old rm -rf path; git push is separate\n\nCo-Authored-By: x\nEOF\n)\"",
        "armada worktree lease sessions/next",
    ] {
        assert_eq!(bash(line), None, "{line} should run");
    }
}

#[test]
fn an_edit_in_the_sessions_own_slot_runs_and_one_outside_it_still_asks() {
    for tool in ["Write", "Edit", "NotebookEdit"] {
        assert_eq!(edit(tool, &format!("{SLOT}/Cargo.toml")), None);
    }
    assert_eq!(
        because_in_a_session(
            &call(
                "Edit",
                Path {
                    file_path: "src/lib.rs"
                }
            ),
            &[SLOT]
        ),
        None,
        "a relative path is read against the slot"
    );
    // The file exists, so overwriting it is the destructive act.
    assert_eq!(
        edit("Edit", "/etc/hosts"),
        Some(Because::Destructive),
        "outside the slot, an overwrite keeps its card"
    );
    // A `..` is followed to where the path lands: back inside is inside, out past the
    // directory is not, whatever prefix it starts with.
    let under = |path: &str, held: &str| {
        because_in_a_session(&call("Edit", Path { file_path: path }), &[held])
    };
    assert_eq!(under("/etc/hosts", "/etc"), None);
    assert_eq!(under("/etc/../etc/hosts", "/etc"), None);
    assert_eq!(under("/etc/ssh/../hosts", "/etc/ssh"), Some(Because::Destructive));
}

/// The refused write from a retro: an `Edit` of a file that exists in `/tmp`, and the
/// heredoc writes a subagent made, were read as outside the slot and `/tmp`.
#[test]
fn a_write_is_read_where_it_lands_so_scratch_and_the_slot_pass() {
    let real = std::env::temp_dir().join(format!("armada-scratch-{}", std::process::id()));
    std::fs::create_dir_all(real.join("crates/api/src/tests")).unwrap();
    let file = real.join("crates/api/src/tests/reach.rs");
    std::fs::write(&file, "fn a() {}").unwrap();
    let file = file.to_string_lossy().into_owned();
    // The file exists, so it is an overwrite, and scratch is still no one's work.
    for tool in ["Write", "Edit", "NotebookEdit"] {
        assert_eq!(edit(tool, &file), None, "{tool} of {file}");
    }
    assert_eq!(edit("Edit", "/tmp/helm-tells/crates/api/src/lib.rs"), None);
    assert_eq!(edit("Edit", &format!("{SLOT}/crates/../Cargo.toml")), None);
    assert_eq!(edit("Edit", "/tmp/../etc/hosts"), Some(Because::Destructive));
    assert_eq!(
        bash(&format!("cat > {SLOT}/crates/../src/new.rs <<'EOF'\nfn main() {{}}\nEOF")),
        None
    );
    assert_eq!(
        bash(&format!("cat > {file} <<'EOF'\nfn main() {{}}\nEOF")),
        None
    );
    // A symlink in scratch that points at a file elsewhere is read as that file.
    #[cfg(unix)]
    {
        let link = real.join("hosts");
        std::os::unix::fs::symlink("/etc/hosts", &link).unwrap();
        let link = link.to_string_lossy().into_owned();
        assert_eq!(edit("Edit", &link), Some(Because::Destructive));
    }
    std::fs::remove_dir_all(&real).unwrap();
}

#[test]
fn the_three_classes_still_ask() {
    assert_eq!(bash("rm -rf /opt/build"), Some(Because::Destructive));
    assert_eq!(bash("rm -rf ."), Some(Because::Destructive));
    assert_eq!(bash("cd /tmp && rm -rf target"), Some(Because::Destructive));
    assert_eq!(bash("git -C /elsewhere reset --hard HEAD~1"), Some(Because::Destructive));
    assert_eq!(bash("git stash pop"), Some(Because::Destructive));
    assert_eq!(bash("git branch -D other"), Some(Because::Destructive));
    assert_eq!(bash("git tag -d v1"), Some(Because::Destructive));
    assert_eq!(bash("echo hi > /etc/hosts"), Some(Because::Destructive));
    assert_eq!(bash("git push origin main"), Some(Because::PushesToShared));
    assert_eq!(bash("echo $(git push)"), Some(Because::PushesToShared));
    assert_eq!(bash("gh pr merge 12"), Some(Because::PushesToShared));
    assert_eq!(bash("cargo publish"), Some(Because::PushesToShared));
    assert_eq!(
        bash("curl -X POST https://example.com -d x"),
        Some(Because::WritesOffMachine)
    );
    assert_eq!(bash("gh issue create"), Some(Because::WritesOffMachine));
    assert_eq!(bash("gh issue edit 12 --body-file b.md"), Some(Because::WritesOffMachine));
    assert_eq!(bash("gh pr edit 12 --title t"), Some(Because::WritesOffMachine));
    assert_eq!(bash("gh issue close 12"), Some(Because::Destructive));
    assert_eq!(bash("eval \"$X\""), Some(Because::Unreadable));
}

#[test]
fn removing_or_overwriting_a_file_in_the_sessions_own_slot_runs() {
    for line in [
        "rm packages/overview/src/Summary.tsx && echo ok",
        "rm -rf build",
        "cargo build && rm -rf target",
        "rmdir empty/",
        &format!("rm {SLOT}/notes.txt"),
        "echo hi > notes.txt",
        "cargo test 2>/dev/null > out.log",
        "git rm -q src/old.rs && cat > src/new.rs <<'EOF'\nfn main() {}\nEOF",
        &format!("cd {SLOT}/apps/mock && cat > scenarios/x.ts <<'EOF'\nconst a = () => 1;\nEOF"),
        &format!("cd {SLOT}/apps && sed -i 's/a/b/' x.ts && cat > y.ts <<'EOF'\nb\nEOF"),
        "cd packages && rm old.ts",
    ] {
        assert_eq!(bash(line), None, "{line} should run");
    }
    for line in [
        "rm ../other/file",
        &format!("rm -rf {SLOT}"),
        "rm -rf /repo/.armada/slots/slot-4/x",
        "rm",
        "cd /tmp && rm x",
        "cd .. && rm x",
        "cd && rm x",
    ] {
        assert_eq!(bash(line), Some(Because::Destructive), "{line} should ask");
    }
}

#[test]
fn before_a_lease_the_repositorys_root_is_not_a_slot() {
    let at_root =
        |line: &str| because_in_a_session(&call("Bash", Command { command: line }), &["/repo"]);
    assert_eq!(at_root("rm notes.txt"), Some(Because::Destructive));
    assert_eq!(at_root("echo hi > notes.txt"), Some(Because::Destructive));
}

#[test]
fn reaching_into_the_owners_files_asks_unless_it_names_armada() {
    for line in [
        "find / -name pnpm -maxdepth 6 -type f 2>/dev/null | head -3",
        "ls ~/Documents",
        "ls ~",
        "grep -r foo $HOME/Music",
        "cat /Users/someone/Desktop/notes.txt",
        "ls /Volumes/Backup",
    ] {
        assert_eq!(bash(line), Some(Because::ReachesOutside), "{line} should ask");
    }
    for line in [
        "ls ~/Library/Application\\ Support/Armada/mod",
        "cat /Users/someone/Development/armada/README.md",
        "ls ~/.claude/projects/-Users-someone-Development-armada/memory",
        "ls /opt/homebrew/bin",
        "cargo build 2>/dev/null",
        "cd /tmp && ls",
    ] {
        assert_eq!(bash(line), None, "{line} should run");
    }
    let read = |path: &str| because_in_a_session(&call("Read", Path { file_path: path }), &[SLOT]);
    assert!(bash("rm ~/notes.txt").is_some(), "outside the slot still asks");
    assert!(bash("rm $HOME/x").is_some(), "outside the slot still asks");
    assert_eq!(read("/Users/someone/Desktop/shot.png"), Some(Because::ReachesOutside));
    assert_eq!(read(&format!("{SLOT}/src/lib.rs")), None);
}

#[test]
fn quoted_text_is_an_argument_not_a_redirect_or_a_pipe() {
    for line in [
        "git add -A && git commit -qm \"WIP: roomier bar\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\"; git log --oneline -1",
        "sed -i '' 's|<CallPane item={call} />|<CallPane item={call} onOpen={x} />|' apps/desktop/src/Dashboard.tsx",
        "grep -n 'a > b' src/lib.rs",
        "echo \"x; rm -rf build\"",
    ] {
        assert_eq!(bash(line), None, "{line} should run");
    }
    assert_eq!(bash("echo hi > /etc/hosts"), Some(Because::Destructive));
    assert_eq!(bash("echo 'x' > /etc/hosts"), Some(Because::Destructive));
    assert_eq!(bash("echo \"x\" | rm -rf /opt/y"), Some(Because::Destructive));
}

#[test]
fn a_page_opened_in_the_browser_is_found_in_every_segment() {
    use crate::helm::pages_opened;
    assert_eq!(
        pages_opened(
            r#"open "http://localhost:5191/?walk=a"; open "http://localhost:5191/?walk=b""#
        ),
        ["http://localhost:5191/?walk=a", "http://localhost:5191/?walk=b"]
    );
    assert_eq!(
        pages_opened("open -a Safari https://example.com/x && echo done"),
        ["https://example.com/x"]
    );
    assert_eq!(
        pages_opened("xdg-open http://a.test/ | cat"),
        ["http://a.test/"]
    );
    assert_eq!(
        pages_opened("python3 -m webbrowser http://a.test/"),
        ["http://a.test/"]
    );
    for line in ["open file.txt", "open .", "echo http://a.test/", "curl http://a.test/"] {
        assert!(pages_opened(line).is_empty(), "{line}");
    }
}

#[test]
fn a_session_rewrites_its_own_branch_and_writes_scratch_unasked() {
    for line in [
        "git tag -f dashboard-mock-backup HEAD && git diff 1340ee498 HEAD -- . > /tmp/dashboard.patch",
        "git reset -q --hard origin/main && git apply --3way /tmp/dashboard.patch 2>&1 | tail -8; git status --short | head -40",
        "git checkout -- src/lib.rs",
        "git rebase origin/main",
        "rm /tmp/dashboard.patch",
    ] {
        assert_eq!(bash(line), None, "{line} should run");
    }
    assert!(bash("rm /tmp/../etc/hosts").is_some());
}

/// A slot a subagent leased is the session's own: what the session may do in the slot it runs in
/// it may do in that one.
#[test]
fn every_slot_the_session_holds_is_its_own_for_an_edit_and_a_shell_line() {
    const OTHER: &str = "/repo/.armada/slots/slot-9";
    let held = [SLOT, OTHER];
    let bash = |line: &str| because_in_a_session(&call("Bash", Command { command: line }), &held);

    // The file exists, so overwriting it is the destructive act anywhere it is not held, and
    // it is built beside the crate rather than in `/tmp`, where an overwrite runs anyway.
    let real = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("armada-held-{}", std::process::id()));
    let slot = real.join(".armada/slots/slot-9");
    std::fs::create_dir_all(slot.join("src")).unwrap();
    std::fs::write(slot.join("src/lib.rs"), "fn a() {}").unwrap();
    let file = slot.join("src/lib.rs").to_string_lossy().into_owned();
    let theirs = slot.to_string_lossy().into_owned();
    let edit = |held: &[&str]| because_in_a_session(&call("Edit", Path { file_path: &file }), held);
    assert_eq!(edit(&[SLOT]), Some(Because::Destructive));
    assert_eq!(edit(&[SLOT, &theirs]), None);
    std::fs::remove_dir_all(&real).unwrap();

    for line in [
        format!("sed -i '' 's/a/b/' {OTHER}/src/lib.rs"),
        format!("sed -i 's/a/b/' {OTHER}/src/lib.rs"),
        format!("rm {OTHER}/src/old.rs"),
        format!("echo x > {OTHER}/notes.txt"),
        format!("cd {OTHER} && git merge main"),
        format!("cd {OTHER} && git reset --hard HEAD~1"),
        format!("python3 -c \"open('{OTHER}/a.py', 'w').write('x')\""),
    ] {
        assert_eq!(bash(&line), None, "{line} should run");
    }
    // A slot it does not hold is still outside it.
    assert_eq!(
        because_in_a_session(&call("Bash", Command { command: "rm /repo/.armada/slots/slot-2/a.rs" }), &held),
        Some(Because::Destructive)
    );
    assert_eq!(
        because_in_a_session(&call("Bash", Command { command: "echo x > /repo/.armada/slots/slot-2/a.rs" }), &held),
        Some(Because::Destructive)
    );
}

/// What a session started or made is its own to remove (the owner, retro of session 9c74d82e):
/// a dev server, a merged branch of its own and a scratch file. Nothing else's.
#[test]
fn a_session_removes_what_it_started_and_nothing_more() {
    let started = |pid: u32| pid == 4120 || pid == 4121;
    let merged = ["armada/session-9c74d82e"];
    let own = Own { started: Some(&started), merged_branches: &merged };
    let run = |line: &str| because_in_a_session_owning(&call("Bash", Command { command: line }), &[SLOT], own);

    for line in [
        "kill 4120",
        "kill -9 4120 4121",
        "kill -TERM 4120",
        "kill -s TERM 4120",
        "git branch -D armada/session-9c74d82e",
        "git branch -d armada/session-9c74d82e",
        "rm $TMPDIR/scratch_test.rs",
        "rm \"${TMPDIR}/scratch_test.rs\"",
    ] {
        assert_eq!(run(line), None, "{line} should run");
    }
    for line in [
        // A process it did not start, or one it cannot name.
        "kill 99",
        "kill 4120 99",
        "kill",
        "kill -9",
        // A process group, and every process.
        "kill -9 -1",
        "kill -9 -4120",
        "kill 1",
        "kill 0",
        "pkill node",
        "killall node",
        // A branch of someone else's, an unmerged one, a remote's, a rename and a glob.
        "git branch -D armada/session-other",
        "git branch -D armada/session-9c74d82e armada/session-other",
        "git branch -D",
        "git branch -D -f armada/session-9c74d82e",
        "git branch -D armada/*",
        // Scratch that climbs out, or hides a second path.
        "rm $TMPDIR/../x",
        "rm $TMPDIR/a /etc/hosts",
        "rm $TMPDIR",
    ] {
        assert_eq!(run(line), Some(Because::Destructive), "{line} should be refused");
    }
    assert_eq!(run("git push origin --delete armada/session-9c74d82e"), Some(Because::PushesToShared));
    // With nothing owned the same lines are refused, as before.
    assert_eq!(bash("kill 4120"), Some(Because::Destructive));
    assert_eq!(bash("git branch -D armada/session-9c74d82e"), Some(Because::Destructive));
}
