# Pull review

**What it is:** Bridge's Reviews tab: the pull requests waiting on a person, a page for each, and the review he writes on one.

---

**Kind:** Surface.

Reviews reads the forge through Fleet, and what an agent says about a pull request (the explanations, Ask and the review tools) is a run in Fleet.
Natural Search is a run across every repository, and the Walk tool is a run that needs to know where the change runs.

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

**What the agent is given:** the definition's prompt, the pull request's title, description, head commit and diff, and a checkout of the head to read.
**It can read and nothing else**: Fleet leases a bay (a slot of the repository's pool) on a branch of its own at the head, makes one model call with the read tools only, and gives the bay back clean, so a run holds no shell, edits nothing and reaches no network.
A full pool leaves the run `queued`.
The model is `models.review`, which ships as the Job model and is its own setting after that.

**The answer is JSON in the shape the kind has, and is checked before it is kept.** A finding or a change names a file the pull request touches and a range from line 1. A How to Test step names where it came from (the description first, then a README or an agent file in the checkout, up the tree from a changed path), and a doc it names must exist at the head.
A Why link is an http or https address.
**An answer that fails any of this is a failed run with its reason, and is never a result.** A failed run does not count as kept, so pressing again runs again.

**A result is kept for the commit it read.** It is keyed by repository, pull request, head commit and kind and name.
Pressing a run that has a kept result shows it with no run and no spend.
A pull request whose head moved still shows the old result, with the commit it ran on, and Bridge marks it out of date.
Run again is the one press that spends on a commit that already has an answer.
A run's cost is kept with it.

**Progress is three small events.** A run is `queued`, `running`, then `done` or `failed`, and each change arrives as `review_run.changed` carrying the run whole.
Nothing streams, and the text of an answer is never sent in pieces.
A reconnecting Bridge reads the runs again, since the event is not replayed.
A run the last Fleet left going ends failed the first time anyone asks, and its bay is given back.

## Natural Search

**Natural Search is the last entry of the list dropdown, and it opens a box in place of the list.** Sending it asks first, with the same box every act that spends asks with.
Bridge reads the searches kept when the tab opens and on Refresh.
A pull request a search turns up opens like any other, even where it is on none of the four lists.

**A search is one run of kind `search` over every repository Fleet serves, and only a press starts one.** The words are his.
Fleet lists the open pull requests (the newest forty, one forge call), reads the files each changes, and reads the paths he has touched from each repository's own history under his logins and the repository's configured identity.
It makes one model call, `models.review`, **with no tools at all**: the pull requests' titles, branches and file names were written by other people, so they are handed over as data and there is nothing for their words to reach.

**Every match is held to what Fleet gathered.** An answer is `{"matches": [{repository, number, reason}]}`.
A match that names a pull request that was not in the open set, names one twice, has no reason, or has a reason that is not one line of at most 200 characters fails the run with that reason, and no match is shown.
An empty list is an answer.
Nothing open is an answer too, with no call.

**A search is kept under his words, lower cased and single spaced, and has no out-of-date mark.** Pressing the same words shows what is kept with no run and no spend.
Run again is the one press that spends.
A kept search is dropped when he removes it, and Fleet drops the row, so it is gone after a restart too.
One still going is not stopped; it finishes and is kept again.
A search the last Fleet left going ends failed the first time anyone asks.
Progress is `review_run.changed`, as for any run.

## Walk

**The Walk tool is a review tool in the Flight control, and it asks first like the others.** It is a run that writes a walk against a place.
The place is where the change runs: a web address, an API base, or both.
It is Fleet's and never the agent's.
Fleet validates it before the run starts, puts it in the brief, and writes it into the kept result itself, so a walk cannot name a place it was not asked to write against.
The agent answers a web walk (steps the walk window plays, checked as `show_window` checks them) or an API walk (up to a hundred requests, each a method, a path on the base and what to look for in the response).

**Where the change runs is found in the person's order, and finding spends nothing.** Each source is tried only if the one before found nothing.

| Order | Source | Gives |
|---|---|---|
| 1 | The pull request | A deploy bot's comment naming a preview or deployed address, else the details address of a passed check named for a preview or a deploy. A place that is already running |
| 2 | What Fleet remembers | The place kept for this repository and the area of code the pull request changes. The area is the directory, two deep at most, with the most changed files, and a place kept for a directory above it covers it |
| 3 | Asking him | Bridge's. Nothing found leaves `place` absent and Bridge asks, and what he answers is remembered for the area unless he unchecks it |

**A place is deployed or pulled down.** A deployed one is already running and holds nothing.
A pulled-down one carries the command that starts it, which is his own text, typed in the sheet or remembered from it, and an agent never writes one.
Places are kept per repository and area, one each, and listed, edited and forgotten under "Where changes run", which `⇧W` opens.

**Walking a pulled-down place pulls the change into a leased bay.** Fleet leases a bay at the pull request's head, starts the command there as a server under the machinery `start_server` uses, and asks the address until it answers, for two minutes at most.
Progress is `review_walk.changed`: `checking_out`, `starting`, `ready`, then `closed`, or `failed` with the reason.
A walk that fails has already given its bay back when the event says so.
The walk then plays in Armada's real walk window, web walks on the page and API walks as requests sent to the base.

**A held bay always goes back.** Bridge closes the walk when its window closes, and a server that ends on its own gives the bay back.
The held bays are also written down (`review_walks`), and at the start of every review walk call Fleet gives back any held bay it is not serving, which is how a bay a dead Fleet held is found, and any held for more than a day since it was opened.
The server is stopped first, the bay released next and the row deleted last, so a Fleet that dies between them leaves the row for the next sweep.

**A walk note on a pull request no Job holds stays in Armada.** It is never a draft comment on the forge.

## Review tools

**A review tool or explanation is one file: its kind, its name and its prompt.** What shape its answer takes is not the file's to say.
A tool answers findings, the four explanations Armada carries answer the shape their id has, and any other explanation answers text.

| Place | Whose | Wins over |
|---|---|---|
| Armada's own | Carried in the binary | |
| `<repository>/.armada/review-tools/` | The repository's, for everyone on it | Armada's |
| `~/.armada/review-tools/` | His, in every repository on the machine | The repository's and Armada's |

**His own wins over the repository's**, so a teammate's commit never replaces a tool he set up.
Where Kit's hides a repository tool of the same id that says something else, `list_review_tools` carries `hides: "repository"` on it, so Bridge can mark it; two copies that are the same carry nothing.

**The name lower cased with its words joined by a dash is the id**, so two places define the same thing by naming it the same.
A file that does not fit (no front matter, an unknown key, no prompt, two files of one id in one place) is left out and named, and the rest stand.
The Add sheet writes one file into Kit or the repository.
A save that would replace a file of the same id is refused unless the press said to overwrite.

## Send to a Job

**A pull request a Job of his holds says so on its row**, from the same owners data the pull request panel uses.
Send to Job and Request changes to Job go to that Job.
A note on a pull request with no holding Job stays in Armada.

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
| Natural Search in Bridge | `apps/desktop/src/renderer/src/pr-review/store-search.ts`, `crates/fleet/src/review_searches.rs` |
| The Walk in Bridge | `apps/desktop/src/renderer/src/pr-review/store-walk.ts`, `walk.tsx`; the window is `apps/desktop/src/main/capture/` |
| Walk wire types | `crates/ipc/src/review_walks.rs`, `packages/protocol/src/review-walks.ts` |
| Remembered places | `crates/fleet/src/walk_places.rs`, `crates/store/src/walk_places.rs` |
| Run and definition wire types | `crates/ipc/src/review_runs.rs`, `packages/protocol/src/review-runs.ts` |
| Run routes | `crates/ipc/operations/start_review_run.toml`, `get_review_runs.toml`, `review_run.changed.toml` |
| Definition routes | `crates/ipc/operations/list_review_tools.toml`, `save_review_tool.toml` |
| Run engine | `crates/fleet/src/review_runs.rs` |
| Definitions | `crates/config/src/review_tools.rs`, `crates/config/review-tools/*.md`, `crates/fleet/src/review_tools.rs` |
| The bay | `Vcs::lease_review_bay` and `release_review_bay`, in `crates/adapters/src/leasing/jobs.rs (deleted)` |
| Kept results | `crates/store/src/review_runs.rs` |
| Search and Walk wire types | `crates/ipc/src/review_walks.rs`, `packages/protocol/src/review-walks.ts` |
| Search routes | `crates/ipc/operations/start_review_search.toml`, `get_review_searches.toml`, `forget_review_search.toml` |
| Search in Fleet | `crates/fleet/src/review_searches.rs` |
| Walk routes | `crates/ipc/operations/find_walk_place.toml`, `list_walk_places.toml`, `save_walk_place.toml`, `forget_walk_place.toml`, `open_review_walk.toml`, `close_review_walk.toml`, `review_walk.changed.toml` |
| Walk in Fleet | `crates/fleet/src/walk_places.rs` (finding and keeping places), `review_walk_runs.rs` (the tool), `review_walks.rs` (the bay and the sweep), `servers/launched.rs` (starting the server) |
| Kept places and held bays | `crates/store/src/walk_places.rs` |
| Keys | `review_*` rows of `crates/core-model/domain/actions.toml`, scope `review` |
