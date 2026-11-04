# Spike 39 — Can the on-device model take Fleet's small calls?

**Answer: it is cheap and it runs, and it is not good enough for the two calls that make most of Fleet's volume.**
A call is a 92 KB Swift tool, 22 MB of memory and 0.1 s of CPU, against a 300 MB Node process per call today.
But the account and the running summary read as well-formed to Fleet's own readers and are mostly wrong or empty of the work: 79 of 80 summaries repeated the prompt's own example.
**Move no call yet.** Two small calls, the quick-reply chips and the dispute read, are worth a quality trial first; nothing here measured them.

Measured 4 Nov 2026 on this Mac: macOS 26.5.2, arm64, 32 GB, Swift 6.3.2 from the Command Line Tools alone.
The question is the owner's: `#3260` found the Claude CLI started once per small call, about 100 times an hour, to be part of what made his Mac chug, and he will not spend Claude tokens on those calls.
No Claude call was made to set a baseline.
The cost of today's path is taken as given: one ~300 MB process per call, plus the owner's plugin hooks.

## How it was run

`039-ask.swift` is the whole caller: it reads one prompt on stdin, streams `SystemLanguageModel` and prints the answer, with first and last token times and the error kind as one JSON line on stderr.
`039-readers.rs` mounts `crates/fleet/src/session_account.rs` unchanged and exposes its question builders and its readers (`read`, `read_so_far`), so the prompts and the pass test measured are the shipped ones.
`039-run.py` drives both one call at a time, each in a fresh process under `/usr/bin/time -l`.
The proposer's readers (`field`, `setting`) are ported to Python in it, because `session_proposer.rs` needs the rest of Fleet to build.
Fleet's code is unchanged.

Real inputs: 80 snapshots cut at random points of turns in this repository's own Claude Code transcripts, with the calls turned into `now` words as `activity.rs` does, and 30 first requests for the proposer.
**They are not committed**: they hold the owner's messages.
What is committed is `039-transcript.txt` (six invented turns, each through the account and the summary, as shipped and without the prompts' examples) and `039-proposer.txt` (14 invented requests).
The shipped-prompt answers shown below come from those.
Pass rates and gradings of the real snapshots are aggregates.

## Availability, and the smallest way to call it

`SystemLanguageModel.default.availability` is `available`, and it supports 23 languages.
`swiftc` from the Command Line Tools builds the caller in 18 s, with no Xcode, no entitlement and no signing step.
`contextSize` and `tokenCount(for:)` need a deployment target of macOS 26.4; the call itself needs 26.0.

Nothing lighter turned up for generation.
Apple ships no command-line tool for the model, and the system process that holds it is not ours to talk to.
The three shapes are:

| Shape | Per call | Status |
|---|---|---|
| A process per call, as the Claude CLI is today | 22 MB, 0.05 to 0.1 s CPU, 0.4 to 0.6 s of process start | Measured |
| A long-lived helper on a pipe | Same answers 0.4 to 0.6 s sooner: first token 0.62 to 0.89 s after the first call | Measured, 6 calls in one process |
| Swift linked into Fleet as a library | No process at all | Not measured |

Fleet's seam already fits a process.
`ModelClient::render` returns a `JudgeCall` (a program, its arguments and the question) and `said` in `crates/fleet/src/judging/running.rs` writes the question to stdin and reads stdout.
A client whose program is the helper is a second `ModelClient` and no change to the trait.
Fleet holds one client for all sites (`Fleet.judge`), so choosing per site needs a router in front of it.

## What a call costs

Account prompt, 80 calls, one at a time, one process each, no response cap.
Last-token times leave out the 2 calls that ran away (below).

| | First token | Last token | CPU | Peak memory of the caller |
|---|---|---|---|---|
| Median | 0.99 s | 1.66 s | 0.10 s | 21.9 MB |
| 90th percentile | 1.36 s | 2.38 s | 0.13 s | |
| Worst | 2.00 s | 6.13 s | 0.36 s | 22.5 MB |

The running summary is shorter: median 0.83 s to first token, 1.02 s to the last, 0.05 s of CPU.
Time scales with the prompt: the proposer's 474-token prompt took 0.70 s to the first token, and real requests of about 2,300 tokens took 2.7 s.

**Cold against warm.** The model sits in a system process, so a call after a gap is not slower by much:

| Gap since the last call | 0 s | 30 s | 120 s | 300 s | 600 s |
|---|---|---|---|---|---|
| First token | 0.83 to 1.33 s | 1.42 s | 1.38 s | 1.70 s | 1.77 s |

The 900 s gap did not finish inside the run's time limit.
A call after hours idle or after a reboot was not measured.
The system process's own memory and energy were not measured either, only the caller's.

**A call can run away.** Two of the 80 account calls repeated `done:` lines until they hit the 4,096-token limit.
One took 56 s, 3,118 tokens and 8 s of CPU in the caller.
With `maximumResponseTokens` set to 200, the same two took 2.6 s and 1.1 s and read as valid.
Every call after that one carried the cap, and a real client needs it.

## Context window and guardrails

The window is 4,096 tokens for the prompt and the answer together, about 16,000 characters of English.
Past it the call fails with `exceededContextWindowSize`, after the model has read all of it.
The account's question is bounded by its own caps and stayed under 1,610 tokens in all 80.
**11 of 30 real first requests overflowed on the proposer**, which carries the request uncapped (the largest was 27,000 tokens).
The call sites that carry diffs, files, logs or the whole Logbook are out of reach by size alone.

About 300 calls ran in all.
**No guardrail violation and no refusal.**
Eight code-ish cases ran under the default guardrails and under `.permissiveContentTransformations`: a diff, a compiler error, `git push --force` and `rm -rf`, an SQL injection fix, a leaked key, an exploit report, and "kills", "murders" and "abuses" used about processes.
None was refused, and the answers were as wrong or right with the permissive setting as without.
The only errors in the whole run were the context overflows.

## Quality

### The account, 80 real snapshots

| | Count |
|---|---|
| Followed `done:`, `now:`, `next:` | 78 |
| `read` kept at least one line | 72 of 80 (90%) |
| `read` returned nothing | 8 (garbled lists, or every line named code) |
| All three lines kept | 60 |

**The reader passes what a person would not.**
I graded 40 of the answers by hand against what the turn did (this is my judgement, not a second reader's).
7 were good, 12 vague but harmless ("The agent is on the next step"), 19 wrong or leaking, and 2 unreadable.
The leaks are the kind `names_code` does not see by shape:

- **A `done:` line that echoes a call** ("grep found 10 lines", "git status", "Git fetched all the latest changes"), 12 of 80.
- **The prompt's own example in the answer** ("the sign-in page"), 10 of 80, where no input mentions it.
- **A tool's name that is not on the reader's list or in a hump** (`TaskCreate for the sign-in page`).
- **An invented outcome**: "Pull request was merged" for a push, "Custom margins removed from the code and commit history" for an edit of a note.

Side by side, invented turns, as shipped (`039-transcript.txt`):

| The turn | The model wrote |
|---|---|
| "make the profile page remember my last tab"; reading, editing the page, running tests | done: edited profile page to remember last tab · now: running tests · next: add stored choice for last tab and read it back |
| "the export button opens a new tab…"; read, edit, `git diff` | done: the button was a link that opens a new tab; it is now a download link. · now: updating the export button to download · next: adding the download functionality |
| "Merge main into your branch…"; fetch, merge, test | done: Merged main with no conflicts. The tests are running. · now: Running git push · next: Push the changes to the main branch. |
| "why is the nightly sync taking four hours now?"; search, read two files, run tests | done: The retry loop waits a full minute between tries. · now: Checking the retry tests. · next: Running the retry tests. |
| "Approved: …walk=invoice-list"; add, push, `gh pr` | done: the pull request is open and CI is running. · now: reviewing the code changes. · next: making the necessary fixes. |
| "fix the price rounding"; read, edit, with a diff in what the agent said | done: rounded the money · now: fixing the button color · next: testing the new button |

Some copy what the agent said, which is right, and some invent a next step the turn does not hold.

### The running summary: 79 of 80 repeat its example

80 calls (40 snapshots, twice, the second with the first's answer as the summary so far).
`read_so_far` kept all 80.
**79 of them say "Found the reads happen once per Job. Added a cache, not keyed yet." or a variant**, which is the example the prompt gives for the voice.
On the six invented turns, the shipped prompt gave that sentence for five.

With both examples taken out of the prompt, 15 snapshots read as follows: 6 good (mostly the agent's own words, shortened), 4 vague, 5 wrong.
Here the input to the model was partly the account's own wrong `done` lines, so the 6 is a floor and the 5 a ceiling.

| The turn | Shipped | Without the examples |
|---|---|---|
| Profile page, last tab | The agent added a stored choice for the last tab and read it back when the page opens. The test is running. | The profile page now remembers the last tab. |
| Export button, download | Found the reads happen once per Job. Added a cache, not keyed yet. | The button was changed from a link to a download link. |
| Nightly sync | Found the reads happen once per Job. Added a cache, not keyed yet. | The job was completed, but the nightly sync is now taking four hours because the retry loop waits a full minute between tries, even for a failure that cannot succeed. |
| Pull request opened | Found the reads happen once per Job. Added a cache, not keyed yet. | The pull request is open and CI is running. |

The prompt's examples are the cause, and the no-example answers are not yet trustworthy: the nightly-sync summary says the sync "is now taking four hours" as a result.

### The proposer

On the 14 invented requests (each with a workflow I expected):

| | First run | Rerun, committed script |
|---|---|---|
| Followed the format | 14 | 14 |
| Model and effort in the lists | 14 and 13 | 14 and 14 |
| Workflow as expected | 4 (one of them `none` for a question) | 2 |
| Said `none` for a request a held workflow fits | 9 | 11 |

It picks a workflow rarely and a different one from run to run.
It names `opus` for 16 of the 19 real requests that fit; titles were usually fair (`Fix typo`, `Review invoice rounding change`) and sometimes not (`add_dark_mode_toggle_to_settings_page_and_remember_choice_between_visits`).
On the 19 real requests that fit the window, the answer was readable 19 times and named no workflow 19 times.
`workflow-pick-by-model.md` found that an embedder cannot pick the workflow either, so neither local model takes that call.

## Recommendation, per call site

`model_asked` is called from two places, the account and the summary, both in `session_host/account.rs`.
The other sites are those the survey found that call `Ask::put`.
**The load is mostly the account**: it may be asked once every 5 s a Session is at work (`WRITE_EVERY`), and the summary after each turn and each call to the person.

| Site | On-device? | Why, and what it would take |
|---|---|---|
| Account (`session_host/account.rs`) | **No** | Reads well-formed and is 18% good by my grading. Needs a prompt without examples, a response cap, guided generation instead of free lines (not tried), a reader that drops lines echoing a call, and a labelled set judged by a person before it replaces anything |
| Running summary (`so_far`) | **No** | 79 of 80 repeat the example. A shipped example-free prompt reaches roughly 40% good, which is too low for a line the person reads before he answers a call. Showing the agent's own last sentence, trimmed, needs no model |
| Session proposer (`session_host/proposing.rs`) | **No** | Workflow 2 to 4 of 14, `none` most of the time, `opus` nearly always, 37% of real requests over the window. A title-only call might pass, and was not measured against his renames |
| Quick-reply chips (`session_host/quick.rs`) | **Try** | Cosmetic, a lenient reader, about 2.5k characters in (under 700 tokens), a failure is cached as empty. Quality not run here |
| Dispute read (`session_host/dispute_reading.rs`) | **Try, with a cap** | `pushback: yes\|no`, a small answer, one scan a minute with a call per unread notice; its inputs are uncapped replies, and a false no loses the dispute. Quality not run here |
| Wait read (`session_host/after.rs`) | Later | Five lines, total reader, but the request is uncapped and 37% of real ones overflow |
| Helm routing (`helm/routing.rs`) | No | Already tries the local embedder first; the model call carries up to 20 candidates and 2,000 characters |
| Test map (`test_map_reading.rs`) | No | A 12 KiB file and 200 test paths: over the window |
| Autopilot decide, pick, sentence (`autopilot/*`) | No | A wrong answer acts for the person or starts a Job, and the inputs are uncapped |
| Retros, Logbook distil, backlog, tidy, sort, review of retro items, Teach (`retro/*`, `logbook/*`, `teaching/`) | No | Long inputs (10 to 100k characters), strict JSON, and the owner moved the retro from haiku to sonnet after a rejected retro |
| Judge, review runs (`judge_serving.rs`, `review_runs.rs`) | No | They read the repository with tools; the on-device model has none, and the diff is up to 200k characters |
| Issue query, PR search, workflow change (`issue_queries.rs`, `review_searches.rs`, `workflow_changes.rs`) | No | Strict JSON or a whole workflow file; opus today |

## What adopting the two trials would take

- **A second `ModelClient`** whose `render` names the helper as its program, behind a router that sends only these sites to it.
  `Model::named_at` checks `models.account` against a roster, so a new alias would need to pass it; this was not checked.
- **The helper as a shipped binary**: built on the Mac from the Command Line Tools, or prebuilt and signed in the app. It reads the prompt on stdin and prints the answer, as the Claude CLI does.
- **An availability probe.** `availability` can be false (Apple Intelligence off, model not ready, device not eligible).
  The owner's rule is no Claude tokens, so the fallback is to keep the last line and write none.
- **A response cap and a timeout** on every call, 200 tokens at most.
- **A long-lived helper** only if the 0.5 s of process start matters; at 100 calls an hour, it does not.

## What was not measured

- Quality against Claude's, on any prompt: no Claude call was made.
  The gradings are mine, on 40 and 15 answers.
- The quick chips, the dispute read and the wait read on the model at all.
- Guided generation (`@Generable`), which could fix the format and may not fix the content.
- The system process's memory, CPU and energy; an idle gap over 10 minutes; a first call after reboot.
- Whether the Swift helper survives being started from Fleet's cleared environment (`env_clear()` in `running.rs`).
