# Spike 40 — Can a running Job load an MCP server it added?

**Answer: yes, with the `mcp_set_servers` control request.**
A headless CLI in stream-json mode took it on a live process, connected the server within two seconds, and listed it with its tools in the next `mcp_status`.
Nothing restarted.

**The set is a set, and it owns every name it is given.**
A server the process loaded at start, from `--mcp-config` or a config file, is left alone until a set names it.
Once named, it moves to scope `dynamic`, and the next set that leaves it out disconnects it.
So Fleet sends only the servers it added itself, all of them every time, and never a name the process held before.
Sending Armada's own door by name and then leaving it out would cut the Job off from Fleet.

**The CLI does not notice a server added to its config while it runs.**
After `claude mcp add` wrote `.mcp.json`, `mcp_status` still listed the old servers.
Something outside the process has to read the config again and push the difference.

Measured 18 Nov 2026 against Claude Code 2.1.319, macOS, no user message sent, so no turn ran and no model was called.

## The runs

[`040-mcp-set-servers-driver.py`](040-mcp-set-servers-driver.py) starts the CLI as `hosted_session.rs` does for the parts that bear on MCP: `-p --input-format stream-json --output-format stream-json --verbose --mcp-config <door>`.
It adds `--setting-sources` so the person's own plugins stay out of the run.
Every server is [`040-mcp-set-servers-server.py`](040-mcp-set-servers-server.py), a stdio server with one tool.
`spike-door` stands for Armada's door, given by `--mcp-config`.

The request, one line on stdin:

```json
{"type":"control_request","request_id":"r2","request":{"subtype":"mcp_set_servers","servers":{"spike-added":{"type":"stdio","command":"python3","args":["server.py","spike-added"]}}}}
```

The answer names what changed: `{"added":["spike-added"],"removed":[],"errors":{}}`.

### Run 1, `live`: adding and removing ([transcript](040-mcp-set-servers-transcript.ndjson))

| Step | Sent | Answer | `mcp_status` after |
|---|---|---|---|
| 1 | status | | `spike-door` connected, `dynamic`, 1 tool |
| 2 | set `{spike-added}` | added `spike-added` | both connected, 1 tool each |
| 4 | the same set again | nothing changed | unchanged |
| 5 | set `{spike-added, spike-door}` | added `spike-door` | both connected |
| 7 | `claude mcp add -s project spike-file`, nothing sent | | `spike-file` absent |
| 8 | set `{spike-added, spike-file}` | added `spike-file`, **removed `spike-door`** | `spike-added`, `spike-file` |
| 10 | set `{}` | removed both | no servers |

### Run 2, `operator`: a server from a config file ([transcript](040-mcp-set-servers-transcript-operator.ndjson))

`.mcp.json` held `spike-file` from the start, with `--setting-sources project,local`.

| Step | Sent | Answer | `mcp_status` after |
|---|---|---|---|
| 1 | status | | `spike-file` scope `project`, `spike-door` scope `dynamic` |
| 2 | set `{spike-added}` | added `spike-added` | all three connected |
| 4 | set `{}` | removed `spike-added` | `spike-file` and `spike-door` kept |
| 6 | set `{spike-file}` | added `spike-file` | `spike-file` now scope `dynamic` |
| 8 | set `{spike-door}` | added `spike-door`, removed `spike-file` | |
| 10 | set `{}` | removed `spike-door` | neither left |

### Run 3, `unapproved` ([transcript](040-mcp-set-servers-transcript-unapproved.ndjson))

The same `.mcp.json`, with no `enableAllProjectMcpServers` and no approval on record.
A headless start loaded `spike-file` and connected it.
So a server a Job adds to `.mcp.json` would have loaded at its next start, and Fleet loading it now asks no more trust than a restart would.

## Where `claude mcp add` writes

| Scope | File | Key |
|---|---|---|
| `local`, the default | `~/.claude.json` | `projects["<main checkout>"].mcpServers` |
| `user` | `~/.claude.json` | `mcpServers` |
| `project` | `.mcp.json` in the folder it ran in | `mcpServers` |

**The local scope is keyed by the main checkout, not the slot.**
`claude mcp add -s local` run inside a slot's folder wrote under the repository's own root.
The CLI also read the root's `.mcp.json` when it started in a folder below it.

## How Fleet should notice

The CLI pushes nothing when its config changes, and a set needs the whole config of each server, so Fleet reads the three places above itself.
Looking only when Bridge polls the Loadout would leave a Job without its server whenever nobody has the Loadout open.
A file watcher per Job would work, and costs a watcher on `~/.claude.json`, which every CLI on the machine rewrites as it runs.
Fleet already hears each command a Job runs end, and a stat of three files after each would tell it whether any changed.
The owner chose the Loadout poll alone (18 Nov): a stat of the three files on each `GET /sessions/mcp`, and a read only when one changed.
The cost is that a server loads only while someone has the Loadout open.

## What was not measured

- A server the CLI refuses. The binary holds refusals for a reserved name and for a Remote Control session; neither applies to a hosted Job, and none was seen.
- Whether the model's next request in the same turn sees the new tools. The push races the agent's next call.
- A plugin's servers, which arrive with `reload_plugins`, not a set.
