# Acts on a pull request

**Part of:** [Job](../session.md).

A pull request a job holds is a `pr` row, and a person can act on it without leaving the job. The pull request is named by its repository and number.

| Operation | Does | Refuses |
|---|---|---|
| `get_pull_request` | Reads it: draft, open, merged or closed, auto-merge, the forge's checks (`pending`, `passed`, or `failed` with the failing names), title, branch, address | Where the forge would not answer |
| `ready_pull_request` | Takes a draft out of draft | Anything that is not a draft |
| `merge_pull_request_by_number` | Merges it | While the checks are running or failed, a draft, a closed one, and the forge's own refusals |
| `enable_auto_merge` | Asks the forge to merge when the checks pass | Once they have passed or failed, a draft, a closed one |
| `get_pull_request_detail` | Reads the page the panel draws: base, author, mergeability, review state, the conversation, the forge's checks with their run time, files and commits, and the head, the description and the diff's review threads | Where the forge would not answer, and where it would not give the threads |
| `get_pull_request_file` | One file's patch, read when a person presses a row of the Files tab. Bridge draws it in a panel to the left of the pull request's | Where the forge would not answer, and for a file the pull request does not list. A file the forge sent no patch for answers with Fleet's sentence |
| `write_pull_request` | Comments, approves or requests changes, then reads the page again. With line, file and reply comments it is one review | A comment or a request with no body, any comment with a blank body, a Job the ledger never heard of, and the forge's own refusals |
| `resolve_review_thread` | Resolves a conversation on the diff, or opens it again. Bridge only, see `docs/concepts/pull-review.md` | The forge's own refusals |
| `get_review_lists` | Reads the pull requests waiting on a person, as four lists. Bridge only, see `docs/concepts/pull-review.md` | A forge that would not answer any list |

```
 a person presses       Fleet                                  the forge
 ───────────────        ──────────────────────────────────     ─────────
 merge ───────────────▶ read state + checks ────────────────▶ gh pr view, gh pr checks
                        checks not passed ─▶ refuse, no write
                        passed ────────────────────────────▶ gh pr merge --merge
                        read again, refresh every `pr` row   ◀─
 ◀─ the new state       merged ─▶ row `spent`, closed ─▶ `given_back`
```

> **Rule.** Every write is read first, and a refusal Fleet can name is named before the forge is touched.
> Why: a merge pressed while the checks run must reach nobody, and a count of what the forge was asked to write is the only proof it did not.

> **Rule.** The ledger follows the forge through its own attach.
> Why: a Job's `pr` row is what the harness reported, and `detail` (`state`, `auto_merge`, `checks`, `failing`, `title`, `branch`, `address`, and `body` where the author wrote one) is Fleet's reading of the same pull request, so one writer is not two.

**Only a failed required check is `checks: failed`.** Where a repository names a gate (`ci`), a check outside it that is red while the gate runs reads `pending`, and one red after the gate passed reads `passed`, with its name still in `failing`. Bridge draws those two as a tinted chip and leaves the Job's row alone; only `failed` puts a Job under Idle with a red mark, and a Job that waits on the person is under Needs you whatever its checks say. Fleet refuses a merge on all three.

**The three writes are a person's ask, `pushes to shared`.** The operations are `Helm only`, and Helm calls them on a person's ask. Bridge's own buttons are the person's press and reach the routes directly. `get_pull_request` is open to any agent.

**A press is drawn as underway until Fleet has answered and the pull request has been read again.** The pressed button takes Button's pending rendering with a turning ring before its label, the other acts are disabled so a second cannot start, and a refusal is said above them and the button returns. Merge, Enable auto-merge and Ready for review show the state the re-read brings.
