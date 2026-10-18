# Other things specific to this seam

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**Bridge finds Fleet through a runtime file, not a fixed port.** The file
carries port, pid, and protocol version, and Bridge verifies the pid is still
alive before treating the port as live — a stale runtime file and a genuinely
unreachable Fleet look identical over a bare connection timeout, and the pid
check is what tells them apart. Any change to the runtime file's shape is a
protocol-adjacent change even though it never touches `ipc`: it's still a
contract two independently-versioned binaries agree on ahead of any
connection. Treat it with the same "what does an old reader do with an
unrecognized field" discipline as the DTOs.

**One route on the listener is not on this seam.** `/mcp` serves the Evidence
tool to a Drone — the only way a Job's work is ever reported. It shares the
port because a Drone reaches Fleet the same way Bridge does, and it shares
nothing else: the peer is a process Fleet itself spawned, the vocabulary is
MCP's rather than `ipc`'s DTOs, and the version negotiated is the MCP revision
the client asks for rather than `protocol-version.toml`'s. So it is
deliberately absent from `operations/` and from `SERVED`, and a row added
for either would claim Bridge can call it. It also means the rule below does
not cover it: the address is written into a Drone's `mcp.json` from `api`'s own
constant, and that shared value is what stands between a typo and a Drone that
can never report.

**The rule that reads `operations/` runs both ways now.** One direction
fails on a route serving a name the inventory does not have; the other fails on
a name the inventory has and nothing serves. The second carries an allowance,
and every entry in it states a reason the gate prints — a list of exemptions
with no sentence each is the silent default the column was given reasons to
remove.

**A second route on the listener is not on this seam either, and it is this
seam spoken differently.** `/agent/mcp` is the agent's door: an MCP client
reaches it, and every tool on it is one row of `operations/` served at the
route `SERVED` already names, so there is no second implementation to drift.
What it adds over the HTTP surface is a cap — a tool answer over 64 KiB is cut,
says so, and names the route that serves it whole and the parameters that ask for less — and a scope: every answer
is inside one Manifest, and the handshake says which. It answers 405 to `GET`
and `DELETE` for `/mcp`'s reason, so it adds nothing to the risk above.

**Who opens it is the repository somebody is standing in.** `armada mcp` is the
relay a repository's own `.mcp.json` names: it reads `fleet.json` for the port,
refuses on `Stale { PidHeldByAnother }` rather than connecting to a port another
process now holds, and resolves its own working directory to a Manifest — never
a request field, for the reason a Job id is not one on a Drone's tools. A
job started below a repository root walks up to it, and says at its
handshake which root it settled on. The walk ends at a repository that has no
Manifest of its own, and at the home directory, so it can reach neither a parent
repository nor an `armada.yml` sitting above every project on the machine.
A job that resolves to no Manifest, or to one this Fleet is not serving, is
answered rather than dropped: the handshake succeeds and carries the reason, and
`tools/list` is empty. **None of it is authentication** — the bind is loopback
with nothing in front of it, so this selects a Manifest and grants nothing.

**The route table is hand-written, and that's an accepted cost, not an
oversight.** A typo in a route path is a runtime 404/500, not a compile error,
on both the main protocol and the lifeboat. That trade was made deliberately
in exchange for not carrying codegen where it isn't earning its keep — see
gRPC's rejection above. It means route changes need a `curl` or integration
check in the same change, because the type system will not catch this class
of mistake for you.
