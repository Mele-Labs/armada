# How a job gets in

**Part of:** [Job](../session.md).

The harness sends Fleet facts in Armada's own shape and nothing past the adapter that sends them knows what the job runs in.

```
 harness (mod in a terminal session)          Fleet
 ───────────────────────────────────          ─────────────────────────────
 session starts ── started ───────────────▶  POST /sessions/report
 git, gh, dispatch, subagent, message ────▶   stamp, resolve cwd → repository, slot
 turn ends, usage measured ───────────────▶   keep in sessions, ledger_attachments
 session ends ─── ended ──────────────────▶   publish session.changed, whole
```

| `fact` | Carries | Fleet does |
|---|---|---|
| `started` | `cwd`, `title?`, `origin?`, `mod_version?` | Creates the row or starts a resumed one again; resolves the directory to a repository and a pool slot |
| `titled` | `title`, `named?` | `named` is a person's `/rename` in the terminal and replaces the title; without it, the first prompt's line, kept only where there is no title |
| `moved` | `cwd` | Re-resolves; a slot the job left is given back. A slot whose lease is another Job's is not taken |
| `attached` | `attachment`: `kind`, `target`, `detail?` | Takes it; `branch` and `slot` replace the one held. A `branch` or `pr` read from a slot whose lease is another Job's is dropped (*Whose slot it is*) |
| `settled` | `attachment`: `kind`, `target`, `state` | Moves it to `spent` or `given_back` |
| `measured` | `usage`: `context_tokens?`, `context_window?`, `cost_micros?`; `plan?`: the plan's windows; `context_breakdown?`: what fills the window | Keeps what was reported. The breakdown is kept whole in `session_context`, one row a job, and replaced by the next; a report without one leaves it. The windows are the account's, so Fleet keeps the newest from any job as `get_plan_usage` (`machine.md`, *Quota*) and keeps nothing of them on the job |
| `tuned` | `model?`, `effort?`, `mode?`, `commands?`, `mod_version?` | Held in memory as the terminal's `terminal` facts: model, effort, mode and the commands it lists |
| `turn_started` | nothing | A prompt went in (the mod, never for a slash command, which most of the time ends without a turn): the job is working, and the reply the turn before left is over (*What a Job is doing*) |
| `turn_completed` | `said?`: the first line of what the turn ended on | Stamps the last turn, lets go a hold for the plan's limit, and keeps `said` as the reply (*What a Job is doing*). Marks every window the job showed ready, unless a `subagent` of it still stands (*Showing a window*). The mod does not tell a turn that died on an API error |
| `limited` | `resets_at` | Holds the job until the plan's limit resets (*A job that hit the plan's limit*) |
| `wrote` | nothing | The job itself wrote a file (the mod, never for a subagent): marks every window it showed as still changing |
| `permission_asked` | `tool`, `detail?` | A terminal's harness opened its own permission prompt (the mod, from Claude Code's `PermissionRequest` hook): a `permission` call until it closes (*A terminal's permission prompt*) |
| `permission_settled` | nothing | The tool call that raised the prompt ended: the call goes |
| `touched` | `paths`: files written, from the job's directory | Checks them against who else writes them (*Write clashes*); a terminal Job's are counted for the timeline as `session.edited` |
| `read` | `path`: a file read, from the job's directory | Counted for the timeline as `session.read`, for a terminal Job; nothing on the ledger changes |
| `ran` | `command`: the first line of a shell command, 300 characters at most | Counted for the timeline as `session.ran`, cut and with what looks like a secret in it hidden; nothing on the ledger changes. A command whose first word is `gh` is entered as `session.called` instead, as its subcommand words only |
| `tasks` | `tasks`: the agent's whole list, each with `id`, `text` and `status` (`pending`, `in_progress` or `completed`) | Keeps the list and when each item was first seen completed (*The build record*). An empty list clears it |
| `plan_left` | nothing | The agent left plan mode (*The build record*) |
| `committed` | `branch?` | A `git commit` printed a commit made. Entered on the timeline as `session.committed` |
| `pushed` | `branch?` | A `git push` printed a ref updated. Entered on the timeline as `session.pushed` |
| `ended` | `reason` | Ends it and gives back everything it still holds. A terminal's slot is parked unless the reason is `clear` or `resume` (*Ending a terminal job*) |

**No instant is reported**: Fleet stamps each fact on arrival. **A repeated fact publishes nothing**, so a harness may report on every turn.

The mod for Claude Code is `plugins/armada/`, and `crates/adapters` is the only Rust that names the harness.
It sends no message text and no prompt apart from the first line of real text in the first, as a title, the first line of what a turn ended on, as the reply (*What a Job is doing*), and the first line of each shell command, for the timeline (`docs/practices/protocol.md`, *The Fleet's timeline*): the harness's wrapper tags (`<agent-message …>`, `<command-…>`, `<system-reminder>`) are taken off first, and a prompt with nothing left has no title.
A `/rename` in the terminal is reported as the name typed, or, for a bare `/rename`, the name Claude Code wrote to the transcript.
**A Job's name can also be changed from Bridge**, by pressing its title in the Job view: `rename_session`, hosted or terminal, which stands until the next terminal `/rename`.
A hosted Job's harness is not told, so its own name for itself stays what it was.
**The mod also sends the text of each item on the task list the agent wrote**, read from the task tools it already calls (*The build record*).
