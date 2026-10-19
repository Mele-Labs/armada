//! Where a hosted session may write, as the lease ledger says. **A session writes in a slot it
//! leased, or one something it started leased (its subagents), and in no other**: not an Edit, a
//! `sed -i`, an interpreter, a new file or a `git` that changes the tree. `docs/concepts/session.md`,
//! *What the write gate covers*. The owner's rule, 13 Oct 2026.

use ipc::{GateAnswer, SessionId};

use super::session_gate::{in_its_slot, lease, leased_for, started, Quoted};
use super::session_host::{denied, reason, rig_placing, Rig};
use crate::tests::peer::Placing;

const OTHER: &str = "0b0b0b0b-0000-0000-0000-000000000000";

/// A slot another session's agent leased: held under that session's id, as `armada worktree
/// lease` writes it from a hosted session's process, with a file in it.
fn leased_by(rig: &Rig, slot: u32, other: &str) -> String {
    let at = adapter_traits::slot_path(&rig.root, slot);
    std::fs::create_dir_all(format!("{at}/src")).expect("a slot");
    std::fs::write(
        format!("{at}.lease"),
        format!("branch sessions/theirs\nholder job {other}\nsince 1\n"),
    )
    .expect("a lease record");
    std::fs::write(format!("{at}/src/lib.rs"), "fn a() {}\n").expect("a file");
    at
}

/// The same for a slot `armada worktree lease` held for the session itself.
fn with_a_file(slot: &str) {
    std::fs::create_dir_all(format!("{slot}/src")).expect("a slot");
    std::fs::write(format!("{slot}/src/lib.rs"), "fn a() {}\n").expect("a file");
}

async fn run(rig: &Rig, id: &SessionId, cwd: &str, command: &str) -> GateAnswer {
    let encoded = ipc::encode(&Quoted { command }).expect("a line");
    rig.gate_in(cwd, id, "Bash", &encoded).await
}

async fn tool(rig: &Rig, id: &SessionId, cwd: &str, name: &str, path: &str) -> GateAnswer {
    rig.gate_in(cwd, id, name, &format!(r#"{{"file_path":"{path}"}}"#)).await
}

/// The ways a line writes into `slot`, by absolute path.
fn writing_into(slot: &str) -> Vec<String> {
    vec![
        format!("sed -i '' 's/a/b/' {slot}/src/lib.rs"),
        format!("sed -i 's/a/b/' {slot}/src/lib.rs"),
        format!("perl -pi -e 's/a/b/' {slot}/src/lib.rs"),
        format!("echo x > {slot}/new.txt"),
        format!("echo x >> {slot}/src/lib.rs"),
        format!("touch {slot}/new.txt"),
        format!("python3 -c \"open('{slot}/src/lib.rs', 'a').write('x')\""),
        format!("python3 - <<'EOF'\nopen('{slot}/new.py', 'w').write('x')\nEOF"),
        format!("python3 {slot}/tools/rewrite.py"),
        format!("node -e \"require('fs').writeFileSync('{slot}/n.js', 'x')\""),
        format!("ruby -e \"File.write('{slot}/r.rb', 'x')\""),
        format!("git -C {slot} merge main"),
        format!("git -C {slot} reset --hard HEAD~1"),
        format!("git -C {slot} checkout other"),
        format!("git --work-tree={slot} checkout ."),
        format!("git --git-dir={slot}/.git --work-tree {slot} reset --hard"),
    ]
}

/// The ways a line writes into whatever directory it runs in, with no path in it to read.
const WRITING_HERE: &[&str] = &[
    "touch new.txt",
    "echo x > new.txt",
    "sed -i 's/a/b/' src/lib.rs",
    "python3 edit.py",
    "python3 -c \"open('src/lib.rs', 'a').write('x')\"",
    "git merge main",
    "git reset --hard HEAD~1",
    "git checkout other",
    "cargo build",
];

#[tokio::test]
async fn a_slot_its_subagent_leased_takes_the_parents_writes_and_the_subagents() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    // The subagent ran `armada worktree lease`, which held the slot under the session's id.
    let theirs = leased_for(&rig, 9, &id);
    with_a_file(&theirs);
    let file = format!("{theirs}/src/lib.rs");

    // The parent, from its own slot, by every route.
    for name in ["Edit", "Write", "MultiEdit"] {
        assert!(!denied(&tool(&rig, &id, &rig.root, name, &file).await), "{name}");
        assert!(!denied(&tool(&rig, &id, &rig.root, name, &format!("{theirs}/new.rs")).await));
    }
    for line in writing_into(&theirs) {
        assert!(!denied(&run(&rig, &id, &rig.root, &line).await), "{line} should run");
    }
    // The subagent, standing in the slot it leased.
    for name in ["Edit", "Write"] {
        assert!(!denied(&tool(&rig, &id, &theirs, name, "src/lib.rs").await));
        assert!(!denied(&tool(&rig, &id, &theirs, name, "src/new.rs").await));
    }
    for line in WRITING_HERE {
        assert!(!denied(&run(&rig, &id, &theirs, line).await), "{line} should run in its slot");
    }
}

#[tokio::test]
async fn a_slot_another_session_leased_takes_no_write_from_this_one() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let me = std::process::id();
    // One held by another session's agent, one by a process that is nothing of this session's.
    let by_session = leased_by(&rig, 6, OTHER);
    let by_process = lease(&rig, 7, me, &started(me));
    with_a_file(&by_process);
    for theirs in [&by_session, &by_process] {
        for name in ["Edit", "Write", "MultiEdit", "NotebookEdit"] {
            for path in [format!("{theirs}/src/lib.rs"), format!("{theirs}/new.rs")] {
                let answer = tool(&rig, &id, &rig.root, name, &path).await;
                assert!(denied(&answer), "{name} {path} should be held");
                assert!(reason(&answer).contains("not leased by this job"));
            }
        }
        for line in writing_into(theirs) {
            assert!(denied(&run(&rig, &id, &rig.root, &line).await), "{line} should be held");
        }
        // From the slot it does hold, with a path that climbs out of it.
        let own = adapter_traits::slot_path(&rig.root, 1);
        let name = theirs.rsplit('/').next().unwrap();
        for line in [
            format!("sed -i 's/a/b/' ../{name}/src/lib.rs"),
            format!("echo x > ../{name}/new.txt"),
            format!("touch ../{name}/new.txt"),
            format!("git -C ../{name} merge main"),
        ] {
            assert!(denied(&run(&rig, &id, &own, &line).await), "{line} should be held");
        }
        assert!(denied(&tool(&rig, &id, &own, "Edit", &format!("../{name}/src/lib.rs")).await));
    }
}

/// The working directory persists between a session's calls, so one `cd` into a slot that is not
/// the session's leaves every later call standing in it with no path to read.
#[tokio::test]
async fn a_call_standing_in_a_slot_the_session_does_not_hold_writes_nothing_there() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let theirs = leased_by(&rig, 6, OTHER);
    for line in WRITING_HERE {
        let answer = run(&rig, &id, &theirs, line).await;
        assert!(denied(&answer), "{line} should be held");
        assert!(reason(&answer).contains("not leased by this job"));
    }
    for name in ["Edit", "Write"] {
        for path in ["src/lib.rs", "src/new.rs"] {
            assert!(denied(&tool(&rig, &id, &theirs, name, path).await), "{name} {path}");
        }
    }
    // Looking is not writing.
    for line in ["git status", "git log --oneline", "ls src", "cat src/lib.rs", "grep -rn fn src"] {
        assert!(!denied(&run(&rig, &id, &theirs, line).await), "{line} should run");
    }
    // A line that leaves for a slot it holds writes there.
    let own = adapter_traits::slot_path(&rig.root, 1);
    assert!(!denied(&run(&rig, &id, &theirs, &format!("cd {own} && touch f")).await));
}

#[tokio::test]
async fn a_fork_writes_in_none_of_the_slots_of_the_session_it_came_from() {
    let peers = Placing::nothing();
    let (rig, parent) = in_its_slot(&peers).await;
    // The old session's slot (1), its subagent's (5), and one held by a process under it (7).
    let subagent = leased_for(&rig, 5, &parent);
    with_a_file(&subagent);
    let own = adapter_traits::slot_path(&rig.root, 1);
    with_a_file(&own);
    let me = std::process::id();
    let by_process = lease(&rig, 7, me, &started(me));
    with_a_file(&by_process);
    peers.started(1001, me);

    let fork = rig.start().await;
    rig.send(&fork, "carry on").await;
    // It has leased a slot of its own, so a write it is refused is refused for whose slot it is
    // and not for having none yet.
    let mine = leased_for(&rig, 2, &fork);
    with_a_file(&mine);
    let mut hosting = rig.fleet.store().lock().await.hosting(fork.as_str()).unwrap().unwrap();
    hosting.fork_of = Some(parent.as_str().to_string());
    hosting.lease_slot = Some(2);
    rig.fleet.store().lock().await.keep_hosting(&hosting).unwrap();

    // The old session still may, in all three.
    for slot in [&own, &subagent, &by_process] {
        assert!(!denied(&tool(&rig, &parent, &rig.root, "Edit", &format!("{slot}/src/lib.rs")).await));
    }

    for slot in [&own, &subagent, &by_process] {
        for name in ["Edit", "Write"] {
            for path in [format!("{slot}/src/lib.rs"), format!("{slot}/new.rs")] {
                let answer = tool(&rig, &fork, &rig.root, name, &path).await;
                assert!(denied(&answer) && reason(&answer).contains("not leased by this job"), "{name} {path}");
            }
        }
        for line in writing_into(slot) {
            let answer = run(&rig, &fork, &rig.root, &line).await;
            assert!(denied(&answer) && reason(&answer).contains("not leased by this job"), "{line}");
        }
        for line in WRITING_HERE {
            let answer = run(&rig, &fork, slot, line).await;
            assert!(denied(&answer) && reason(&answer).contains("not leased by this job"), "{line} in {slot}");
        }
        let entered = run(&rig, &fork, &rig.root, &format!("cd {slot} && git add -A")).await;
        assert!(denied(&entered) && reason(&entered).contains("not leased by this job"));
    }
    // Its own it may write.
    assert!(!denied(&tool(&rig, &fork, &rig.root, "Edit", &format!("{mine}/src/lib.rs")).await));
}

/// The owner, 15 Oct 2026: the ledger draws every slot the session holds, those its subagents
/// leased included, and none another session holds.
#[tokio::test]
async fn the_ledger_draws_every_slot_the_session_holds_its_subagents_included() {
    use api::HostedSessions as _;
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    // One leased under the session's id, one by a subagent's process that named the session.
    leased_for(&rig, 9, &id);
    super::slot_leases::leased(&rig.root, 5, "sessions/sub", super::slot_leases::By::Session(id.as_str()));
    leased_by(&rig, 6, OTHER);

    let record = std::sync::Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session;
    let slots: Vec<&str> = record
        .attachments
        .iter()
        .filter(|one| one.kind == "slot" && one.state == ipc::AttachmentState::Standing)
        .map(|one| one.target.as_str())
        .collect();
    assert_eq!(slots, vec!["1", "5", "9"]);
}

/// What the permission door says to a call the session made, placed by its connection.
async fn at_the_door(rig: &Rig, tool: &str, input: &str) -> ipc::RunOrNot {
    use api::Conversations;
    let asking: ipc::AskingToRun = ipc::decode(
        "an ask",
        format!(r#"{{"tool_name":"{tool}","input":{input}}}"#).as_bytes(),
    )
    .expect("an ask");
    let caller = api::Caller::at("127.0.0.1:50999".parse().unwrap());
    std::sync::Arc::clone(&rig.fleet).ask_the_person_at(asking, None, caller).await.expect("answered")
}

/// The door and the gate read one ledger. A slot the gate lets the session write in is not
/// "outside your slot" to the door (13 Oct 2026: the parent's Edit and the subagent's own Edit and
/// `sed -i` were refused in the slot the subagent leased).
#[tokio::test]
async fn the_door_lets_the_session_write_in_a_slot_its_subagent_leased() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    peers.holding(1001, 50999, rig.fleet.host().port);
    let theirs = leased_for(&rig, 4, &id);
    with_a_file(&theirs);
    let allowed = |answer: ipc::RunOrNot| matches!(answer, ipc::RunOrNot::Allow { .. });

    assert!(allowed(at_the_door(&rig, "Edit", &format!(r#"{{"file_path":"{theirs}/src/lib.rs"}}"#)).await));
    for line in [
        format!("sed -i '' 's/a/b/' {theirs}/src/lib.rs"),
        format!("rm {theirs}/src/lib.rs"),
        format!("cd {theirs} && git merge main"),
        format!("cd {theirs} && git reset --hard HEAD~1"),
    ] {
        let encoded = ipc::encode(&Quoted { command: &line }).expect("a line");
        assert!(allowed(at_the_door(&rig, "Bash", &encoded).await), "{line}");
    }
    // Whether a slot somebody else holds is its own is the gate's, above, and the held list's
    // at `session_auto`. This rig sits in the temp dir, which the door treats as scratch.
}

/// #2239. A line that only looks (`cd` first, `git -C`, a GET) leased a slot to a session that
/// never wrote, and the slot then had a holder that was not working it. A line that writes still
/// leases.
#[tokio::test]
async fn a_read_that_looks_elsewhere_leases_no_slot_and_a_commit_still_does() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "look around").await;
    rig.stand_in.init(0);

    for line in [
        "cd /tmp && git log --oneline | head -3",
        "git -C /tmp status",
        "git merge-base origin/main HEAD",
        "git remote -v",
        "gh issue view 2239",
        "gh api repos/Mele-Labs/armada/pulls/2234",
    ] {
        assert!(!denied(&run(&rig, &id, &rig.root, line).await), "{line} should run");
    }
    let hosting = rig.fleet.store().lock().await.hosting(id.as_str()).unwrap().unwrap();
    assert_eq!(hosting.lease_slot, None, "nothing leased for a read");

    for line in ["git commit -m x", "gh pr create --fill", "sed -i s/a/b/ src/lib.rs"] {
        let fresh = rig_placing(None);
        let id = fresh.start().await;
        fresh.send(&id, "change a file").await;
        fresh.stand_in.init(0);
        assert!(denied(&run(&fresh, &id, &fresh.root, line).await), "{line} should lease");
    }
}

/// 15 Oct 2026: the door counted `setup.worktrees`, eight, so on a machine of fifteen bays a
/// slot a subagent leased past slot-8 was refused as outside the session's slots.
#[tokio::test]
async fn the_pool_counts_every_bay_this_machine_has_past_the_manifests_eight() {
    let rig = rig_placing(None);
    let slots = format!("{}/{}", rig.root.trim_end_matches('/'), adapter_traits::SLOT_ROOT);
    std::fs::create_dir_all(&slots).expect("the slots");
    let bays: Vec<String> = (1..=15).map(|n| n.to_string()).collect();
    std::fs::write(format!("{slots}/pool"), format!("bays {}\n", bays.join(" "))).expect("the pool");

    let served = rig.fleet.served_named(None).expect("one repository");
    assert_eq!(crate::leasing::bays_of(&served), (1..=15).collect::<Vec<u32>>());
}
