# Protocol 14.17: what a Link's address names on the forge

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`StudioNodeContent::Link` gains `forge`, optional: `issue`, `pull_request` or `milestone`, and
absent where the address names nothing on the forge. `#1379`.

**Read off the address by Fleet on every Studio it sends, and never kept on the record.** Which
host is the forge is `crates/adapters`' to know — `verify-foundations` refuses the vendor's name
anywhere else, Bridge and `crates/ipc` included — so a rule about issue links could not be written
in TypeScript at all. This field is how Bridge knows to offer Dispatch on a Link naming an issue
without reading one.

`dispatch_studio_draft` takes such a Link as well as an Issue draft, and sends the address as the
request. **No shape moves on that route**: the request already carried `node_id` and `position`,
the gate is the same gate, and the Job node lands with a `produced` edge from the node it came
from either way. A Link naming anything else is refused as `fleet.studio_not_a_draft`, the code
that route already had.

**Additive on both counts.** An older Bridge reads a Link with no `forge` as the Link it always
read, and an older Fleet is refused by the skew rule as it always was.

**Superseded at 14.18**, which makes what an address names the node's own kind and drops this
field.
