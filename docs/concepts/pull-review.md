# Pull review

**What it is:** Bridge's Reviews tab: the pull requests waiting on a person, a page for each, and the review he writes on one.

---

**Kind:** Surface.

Reviews reads the forge through Fleet, and what an agent says about a pull request (the explanations, Ask and the review tools) is a run in Fleet. Natural Search is the one piece still a mock, and sits behind one seam in Bridge.

## The four lists

| List | Holds |
|---|---|
| Requested for Review | Pull requests that name him as a reviewer |
| Team | Pull requests that asked a team he is on |
| Assigned | Pull requests the forge assigned to him |
| My PRs | Pull requests he or an agent of his opened |

**Identity is the logins `crates/fleet/src/my_pulls.rs` already uses.** A pull request on two lists is one row that knows both.

**The lists are fetched when the tab opens and when Refresh is pressed.** Nothing polls.

**Each list holds at most the wire's `MOST_PER_LIST` rows**, newest change first.

**A list the forge would not answer is named in `silent`.** Bridge keeps what that list held and replaces the others, so one failure does not blank a list.

## A page

**A page is the pull request's description, conversation, checks, files and commits, with its review threads and head commit.** It is read when the pull request is opened, and again when the lists are refreshed while it is open.

**A file's patch is read the first time its change is shown.** The forge numbers a line by path, line and side; the diff draws no numbers, so `apps/desktop/src/renderer/src/pr-review/patch.ts` is the one place the two meet.

## A review

**Drafts are held in Bridge until Submit.** A draft is a line comment, a comment on a file, or a reply to a thread.

**Submit posts the verdict, its summary and every draft as one review at the commit he read.** Comment, Approve and Request changes are the three verdicts. Helm still asks before it writes.

**A comment with a blank body is refused before the forge**, as a plain comment is.

**A refused review changes nothing.** His drafts stay, and the forge's words are shown beside Submit.

**Resolve and unresolve act on a thread at the forge.** A refusal leaves the thread as it was.

## Agent runs

**A run is an agent's read-only turn about one pull request, and only a press starts one.** Listing or opening a pull request spends nothing. A run is one of three kinds:

| Kind | Asked with | Answers |
|---|---|---|
| Explanation | How it works, User Impact, How to Test, Why, or one a person added | Themes, before and after, steps, or the problem and its cost. An explanation added by a person answers text |
| Tool | Risks, Possible bugs, API breaking changes, or one a person added | Findings, each pinned to a path and line range with a severity, title and body |
| Ask | A question in his words | Markdown |

Comments is not a run. It is the forge's conversation.

**What the agent is given:** the definition's prompt, the pull request's title, description, head commit and diff, and a checkout of the head to read. **It can read and nothing else**: Fleet leases a bay (a slot of the repository's pool) on a branch of its own at the head, makes one model call with the read tools only, and gives the bay back clean, so a run holds no shell, edits nothing and reaches no network. A full pool leaves the run `queued`. The model is `models.review`, which ships as the Session model and is its own setting after that.

**The answer is JSON in the shape the kind has, and is checked before it is kept.** A finding or a change names a file the pull request touches and a range from line 1. A How to Test step names where it came from (the description first, then a README or an agent file in the checkout, up the tree from a changed path), and a doc it names must exist at the head. A Why link is an http or https address. **An answer that fails any of this is a failed run with its reason, and is never a result.** A failed run does not count as kept, so pressing again runs again.

**A result is kept for the commit it read.** It is keyed by repository, pull request, head commit and kind and name. Pressing a run that has a kept result shows it with no run and no spend. A pull request whose head moved still shows the old result, with the commit it ran on, and Bridge marks it out of date. Run again is the one press that spends on a commit that already has an answer. A run's cost is kept with it.

**Progress is three small events.** A run is `queued`, `running`, then `done` or `failed`, and each change arrives as `review_run.changed` carrying the run whole. Nothing streams, and the text of an answer is never sent in pieces. A reconnecting Bridge reads the runs again, since the event is not replayed. A run the last Fleet left going ends failed the first time anyone asks, and its bay is given back.

## Review tools

**A review tool or explanation is one file: its kind, its name and its prompt.** What shape its answer takes is not the file's to say. A tool answers findings, the four explanations Armada carries answer the shape their id has, and any other explanation answers text.

| Place | Whose | Wins over |
|---|---|---|
| Armada's own | Carried in the binary | |
| `~/.armada/review-tools/` | His, in every repository on the machine | Armada's |
| `<repository>/.armada/review-tools/` | The repository's, for everyone on it | Kit's and Armada's |

**The name lower cased with its words joined by a dash is the id**, so two places define the same thing by naming it the same. A file that does not fit (no front matter, an unknown key, no prompt, two files of one id in one place) is left out and named, and the rest stand. The Add sheet writes one file into Kit or the repository. A save that would replace a file of the same id is refused unless the press said to overwrite.

## Send to a Session

**A pull request a Session of his holds says so on its row**, from the same owners data the pull request panel uses. Send to Session and Request changes to Session go to that Session. A note on a pull request with no holding Session stays in Armada.

## Where it lives

| What | Where |
|---|---|
| Wire types | `crates/ipc/src/pull_reviews.rs` |
| Lists route | `crates/ipc/operations/get_review_lists.toml` |
| Resolve route | `crates/ipc/operations/resolve_review_thread.toml` |
| Page and review | `crates/ipc/operations/get_pull_request_detail.toml`, `crates/ipc/operations/write_pull_request.toml` |
| Lists in Fleet | `crates/fleet/src/pull_review_lists.rs` |
| Store | `apps/desktop/src/renderer/src/pr-review/store.tsx` |
| Runs in Bridge | `apps/desktop/src/renderer/src/pr-review/store-agent.ts`, `runs.ts` |
| Run and definition wire types | `crates/ipc/src/review_runs.rs`, `packages/protocol/src/review-runs.ts` |
| Run routes | `crates/ipc/operations/start_review_run.toml`, `get_review_runs.toml`, `review_run.changed.toml` |
| Definition routes | `crates/ipc/operations/list_review_tools.toml`, `save_review_tool.toml` |
| Run engine | `crates/fleet/src/review_runs.rs` |
| Definitions | `crates/config/src/review_tools.rs`, `crates/config/review-tools/*.md`, `crates/fleet/src/review_tools.rs` |
| The bay | `Vcs::lease_review_bay` and `release_review_bay`, in `crates/adapters/src/leasing/jobs.rs` |
| Kept results | `crates/store/src/review_runs.rs` |
| Keys | `review_*` rows of `crates/core-model/domain/actions.toml`, scope `review` |
