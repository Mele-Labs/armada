# Pull review

**What it is:** Bridge's Reviews tab: the pull requests waiting on a person, a page for each, and the review he writes on one.

---

**Kind:** Surface.

Reviews reads the forge through Fleet. Everything an agent would do on a pull request (quick actions, Ask, tools, Natural Search) is a mock until runs land, and sits behind one seam in Bridge.

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

**A refused review changes nothing.** His drafts stay, and the forge's words are shown beside Submit.

**Resolve and unresolve act on a thread at the forge.** A refusal leaves the thread as it was.

## Where it lives

| What | Where |
|---|---|
| Wire types | `crates/ipc/src/pull_reviews.rs` |
| Lists route | `crates/ipc/operations/get_review_lists.toml` |
| Resolve route | `crates/ipc/operations/resolve_review_thread.toml` |
| Page and review | `crates/ipc/operations/get_pull_request_detail.toml`, `crates/ipc/operations/write_pull_request.toml` |
| Lists in Fleet | `crates/fleet/src/pull_review_lists.rs` |
| Store | `apps/desktop/src/renderer/src/pr-review/store.tsx` |
| Mock agent half | `apps/desktop/src/renderer/src/pr-review/store-agent.ts` |
| Keys | `review_*` rows of `crates/core-model/domain/actions.toml`, scope `review` |
