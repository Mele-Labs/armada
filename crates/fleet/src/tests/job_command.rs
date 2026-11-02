//! Which shell lines `armada job` is allowed to run without asking, and the ones it is not.
//!
//! Every refusal here is a line that would do something besides the one command, so a prompt is
//! the right answer to it.

use crate::session_host::job_command::is_one_armada_job;

#[test]
fn one_armada_job_command_is_allowed_whatever_it_is_given() {
    for line in [
        "armada job",
        "armada job status",
        "armada job finish_up",
        r#"armada job dispatch repo=web title="Fix the login" message='a; b | c && d'"#,
        r#"armada job waiting_for '{"items":[{"id":"a","text":"Restart; now"}]}'"#,
        r#"armada job record_plan "two words" '$HOME `x`'"#,
        "armada   job\tstatus ",
        "armada job message='two\nlines in quotes'",
    ] {
        assert!(is_one_armada_job(line), "{line:?} is one armada job command");
    }
}

#[test]
fn a_second_command_a_pipe_or_a_redirect_is_the_shell_s_and_asks() {
    for line in [
        "armada job x; rm -rf ~",
        "armada job x && y",
        "armada job x || y",
        "armada job x | y",
        "armada job x & ",
        "armada job x > out",
        "armada job x < in",
        "armada job x 2>&1",
        "armada job x\nrm -rf ~",
        "armada job x\r",
        "(armada job x)",
        "armada job $HOME",
        "armada job ${X}",
        "armada job `y`",
        r#"armada job "$(y)""#,
        r#"armada job "`y`""#,
        r#"armada job "$HOME""#,
        r#"armada job "a\"b""#,
        r"armada job a\ b",
        r#"armada job x "unterminated"#,
        "armada job x 'unterminated",
    ] {
        assert!(!is_one_armada_job(line), "{line:?} does more than one command");
    }
}

#[test]
fn only_the_program_armada_with_the_verb_job_is_the_shortcut() {
    for line in [
        "",
        "armada",
        "armada jobs",
        "armada mcp",
        "armada check test",
        "rm armada job",
        "FOO=1 armada job",
        "/usr/local/bin/armada job",
        "echo armada job",
        "armadajob",
    ] {
        assert!(!is_one_armada_job(line), "{line:?} is not `armada job`");
    }
}

#[test]
fn one_heredoc_with_a_single_quoted_delimiter_carries_a_body_to_stdin() {
    let body = "armada job record_plan - <<'PLAN'\n{\"summary\":\"a; b `c` $d\"}\nPLAN";
    assert!(is_one_armada_job(body));
    assert!(is_one_armada_job(&format!("{body}\n")), "a trailing newline is the shell's own");
    assert!(is_one_armada_job("armada job waiting_for - <<'EOF'   \n{}\nEOF\n\n"));
}

#[test]
fn a_heredoc_that_could_expand_run_on_or_leave_stdin_unnamed_is_refused() {
    for line in [
        "armada job x - <<EOF\n{}\nEOF",
        "armada job x - <<\"EOF\"\n{}\nEOF",
        "armada job x - <<-'EOF'\n{}\nEOF",
        "armada job x <<'EOF'\n{}\nEOF",
        "armada job x - <<'EOF'\n{}",
        "armada job x - <<'EOF'\n{}\nEOF\nrm -rf ~",
        "armada job x - <<'EOF' | sh\n{}\nEOF",
        "armada job x - <<'EOF'; rm -rf ~\n{}\nEOF",
        "armada job x - <<'EOF' && y\n{}\nEOF",
        "armada job x - <<'E O F'\n{}\nE O F",
        "armada job x - <<''\n{}\n",
        "armada job x - <<'EOF'\n{}\nEOF extra",
        "rm - <<'EOF'\n{}\nEOF",
        "armada job x - <<'EOF'\n<<'EOF'\nEOF\n<<'EOF'\n{}\nEOF",
    ] {
        assert!(!is_one_armada_job(line), "{line:?} must ask");
    }
}
