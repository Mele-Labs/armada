//! Whether the end of an agent's reply asks the person for something. Every sentence here is written
//! for the test, in the shapes real replies take; `docs/spikes/reply-asks-classifier.md` has the
//! measurement over real ones.

use crate::session_host::asks::asking_line;

/// Replies that need the person, and the sentence that should be put in front of him.
const ASKS: &[(&str, &str)] = &[
    ("The retry cap is in. Want me to land it?", "Want me to land it?"),
    ("Both paths work.\n\nShould I keep the old flag or drop it?", "Should I keep the old flag or drop it?"),
    ("Tests pass.\n\nShall I open the pull request now?", "Shall I open the pull request now?"),
    ("I found two causes. Which one do you want fixed first?", "Which one do you want fixed first?"),
    ("The build is green. Do you want the docs touched too?", "Do you want the docs touched too?"),
    ("Done with the parser.\n\nWhat size should the page be?", "What size should the page be?"),
    ("Everything is staged. Would you like a single commit or two?", "Would you like a single commit or two?"),
    ("Could you run the migration on your machine and tell me what it prints?", "Could you run the migration on your machine and tell me what it prints?"),
    ("Two ways to go.\n\n- Keep the table and add a column.\n- Split it in two.\n\nLet me know which you prefer.", "Let me know which you prefer."),
    ("There are three options:\n\n1. Cache it in memory.\n2. Cache it on disk.\n3. Skip the cache.\n\nTell me which one.", "Tell me which one."),
    ("Choices for the label:\n\na) Rename the tab\nb) Leave it\n\nWhich do you want?", "Which do you want?"),
    ("The schema change is a breaking one. It is your call.", "It is your call."),
    ("Either works for me. Up to you.", "Up to you."),
    ("Both fixes are pushed.\n\n**Waiting on you**\n- Walk the new page.\n- Say whether the copy reads right.", "Say whether the copy reads right."),
    ("I stopped before the destructive step. I need you to confirm the branch name.", "I need you to confirm the branch name."),
    ("The change is committed.\n\nOnce you approve the look, I will add the tests.", "Once you approve the look, I will add the tests."),
    ("Next: merge #12 when you are happy with it.", "Next: merge #12 when you are happy with it."),
    ("The server restarted. Press Reload in the window.", "Press Reload in the window."),
    ("The hook refused my delete, so the branch is still there. To clear it yourself:\n\n```bash\ngit branch -D old-work\n```", "To clear it yourself:"),
    ("Cleanup is blocked from here. Run this in your checkout:\n\n```bash\nrm scratch.txt\n```\n\nThe slot is released after that.", "Run this in your checkout:"),
    ("I can trace the leak now, or file it for later.", "I can trace the leak now, or file it for later."),
    ("Shipped it.\n\nShould we also bump the version?\n\nI have not touched the lockfile.", "Should we also bump the version?"),
    ("Still waiting on you: the license choice.", "Still waiting on you: the license choice."),
    ("**Your call**\n- Drop the flag.\n- Keep it behind a setting.\n\nNothing else changed.", "Your call"),
    ("Could the title be shorter?\n\nI left it as it was.", "Could the title be shorter?"),
];

/// Replies that only report. Several end the way an ask would, so the sentence that tempts is the point.
const TELLS: &[&str] = &[
    "The retry cap is in and the tests pass.",
    "Landed as the third commit. Nothing is waiting on you.",
    "The fix is merged. If you want, I can also trim the logging.",
    "All done. Let me know if anything looks off.",
    "I changed the timeout from ten seconds to thirty. Tell me if you would rather keep it short.",
    "Two files changed:\n\n- `a.rs`\n- `b.rs`\n\nBoth pass.",
    "The check finished.\n\n| Test | Result |\n|---|---|\n| parser | pass |",
    "I asked whether the cache is shared, and it is not.",
    "You decided this earlier, so I kept it.",
    "It comes after you approve the plan, which is already recorded.",
    "The old build may need you to restart once more, but this one does not.",
    "Waiting for the result.",
    "Still waiting on `test`.",
    "You do not need to do anything. The slot is released.",
    "The walk is open at the address below.\n\nhttp://localhost:5173/?walk=thing",
    "",
    "   \n",
];

#[test]
fn a_reply_that_asks_gives_back_the_sentence_that_asks() {
    for (reply, expected) in ASKS {
        assert_eq!(asking_line(reply).as_deref(), Some(*expected), "{reply:?}");
    }
}

#[test]
fn a_reply_that_only_tells_asks_nothing() {
    for reply in TELLS {
        assert_eq!(asking_line(reply), None, "{reply:?}");
    }
}

/// A question the agent answers itself still reads as an ask: a wrong call costs one dismiss, a
/// missed one costs the person a Session left waiting.
#[test]
fn a_question_the_reply_answers_itself_is_still_called() {
    let reply = "Why did it fail? The lock was held by a process from the earlier run, so I reran and it passed.";
    assert_eq!(asking_line(reply).as_deref(), Some("Why did it fail?"));
}

#[test]
fn only_the_close_of_a_long_reply_is_read() {
    let long = format!("Should I rename it?\n\n{}\n\nA third paragraph that reports.\n\nA fourth that reports.", "Filler that reports. ".repeat(5));
    assert_eq!(asking_line(&long), None);
    let closing = format!("{long}\n\nWant me to go on?");
    assert_eq!(asking_line(&closing).as_deref(), Some("Want me to go on?"));
}

#[test]
fn the_last_ask_of_the_close_wins() {
    let reply = "Should I rename it?\n\nShould I also move it?";
    assert_eq!(asking_line(reply).as_deref(), Some("Should I also move it?"));
}
