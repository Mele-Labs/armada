//! What a hosted session's `auto` runs and what it asks about. The owner's
//! three classes and nothing else: a call that is none of them must not reach
//! a person. Since 23.71, `docs/spikes/029-*`.

use ipc::AskingToRun;

use crate::helm::{because_in_a_session, Because};

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
    because_in_a_session(&call("Bash", Command { command: line }), SLOT)
}

fn edit(tool: &str, path: &str) -> Option<Because> {
    because_in_a_session(&call(tool, Path { file_path: path }), SLOT)
}

#[test]
fn a_read_a_search_and_the_ordinary_shell_run_unasked() {
    let read = call(
        "Read",
        Path {
            file_path: "/repo/src/lib.rs",
        },
    );
    assert_eq!(because_in_a_session(&read, SLOT), None);
    let grep = call("Grep", Pattern { pattern: "fn main" });
    assert_eq!(because_in_a_session(&grep, SLOT), None);
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
            SLOT
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
    // A `..` is not read as inside, even under the directory's own prefix.
    let under = |path: &str| because_in_a_session(&call("Edit", Path { file_path: path }), "/etc");
    assert_eq!(under("/etc/hosts"), None);
    assert_eq!(under("/etc/../etc/hosts"), Some(Because::Destructive));
}

#[test]
fn the_three_classes_still_ask() {
    assert_eq!(bash("rm -rf /tmp/build"), Some(Because::Destructive));
    assert_eq!(bash("rm -rf ."), Some(Because::Destructive));
    assert_eq!(bash("cd /tmp && rm -rf target"), Some(Because::Destructive));
    assert_eq!(bash("git reset --hard HEAD~1"), Some(Because::Destructive));
    assert_eq!(
        bash("git checkout -- src/lib.rs"),
        Some(Because::Destructive)
    );
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
        |line: &str| because_in_a_session(&call("Bash", Command { command: line }), "/repo");
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
    let read = |path: &str| because_in_a_session(&call("Read", Path { file_path: path }), SLOT);
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
    assert_eq!(bash("echo \"x\" | rm -rf /tmp/y"), Some(Because::Destructive));
}
