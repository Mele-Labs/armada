# Acceptance tests

One test per milestone, standing for the milestone's claim. It lives in
`crates/acceptance/`, it is written before the code it tests, and two things
watch it.

| Half | Owned by | Asserts |
|---|---|---|
| **Exists** | Rule one of `cargo xtask verify-foundations` | Each row of the table below names a file that is there and holds a `#[test]` |
| **Passes** | The `acceptance` Check in `armada.yml` | `cargo nextest run -p acceptance`, on every merge-line turn whose change it could break |

**The gate does not run the suite.** Until 2 Oct 2026 rule one ran
`cargo test -p acceptance` as well, so a merge-line turn compiled the suite for
the gate and again for the Check. The Check's `when` is everything the suite
compiles from, and a path missing from it is a change nothing tests.
`pnpm gate` runs the gate alone, so it says nothing about passing.

Read this before writing a milestone's test, reconciling one, or changing what
the gate asserts about it.

## What one is

> **Rule.** A milestone has exactly one acceptance test, and it drives the
> milestone's claim end to end.
> Why: a claim split across many tests is a claim no single run makes.

> **Rule.** The test is hermetic — no process spawned, no repository touched,
> no network opened, every adapter a fake.
> Why: a test that reaches the machine measures the machine.

| Milestone | The claim | Where |
|---|---|---|
| Reach | Armada works on a repository I did not write the Manifest for by hand | `crates/acceptance/tests/reach.rs` |
| Studio | I can work something out on a Studio — notes from using the app, what a scout read, what I pasted in — turn what holds up into a Job, and come back later to see how I got there | `crates/acceptance/tests/studio.rs` |
| Workspaces | A change runs the Checks of the manifests it touches and of those that depend on what it touched, and no others | `crates/acceptance/tests/workspaces.rs` |
| Triggers | A Trigger runs at a moment in a Job from a file in the repository or on this machine, the machine's wins, and a file that does not fit is named while the rest stand | `crates/acceptance/tests/triggers.rs` |
| Autopilot | I start Autopilot on a repository with one sentence and a cap, leave it, and come back to Jobs that ran, picks it was unsure of set aside with their reason, and nothing past the caps I set; Pause and Stop do what they say | `crates/acceptance/tests/autopilot.rs` |
| Pinned notes | A note I pin on a page reaches the Job with the line of code that drew the spot ranked first, found by the words on screen, and a stamp on the page counts only where the text bears it out | `crates/acceptance/tests/pinned_note.rs` |
| Token watching | A Job that crosses a token rule's limit is told once, with the number and one action; staying over tells it nothing more, it is told again only after the figure fell back under and a cooldown passed, and a Job under every limit hears nothing | `crates/acceptance/tests/usage_rules.rs` |
| Requests | A secret `{{token}}` fills in on an agent's send, the agent never reads it, and a commit or package carries only its name | `crates/acceptance/tests/requests.rs` |
| Shared home | A mod and a skill added to the home on one Mac are only in the home on another, Bring puts them in its mods and skills folders and they then read the same, and a server carried across holds its name and host and no key | `crates/acceptance/tests/shared_home.rs` |
| Beads | An issue filed on one machine shows on the other machine's Issues within a minute with nobody pulling by hand, and when two machines claim one issue both name the same winner after a sync and the loser is told who holds it | `crates/acceptance/tests/beads.rs` |
| Adversarial review | A plan waiting for approval is shown to one read-only call by its words alone, beside what the person asked and the repository's rules; each finding is pinned to the task or group it quotes, or to the whole plan, and an answer citing a file the checkout lacks or quoting words it was never shown is refused whole | `crates/acceptance/tests/adversarial_review.rs` |

The apparatus is `crates/acceptance/tests/bench/`, shared, with a file per
milestone. Claim and apparatus are separated so that what a milestone claims and
what it is claimed against read as different things.

## Why it is written first

> **Rule.** The test is written before the crates it calls.
> Why: the compiler's error list then becomes the list of what those crates
> have to provide.

Written first, the test asks for the vocabulary the claim needs, and the crates
are built to that shape. Written against a finished skeleton, it is shaped by
the code it finds, and it asserts what happens to be reachable.

## Why failing was enforced, and what it cost

Through M0 — Foundations the test named APIs that did not exist, so it did not
compile, and both the gate and a job hook asserted that. Rule one was
satisfied by a non-zero exit; a Stop hook ended any job in which
`cargo test -p acceptance` came back green.

| Cost | For the whole of M0 |
|---|---|
| `cargo nextest run --workspace` | Unusable — `--exclude acceptance` everywhere |
| The acceptance crate | Uncompiled, so unchecked by every other rule |
| An agent's instinct | Fighting the gate, which is why the hook existed |

Both mechanisms are now retired: the `acceptance` Check asserts the test
passes, and the Stop
hook and its entry in `.claude/settings.json` are deleted. The reasoning did
not retire with them. A milestone that can fake itself green proves nothing;
what changed is the direction the falsehood would run.

## Reconciling one

A test written first names an API that may be superseded while the code is
built. Reconciling is editing the test to the vocabulary that exists.

> **Rule.** A claim edited to match what was built claims nothing.
> Why: the test is the milestone's promise, not a description of the code.

Every assertion in a reconciliation is sorted into one of three, and no fourth:

| Verdict | Meaning | What happens |
|---|---|---|
| Stale | Same claim, renamed vocabulary | Rewrite against the new names |
| Right and unbuilt | Claim holds, code is missing | Build the code |
| Excluded | Claim is dropped for this milestone | Named as a change to the milestone |

An exclusion is a change to what the milestone promises. It is written down as
one, in the step that made it, and in the test's own header.

## A later milestone's test, and the one thing M0's answer cannot repeat

A later milestone writes its test first. `CLAUDE.md` says a milestone's test is
written before the code, so that half was never really open.

**What is decided here is that it compiles.** M0's test named APIs that did not
exist, so `cargo test -p acceptance` did not compile and rule one was satisfied
by a non-zero exit. That is unrepeatable now, for a reason that did not exist
when there was one milestone: the package holds every milestone's test and the
`acceptance` Check compiles all of them in one invocation. A test that named a type before its
issue built it would take every other milestone's test down with it, and their
claims would stop being run for the length of the build.

> **Rule.** A milestone's test may not stop an earlier milestone's test running.
> Why: a shipped claim that quietly stops being checked is the failure the gate
> exists to prevent, arriving from the inside.

So a test written before its code is written in vocabulary that exists, and it
merges green.

> **Rule.** No failing test is merged — a milestone's own test, written first,
> included.
> Why: the `acceptance` Check reads red whichever acceptance test fails, so a claim
> left red on `main` by design hides a real break in every other milestone's.

**It asserts what is carried, and states the rest.** A step of the claim that
cannot be asserted yet is a row in the test's header naming the issue that
carries it. The change that builds a step adds its assertion beside the code and
takes the row out. Two early milestones merged their tests red before this
rule; neither is red now.

**What that costs is stated rather than hidden.** A claim needing vocabulary
that does not exist yet cannot be an assertion, so it is named in the test's own
header as a claim the file does not carry and why. That is weaker than a compiler error list, and the compiler error list is
not available at this price.

**The header is the build order.** Its rows follow the order a person meets
the claim, so the next row to take out is the next thing to build — and a green
run always means every step asserted so far holds.
