# Protocol 23.12: a Job's retro, and who acted

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 3 Oct 2026: every Job reflects when it ends. `../concepts/retro.md` is the concept.

**Two routes, one optional field, two enums, all additive.** `Recorded` gains `via`, the door the
request that made a move came through, absent on a move Fleet made on its own. Bridge sends
`x-armada-caller: bridge` on every request, and Fleet reads a request without it as not Bridge's.

| Route | Answers |
| --- | --- |
| `GET /jobs/:job_id/retro`, `get_job_retro` | `JobRetro`: `state`, `items`, `record`, `annotations` |
| `GET /lessons?manifest_id=&most=`, `list_lessons` | `Lessons`: items across Jobs, newest retro first |

**A person's act from any door but Bridge is signed `helm`.** `actor` keeps its set: an agent made
the request and Fleet cannot see the person behind it. A Bridge before 23.12 sends no header, so
behind a 23.12 Fleet its presses read `helm` with `via: http` until it is rebuilt.

**`submit_evidence` takes an optional `in_the_way`.** It is an MCP tool, not this protocol, and its
own schema says so. **Store V100** keeps each move's door, a Drone's notes and the retro, and marks
every Job already ended `skipped`.
