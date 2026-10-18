# Protocol 16.3: the web app, started from a Studio

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1345`. `start_studio_server`, `POST /studios/:studio_id/start_server`, carrying
`StartStudioServer { name, position, produced_by? }` and answering `StudioServerStarted { studio,
node_id, server, already_up }`. `Helm only`, as `start_studio_run` is. Additive: a new route, a new
pair of DTOs, and one new optional field.

**Beside `start_studio_run` rather than inside it**, as `start_server` is beside `start_run`
everywhere else. A server has no exit until something stops it, it is read back by `list_servers`
and `observe_server`, and it is ended with `stop_server` — three different readers from a run's. A
single call that could answer with either would hand a caller an id and leave it to work out which
reader to ask, which is exactly the confusion the new field below exists to prevent.

`StudioNodeContent` for kind `run` gains `held`, absent or `checkout` or `server`. **Absent is a run
in the checkout**, which is every node written before this, so nothing an older Bridge already reads
changes. A Bridge that does not know the field draws a server node as an unread run — its id as a
fact, and the word *Not read yet* — which is what that build could truthfully say about an id it has
no reader for.

**A server is a Run node and not a fifteenth kind.** `docs/concepts/studio.md` says a Run node holds
a Manifest command started from the Studio, and a Command with `serve` is one; Run is also one of
only two kinds that may take status colour, which *starting* and *serving* need. A new kind would
have meant a `studio_nodes` `CHECK` rebuilt, a fifteenth row in the kinds registry, and that colour
rule amended — three costs for a distinction the content already carries.

`StudioRunKept` is unchanged and a server's fills it: `command` is the `serve` line as it ran,
`duration_ms` is how long the instance was up, and `expect_exit_code` is written but not read,
because a server that exits on its own has failed whatever its code.

**This was written as 16.2 and is 16.3**, because `#1439` took 16.2 underneath it while the branch
was open — the third time this file records that collision, and caught at the merge again.
