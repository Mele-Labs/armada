# Spike: Does a reply ask the person for something?

**Answer: yes, from rules alone. On replies the rules had never seen, precision was 0.96 and
recall 0.68, or 0.87 once the calls that depend on Armada's walk loop are set aside.** After the
rules were tuned on all 457 labelled replies, they read 0.97 and 0.89. A wrong call costs one
dismiss, so the rules lean toward calling; the misses are listed by kind below.

Measured on 17 Oct 2026. The code is `crates/fleet/src/session_host/asks.rs`; what it feeds is
`docs/concepts/session.md`, *What a Job is doing*.

## What was labelled

The last assistant message before the next human prompt, from the owner's local Claude Code
transcripts. Subagent lines and tool results were skipped, as were turns that ended on a tool.
One labeller, the agent that built the rules, marked each `asks` (the person must answer, decide
or do something) or `tells` (it only reports). A polite "let me know if" closer with nothing
pending is `tells`.

The text and labels stay outside the repository. Machine output (retro JSON, verdicts) was
dropped from every count, since no person reads it.

| Set | Replies | Drawn | Used for |
|---|---|---|---|
| A, dev | 189 | 200 random, 100 with a cue, split 200 and 100 | Writing the rules |
| A, test | 91 | The same draw | First held-out run |
| B | 98 | 60 random, 40 with a cue | Second held-out run |
| C | 79 | 45 random, 35 with a cue | Third held-out run |

"A cue" is a `?` or a phrase like "want me", "which", "approve" in the last 500 characters. In the
random draws 30% of replies ask (86 of 287).

## Held-out runs

Each row is the first run of the rules as they stood on a set nobody had tuned them against.
"Strict" leaves out the replies marked hard: 81 of the 184 asks, where what is owed is a walk, an
approval or a command rather than a question.

| Set | Rules | Precision | Recall | Strict recall |
|---|---|---|---|---|
| A, test | First pass, tuned on A dev | 0.909 | 0.606 | 0.636 |
| B | Second pass | 0.943 | 0.673 | 0.759 |
| C | Third pass | 0.963 | 0.684 | 0.867 |

## Final rules, all 457 replies

These were tuned on every set, so they are not held out.

| | Asks | Tells |
|---|---|---|
| Called | 164 | 6 |
| Not called | 20 | 267 |

Precision 0.965, recall 0.891. Strict: precision 0.990, recall 0.942. Random draws only:
precision 0.951, recall 0.907.

## The misses

| Kind | Count | What the reply said |
|---|---|---|
| A walk or an approval, as what comes next | 5 | "It lands only after you approve the walk" |
| Steps for him with no cue word | 4 | A numbered list that starts "Open Settings and set" |
| Ask inside a table cell or findings list | 4 | "needs your merge" in a status table |
| An offer to reframe | 4 | "If the decision is another one, say so and I will drop this" |
| Ask more than two paragraphs from the end | 3 | A question, then a status section |

The six wrong calls are four optional tips ("Press ⌘R if the tab is stale"), one relayed
instruction and one "say which" addressed to another agent.

## The rules

The close is the last two paragraphs that are words (not code, a table row or a link), widened to
the lines that introduce a list. In it, the last sentence that matches wins.

| Signal | Example |
|---|---|
| Ends in `?`, unless wholly quoted | "Want me to land it?" |
| A phrase in `ASKING` | "should I", "which one", "your call", "once you approve" |
| An instruction at the start of a clause | "Press Allow once", "Next: merge #12" |
| A fenced `bash` block after a line | The line that hands it over |
| "I can X, or Y" | "I can trace it now, or file it" |

Three things stop a match: a sentence that says nothing is owed ("nothing is waiting"), an offer
left open ("if you want"), and a status said mid-sentence ("is still waiting for you").

## Why no model

Each miss above is one phrase to add, and the rules are read in a minute. A model would need the
same labelled replies, from one labeller, and would trade that readability for a dependency or a
new crate.

The misses are not word-order problems a bag of words would fix. They are whether something is the
person's to do, which depends on Armada's own loop of walks and waiting lists. If recall on fresh
replies falls below 0.8, a small trained model is the next step.

## What this does not cover

A terminal Job's mod used to report only the first line of its last message, so the rules saw too
little of one to call it (#2534). It now sends the close of the last message, the same last 1,500
characters from a line start that Fleet keeps for a hosted Job, and the rules read it as they read a
hosted one. A mod older than 0.3.39 still sends the first line, and its asks are missed as before.
