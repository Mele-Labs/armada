# Spike 29 — Why a Session's auto asked about everything, and how its question is answered

**Three findings, measured on 7 Oct 2026 against Claude Code 2.1.293 with `--model haiku` and
`--setting-sources project,local`, in a scratch repository, with my own probe processes.**

1. **The answer shape was never the problem; the reading of the call was.** The CLI hands the
   permission tool only what its own rules do not cover, and Fleet's `auto` then asked about two things
   that are not the owner's three classes: **every overwrite of a file** (Helm's 17 Sep rule, carried over
   to Sessions unchanged) and **any line with a redirect into `/dev/null`**, a command substitution or a
   heredoc.
2. **`AskUserQuestion` reaches the permission tool, and the tool answers it.** An allow whose
   `updatedInput` carries an `answers` map is what the tool runs with. No hook is needed.
3. **A slot is under the main checkout's path.** The write gate's "inside the repository" test was true
   for `.armada/slots/slot-4`, so a slot the Session's own subagent had leased read as the checkout.

## 1. Which calls reach the door

A stub permission tool (`029-permission-tool-stub.py`) logged every call the CLI put to it in
`--permission-mode default`, which is what `auto` and `ask` launch as (spike 25).

| Call | Reached the permission tool | Fleet's reading before | After |
|---|---|---|---|
| `Read` of a file | no, the CLI allows it | n/a | n/a |
| `Grep` | no | n/a | n/a |
| `git status`, `ls` | no, the CLI reads them as read-only | n/a | n/a |
| `cargo build` | yes | runs | runs |
| `Edit` of an existing file in the Session's slot | yes | **asked** (an overwrite is destructive) | runs |
| `ls 2>/dev/null`, `cargo build 2>/dev/null` | yes where not read-only | **asked** (a truncating redirect) | runs |
| `git commit -m "$(cat <<'EOF' ...` | yes | **asked** (a command substitution is unreadable) | runs |
| `git checkout -b x`, `git stash list` | yes | **asked** (`checkout`, `stash` are destructive words) | runs |
| `rm -rf`, `git reset --hard`, `git push`, `gh pr merge`, `cargo publish`, `curl -X POST`, `gh issue create` | yes | asked | asked |

I could not make a `Read`, a `Grep` or a plain `git status` reach Fleet at all, so if the owner was
prompted for those the prompt did not come from Fleet's door. **I did not find it**; what is certain is
that every `Edit` and every redirect to `/dev/null` asked, which on a working Session is most of what is
done. The fix is `fleet::helm::because_in_a_session`: the same three classes, with an edit inside the
Session's own slot running, `/dev` redirects not truncating, and a line read through its substitutions
with heredoc bodies cut out. `eval` and `exec` still ask.

**A hook's `allow` does skip the permission tool.** A `PreToolUse` hook answering
`{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}` ran `cargo build`
and an `Edit` with the permission tool never called (its log was not written). It was not used: the door
is the one place the three classes are decided, and an allow from the hook would be a second place that
also has to know them. It is the route to take if the round trip through the door ever measures slow.

## 2. AskUserQuestion

```
agent ── AskUserQuestion{questions} ──▶ CLI ── permission tool (tool_name, input) ──▶ armada mcp ──▶ Fleet
                                                                                                  │ holds the call,
person ◀── Bridge draws the questions on the Session's thread ◀── hosted.asked{questions} ◀───────┘ raises an ask
person ── answer_session_ask{answers:[{question, chosen:[…]}]} ──▶ Fleet
Fleet ── {behavior:"allow", updatedInput:{questions, answers:{"<question>":"<labels joined by ', '>"}}} ──▶ CLI
agent ◀── "Your questions have been answered: "Which size?"="L"" ◀── tool result
```

Measured with the stub answering the three shapes:

| Answer given in `updatedInput.answers` | Tool result the model saw |
|---|---|
| `{"Which color do you prefer?": "Red"}` | `Your questions have been answered: "Which color do you prefer?"="Red".` and the model replied `Red` |
| a multi-select, `"cheese, ham"` | `"Which toppings would you like?"="cheese, ham"` |
| free text for a single-select, `"my own words"` | `"Which size would you like?"="my own words"` |

The tool's input is `{questions: [{question, header, options: [{label, description}], multiSelect}]}`,
at most four questions of two to four options each. **Before this change Fleet allowed it with the input
unchanged**, so the tool had no `answers` and failed. Now `session_permission` holds it whatever the
mode, offers *Answer* and *Skip* and no remembered rule, refuses an answer that leaves a question
without anything chosen, and returns the input with `answers` filled in. *Skip* is a deny the agent is
told about. Free text (*Other*) is one more entry in `chosen`; Fleet does not tell it from a label.

Protocol **23.66**, additive: `HelmCallInFlight.questions?`, `AnswerSessionAsk.answers?`,
`AnswerHelmCall.answers?`. `docs/practices/protocol.md`.

## 3. The write gate and a slot the agent leased

`armada worktree lease` records its holder as the first process above it that is not a shell
(`Holder::the_caller`): for a Bash call that is the agent's own process, whichever subagent asked. The
gate now places each path as main checkout, slot *n*, or elsewhere, and lets a write through in any slot
whose record names the Session's process or a process it started (`peer::started_within`, the same walk
as a connection's). `docs/concepts/session.md`, *What the write gate covers*, says what a shell line
is and is not held on.

## Provenance

`029-permission-tool-stub.py` is the stub. Each probe was `claude -p --model haiku --setting-sources
project,local --strict-mcp-config --mcp-config <stub> --permission-mode default
--permission-prompt-tool mcp__stub__approve` in a scratch repository, with a prompt naming the calls to
make; the hook probe added `--settings` with an `http` hook on `Write|Edit|Bash` answering `allow`. None
reported into a Fleet. The tests that hold the findings are `fleet::tests::session_auto`,
`session_question` and `session_gate`.
