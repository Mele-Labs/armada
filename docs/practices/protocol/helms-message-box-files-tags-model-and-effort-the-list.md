# Helm's message box: files, tags, model and effort, the `/` list

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**Retired 13 Oct 2026 but for `AskHelm`'s `attachments` and `mentions`, `get_issue` and `TagKind::Issue`.** `tune_helm`, `HelmBoxFacts`, the socket's `facts` message and `get_helm_file` are gone: the box is a Job's, tuned with `tune_session` and fed by `session.hosted`. See *Helm is a Job*.

`docs/concepts/helm.md`, *The message box*, and `docs/concepts/session.md`, *What a message names*. **Additive only**: three operations, all `agent_access` `No` and Bridge's, one `TagKind`, and optional fields on existing DTOs. Two required fields are new, `HelmOpened.facts` and the `facts` message, which a Bridge from before them cannot read; the ID refuses that pairing, so nothing meets it half built. Bridge's half is `packages/protocol/src/helm.ts`, `helm-thread.ts`, `hosted-sessions.ts` and `issues.ts`, written by hand.

| Where | Carries | Notes |
| --- | --- | --- |
| `AskHelm` (`ask_helm`) | `attachments[]` of `SessionUpload`, `mentions[]` of `SessionTag`, beside `text` and `context` | `text` is a plain string, no longer a type that refuses blank (`AskCommander` keeps `HelmText`), and may be blank when something is carried; blank words with nothing carried are 422 `fleet.helm_message_empty`. Uploads are kept as a Job's are, in a directory of the conversation's own, and the refusals are the same |
| `ThoughtSent` (`record_thought_sent`), `HelmSent` (`sent` message) | `ThoughtSent` gains `attachments[]` and `mentions[]`; `HelmSent` gains `files[]` of `SentFile` and `tags[]` of `SessionTag` | A thought routed to a Job keeps what went with it on its `sent` row, so the chips survive a restart. Fleet keeps the uploads in Helm's own directory as an ask's, and `get_helm_file` serves a `sent` row's files too. `tell_session` records words only, so both lists are empty |
| `HelmAsked` (`asked` message) | `files[]` of `SentFile`, `tags[]` of `SessionTag` | A row of the thread's file, so a reloaded Bridge draws what each message carried |
| `get_helm_file` (`GET /helm/file?manifest_id=&file=`) | The bytes of a `SentFile.id` from an `asked` row, under the media type they were sent as | `get_session_file`'s shape and refusals |
| `tune_helm` (`POST /helm/tune?manifest_id=`) | `TuneHelm`: `model?`, `effort?`. Answers `HelmBoxFacts` | Absent or blank is the machine's own. 422 `fleet.unacceptable_effort` for anything but `low`, `medium` or `high`. Kept per conversation in the store, passed at the launch of every message, kept through Start fresh |
| `HelmBoxFacts` | `model?`, `effort?`, `commands[]` | `commands` are the agent's own `init` names, else the last any Job read |
| `HelmOpened.facts` | `HelmBoxFacts`, always present | What a viewer starts from |
| `observe_helm` message `facts` | `HelmBoxFacts` | **Live only**: never in the backfill and never written to the thread's file. Sent when the tuning or Helm's own commands change; a change in the Jobs' list is read at the next `opened` |
| `TagKind::Issue`, `SessionTag.url?` | `issue`; the forge address of an issue or pull request tag | Fleet writes `issue #214 "title" (address)` for the agent, and a leading `#N ` in a title is dropped from the line |
| `get_issue` (`GET /issues/:repository/:reference`) | Answers `IssueRead`: `reference`, `title`, `url`, `state` (`open` or `closed`) | 404 `fleet.no_such_issue`, and a pull request's number is not an issue; 500 `fleet.issues_unreadable`; 422 for a repository Fleet does not serve |

**Start fresh deletes the conversation's uploads**, with the thread that named them. The turn a words-only ask writes is unchanged; an ask with a picture is written as a block list, as a Job's is.

**The event stream.** `facts` rides the conversation's own bounded, drop-oldest channel with its `missed` count, and costs a viewer nothing it cannot recover: the next `opened` carries the current facts. It adds no queue to `/events`, so it leaves the unmeasured sink risk where it was. Files are not on a stream at all.

**Skew.** None to meet: a refused ID mismatch means no peer sees this half built. A file's bytes are read once by a picture's thumbnail and held by the window.
