//! A shell line that only reads, or writes only under `/tmp` or a held slot, runs;
//! the same line aimed at the owner's files or another session's slot does not.
//! Each `ran` case is one the guard refused on 15 Oct 2026.

use ipc::AskingToRun;

use crate::helm::{because_in_a_session, Because};

const SLOT: &str = "/repo/.armada/slots/slot-3";

#[derive(serde::Serialize)]
struct Ask<'a> {
    tool_name: &'a str,
    input: Command<'a>,
}

#[derive(serde::Serialize)]
struct Command<'a> {
    command: &'a str,
}

fn bash(line: &str) -> Option<Because> {
    let body = ipc::encode(&Ask { tool_name: "Bash", input: Command { command: line } }).expect("an ask");
    let ask: AskingToRun = ipc::decode("an ask", body.as_bytes()).expect("an ask");
    because_in_a_session(&ask, &[SLOT])
}

fn ran(lines: &[&str]) {
    for line in lines {
        assert_eq!(bash(line), None, "{line} should run");
    }
}

fn refused(lines: &[&str], why: Because) {
    for line in lines {
        assert_eq!(bash(line), Some(why), "{line} should be refused");
    }
}

#[test]
fn a_tilde_or_dollar_inside_an_awk_script_is_not_a_path() {
    ran(&[
        r#"ps -Ao pid,ppid,pri,%cpu,comm | awk '$5 ~ /rustc$/ {printf "%s %s\n", $1, $4}'"#,
        r#"ps -Ao pid,comm | awk '$2 ~ /node$/ {print $1}' | head -3"#,
        r#"echo '$HOME is not expanded in single quotes'"#,
        r#"grep -c "~" notes.txt"#,
    ]);
    refused(
        &["ls ~/Documents", "cat \"$HOME/Desktop/a.txt\"", "find ~ -name x"],
        Because::ReachesOutside,
    );
}

#[test]
fn a_launchctl_or_system_query_reads() {
    ran(&[
        "launchctl print gui/502/com.armada.fleet | grep -iE 'spawn type|process type'",
        "launchctl list | grep armada",
        "sleep 3 &\nps -o pid,ppid,pri,command -p $!; wait; launchctl print gui/502/com.armada.fleet 2>/dev/null",
        "diskutil list",
        "systemctl status ssh",
    ]);
    refused(
        &[
            "launchctl unload /repo/x.plist",
            "launchctl bootout gui/502/com.armada.fleet",
            "launchctl kickstart -k gui/502/com.armada.fleet",
            "diskutil eraseDisk JHFS+ x disk2",
            "systemctl stop ssh",
        ],
        Because::Destructive,
    );
}

#[test]
fn a_write_under_tmp_is_scratch_however_the_path_is_spelled() {
    ran(&[
        "for p in 1 2; do ps -o pid -p $p >> /tmp/fleet-kids-$$.txt; done",
        "for p in 1 2; do echo $p > /tmp/fleet-kids-$$.txt; done",
        "out=/tmp/armada-load-samples.txt; : > $out",
        "out=/tmp/armada-load-samples.txt\n: > \"$out\"\nuptime >> \"$out\"",
        "out=$TMPDIR/samples.txt; : > $out",
    ]);
    refused(
        &[
            "echo x > /tmp/../etc/hosts",
            "out=/etc/hosts; : > $out",
            "out=/tmp/a; out=/etc/hosts; : > $out",
            "echo x > $UNSET",
            "echo x > /tmp/$USER/../../etc/hosts",
        ],
        Because::Destructive,
    );
}

#[test]
fn the_owners_toolchain_config_is_read_but_his_credentials_are_not() {
    ran(&[
        "cat ~/.cargo/config.toml ~/.cargo/config 2>/dev/null; command -v sccache",
        "ls $HOME/.cargo/bin",
        "cat ~/.rustup/settings.toml",
    ]);
    refused(
        &[
            "cat ~/.ssh/id_ed25519",
            "cat ~/.aws/credentials",
            "ls ~/Documents",
            "cat ~/.cargo/../.ssh/id_rsa",
        ],
        Because::ReachesOutside,
    );
    // A write there is not a read.
    assert!(bash("echo x > ~/.cargo/config.toml").is_some());
    assert!(bash("rm ~/.cargo/config.toml").is_some());
    assert!(bash("cp x ~/.cargo/config.toml").is_some());
}

#[test]
fn a_heredoc_script_over_the_slots_own_tree_reads() {
    ran(&[
        "R=/repo/.armada/slots/slot-3\npython3 - \"$R\" <<'EOF'\nimport os, sys\nfor d, _, fs in os.walk(os.path.join(sys.argv[1], 'apps')):\n    print(d > '~', '$HOME')\nEOF",
        "R=\"$(git rev-parse --show-toplevel)\"\npython3 - \"$R\" <<'EOF'\nimport os\nprint(os.walk('/Users/x'))\nEOF",
        "python3 - <<EOF\nprint('/Users/x/Desktop')\nEOF",
        "python3 -I - <<-'PY'\n\tprint(1)\n\tPY",
    ]);
}

#[test]
fn chmod_and_heredocs_inside_a_slot_are_the_sessions_work() {
    ran(&[
        "chmod +x /repo/.armada/slots/slot-3/scripts/run.sh",
        "chmod 755 scripts/run.sh scripts/other.sh",
        "chmod -R u+w target",
        "cat > scripts/run.sh <<'EOF'\n#!/bin/sh\necho hi\nEOF",
        "cat > scripts/run.sh <<EOF\n#!/bin/sh\necho $HOME\nEOF",
        "cat <<'EOF' > /repo/.armada/slots/slot-3/notes.md\nhello\nEOF",
        "cd scripts && cat > run.sh <<'EOF'\nhi\nEOF",
        "cat >> scripts/run.sh <<'EOF'\nhi\nEOF",
    ]);
    refused(
        &[
            "chmod 777 /etc/hosts",
            "chmod +x scripts/run.sh /etc/hosts",
            "chown root /repo/.armada/slots/slot-9/a",
            "chmod +x /repo/.armada/slots/slot-3/../../x",
            "cd /etc && chmod +x hosts",
            "cd /etc && cat > hosts <<'EOF'\nx\nEOF",
        ],
        Because::Destructive,
    );
}

/// A change of mode is a removal's class, so a folder the owner allowed never lets it run unasked.
#[test]
fn chmod_in_the_owners_home_is_destructive() {
    refused(&["chmod -R 777 ~/.ssh", "chmod 777 $HOME/x"], Because::Destructive);
}

#[test]
fn another_sessions_slot_may_be_read_but_not_written() {
    ran(&[
        "git -C /repo/.armada/slots/slot-9 status",
        "git -C /repo/.armada/slots/slot-9 log --oneline -5",
        "git -C /repo/.armada/slots/slot-9 diff --stat",
        "ls /repo/.armada/slots/slot-9/crates",
        "cat /repo/.armada/slots/slot-9/Cargo.toml | head",
        "grep -rn foo /repo/.armada/slots/slot-9/crates",
        "find /repo/.armada/slots/slot-9 -name '*.rs' -maxdepth 3",
    ]);
    refused(
        &[
            "rm /repo/.armada/slots/slot-9/a.rs",
            "echo x > /repo/.armada/slots/slot-9/a.rs",
            "git -C /repo/.armada/slots/slot-9 reset --hard",
            "git -C /repo/.armada/slots/slot-9 checkout main",
            "git -C /repo/.armada/slots/slot-9 clean -fd",
            "chmod +x /repo/.armada/slots/slot-9/a.sh",
            "cat > /repo/.armada/slots/slot-9/a.rs <<'EOF'\nx\nEOF",
        ],
        Because::Destructive,
    );
}

#[test]
fn a_write_outside_the_slots_and_tmp_is_still_refused() {
    refused(
        &[
            "rm /repo/src/main.rs",
            "rm -rf /repo/crates",
            "echo x > /repo/README.md",
            "cat > /repo/README.md <<'EOF'\nx\nEOF",
            "mv /repo/a /repo/b",
            "truncate -s 0 /repo/a",
            "do rm /repo/a",
            "for f in a b; do rm /repo/$f; done",
            "if true; then rm /repo/a; fi",
        ],
        Because::Destructive,
    );
}

#[test]
fn a_read_outside_the_slots_runs_unless_macos_or_credentials_guard_it() {
    // Each one refused a session on 15 or 16 Oct 2026.
    ran(&[
        "cd ~/develop && grep -rl \"now run the preview at\" --include=*.ts . 2>/dev/null | head; ls ~/develop | head -50",
        "F=~/.claude/projects/-Users-user-develop-other/c42b7404.jsonl; grep -c 'run the preview at' $F",
        "ls ~/.claude/projects/ | grep -i other",
        "cat /Users/user/develop/other/.claude/worktrees/cm/.scratch/HANDOFF.md",
        "jq -r 'select(.a // \"\") | .b' /tmp/x.jsonl",
        "ls ~",
        "ls ~ | head",
        "cat ~/Library/Logs/x.log",
    ]);
    refused(
        &[
            "find / -name pnpm",
            "grep -r token ~",
            "ls ~/Downloads",
            "cat ~/Library/Mobile\\ Documents/x",
            "cat ~/.ssh/config",
            "cat ~/.config/gh/hosts.yml",
            "ls /Volumes/Backup",
            "cp ~/notes.txt /tmp/notes.txt",
        ],
        Because::ReachesOutside,
    );
}

#[test]
fn git_pointed_at_a_held_slot_is_its_own_and_cd_out_of_it_is_not() {
    ran(&[
        "git -C /repo/.armada/slots/slot-3 reset -q --keep HEAD~1",
        "git -C /repo/.armada/slots/slot-3 checkout -- src/lib.rs",
        "cd /repo/.armada/slots/slot-3 && git rebase origin/main",
        "git reset --hard HEAD~1",
    ]);
    refused(
        &[
            "git -C /repo reset --hard",
            "git -C /repo/.armada/slots/slot-3/../../.. reset --hard",
            "git -C $SLOT reset --hard",
            "cd /repo && git reset --hard",
            "cd /repo && git checkout main",
            "git -C /repo/.armada/slots/slot-3 --work-tree=/repo reset --hard",
            "git -C /repo/.armada/slots/slot-3 stash",
        ],
        Because::Destructive,
    );
}

#[test]
fn a_subshell_is_read_by_what_it_runs() {
    ran(&["(cd /Users/user/develop/other && git log --oneline -3)", "(cd /repo/.armada/slots/slot-3 && rm x)"]);
    refused(&["(cd /etc && rm hosts)", "(rm /repo/a)"], Because::Destructive);
}

/// Each put to the owner as a reach on 16 Oct 2026: an escaped `|` inside quotes cut the line
/// into a second command, and a quoted path with a space was read as two words.
#[test]
fn an_escaped_pipe_and_a_quoted_path_with_a_space_are_text() {
    ran(&[
        r#"grep -n "a\|\.b\|/Users/user/x" f.rs; git show origin/main:f.rs | grep -n "a\|/Users/user/x""#,
        r#"D="/Users/user/Library/Application Support/Armada/armada.db"; sqlite3 -readonly "$D" "select 1""#,
        r#"cat "/Users/user/Library/Application Support/Tool/log.txt""#,
    ]);
    refused(&[r#"cat "/Users/user/Documents/My Notes/a.txt""#], Because::ReachesOutside);
    refused(&[r#"sqlite3 "/Users/user/Library/Application Support/Tool/x.db" "delete from t""#], Because::ReachesOutside);
}

/// The Job's scratch folder, as `crate::scratch` names it.
const SCRATCH: &str = "/Users/person/.armada/machine/attachments/job-scratch/job-1";

fn bash_holding_scratch(line: &str) -> Option<Because> {
    let body = ipc::encode(&Ask { tool_name: "Bash", input: Command { command: line } }).expect("an ask");
    let ask: AskingToRun = ipc::decode("an ask", body.as_bytes()).expect("an ask");
    because_in_a_session(&ask, &[SLOT, SCRATCH])
}

#[test]
fn the_jobs_scratch_folder_is_written_removed_and_chmodded_as_a_slot_is() {
    for line in [
        "cat > /Users/person/.armada/machine/attachments/job-scratch/job-1/state.sh <<'EOF'\n#!/bin/sh\necho hi\nEOF",
        "chmod +x /Users/person/.armada/machine/attachments/job-scratch/job-1/state.sh",
        "rm -rf /Users/person/.armada/machine/attachments/job-scratch/job-1/old",
    ] {
        assert_eq!(bash_holding_scratch(line), None, "{line} should run");
    }
    for line in [
        "chmod +x /Users/person/.armada/machine/attachments/job-scratch/job-2/state.sh",
        "rm -rf /Users/person/.armada/machine/attachments/job-scratch/job-1",
    ] {
        assert_eq!(bash_holding_scratch(line), Some(Because::Destructive), "{line} should be refused");
    }
}
