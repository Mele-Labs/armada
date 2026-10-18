# Protocol 23.36: a merge held behind a need

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`merge_pull_request` gains one refusal, `fleet.merge_waiting_behind`, a 409 carrying `refused:
waiting_behind`. A Job that declared a need on a file (#1059) is refused its merge while a need ahead
of it on the same file stands, and the message names what it waits behind: what each declarer said it
needed, its branch, and what it took. It is Fleet's own press that is held, so a repository that says
`merge_by: forge` meets it as one that says `push` does; a person pressing the forge's button goes
around it. The Job stays at `awaiting_review`, and a press made again once the need has landed or been
given back goes through. No shape moves.

**Minor because a refusal code added is additive**, for 14.14's reason. The call a Drone declares a
need through, `declare_scope` and the tasks of `record_plan` and `add_task`, is the Drone seam and not
this one: it takes an optional `needs`, which is why it moves no number here.
