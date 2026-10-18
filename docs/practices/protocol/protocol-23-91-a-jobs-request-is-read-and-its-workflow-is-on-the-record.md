# Protocol 23.91: a Job's request is read, and its workflow is on the record

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A Job's proposal*, and `docs/concepts/job-proposer.md`, *A Job's request*.
**Additive only**: one optional field on a request, one operation, two fields on a record, three timeline kinds.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_session` | `StartSession` gains optional `request` | The words the person dispatched. Fleet reads them once with the Job proposer's model and fills in the `title`, `model` and `effort` the start left unset, and the Job's `workflow`. **The answer is the `SessionRecord` at once**, with `proposal.reading` true and no process; the process starts when the answer lands. Not read with `pilot`, `fork` or `root`. A call that fails or cannot be read starts the Job on the machine's defaults and nothing is refused |
| `set_session_workflow` (`POST /sessions/workflow`) | `SetSessionWorkflow`: `session_id`, `workflow?`. Answers 200 `SessionRecord` | Sets the workflow a hosted Job follows, or clears it (absent) so it is a custom Job. Fleet tells the agent at once and enters `session.workflow`. 422 `fleet.session_workflow_not_held` for an id the repository does not hold, 422 `fleet.session_command_refused` for a terminal Job, 409 `fleet.session_closed` for one that ended. `agent_access: No` |
| `SessionRecord` | `workflow?`, `proposal?` | `workflow` is absent for a custom Job. `proposal` is `{ reading, picked }`, each skipped where false or empty; `picked` lists `title`, `workflow`, `model` and `effort` as the proposer filled them, and a field leaves it when the person sets it |
| `timeline.entry` | `TimelineKind` gains `session.workflow`, `session.model`, `session.effort` | `says` is `workflow: <old> → <new>`, `model: <old> → <new>` or `effort: <old> → <new>`, naming a workflow by its name and none as `custom`, a model or effort not set as `the default` |
| `tune_session` | Behaviour only | A changed `model` or `effort` is now told to the agent on the next turn that starts on it, and entered as above. No wake |

**Skew.** A Fleet before 23.91 ignores `request` and answers 404 for `/sessions/workflow`; a Bridge before it never sends either.
A Bridge before reads a `session.*` timeline kind it does not know and draws nothing or draws it badly, and ignores the two record fields.
The IDs differ, so the two are refused regardless and the lifeboat serves.
Bridge's half is `packages/protocol/src/{hosted-sessions,sessions,timeline}.ts`, written by hand like the rest.
