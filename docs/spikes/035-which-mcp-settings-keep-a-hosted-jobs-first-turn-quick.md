# Spike 35 — Which MCP settings keep a hosted Job's first turn quick while a slow server still connects?

**Answer: `CLAUDE_CODE_MCP_STARTUP_WAIT_MS=0` with `MCP_TIMEOUT=120000`.** The first turn
starts in about a second, the door is connected before it, and a server that takes 45 seconds
still connects behind it. Neither setting does it alone.

**The door is connected before the first turn because it is the permission prompt tool's server,
and not because of either setting.** The CLI waits for that one server whatever the startup wait
says. Dropping `--permission-prompt-tool` left the door `pending` at the first turn.

Measured 21 Oct 2026 against Claude Code 2.1.301 with `--model haiku`, macOS, load average 5 to
13, one run at a time under `taskpolicy -c utility`.

## The runs

Each run started the CLI as `hosted_session.rs` does for the parts that bear on MCP: `-p
--input-format stream-json --output-format stream-json --verbose --permission-prompt-tool
mcp__armada-fleet__ask_the_person --mcp-config <file>`, plus `--strict-mcp-config` so only the two
stub servers loaded. It sent one `say ok` turn on stdin at once, and an `mcp_status` control
request 60 seconds in.

- The door is `armada-fleet`, answering `initialize` at once unless the row says otherwise.
- The slow server sleeps 45 seconds before answering `initialize`.
- Times are seconds from the process starting, which is also when the turn was sent.

| Set | Door delay | First turn's init line | First assistant line | Door at the first turn | Slow server at the first turn | Slow server at 60s |
|---|---|---|---|---|---|---|
| nothing | 0 | 28.1 | 29.2 | connected | failed | failed |
| `MCP_TIMEOUT=120000` | 0 | 45.8 | 47.2 | connected | connected | connected |
| `…STARTUP_WAIT_MS=0` | 0 | 1.5 | 2.7 | connected | pending | **failed** |
| `…STARTUP_WAIT_MS=3000` | 0 | 4.9 | 6.1 | connected | pending | **failed** |
| `…STARTUP_WAIT_MS=0`, `MCP_TIMEOUT=120000` | 0 | 1.1 | 2.2 | connected | pending | connected |
| `…STARTUP_WAIT_MS=3000`, `MCP_TIMEOUT=120000` | 0 | 4.2 | 7.2 | connected | pending | connected |
| `…STARTUP_WAIT_MS=0`, `MCP_TIMEOUT=120000` | 2 | 2.9 | 4.3 | connected | pending | connected |
| `…STARTUP_WAIT_MS=3000`, `MCP_TIMEOUT=120000` | 2 | 4.0 | 5.3 | connected | pending | connected |
| `…STARTUP_WAIT_MS=3000`, `MCP_TIMEOUT=120000` | 5 | 6.1 | 7.3 | connected | pending | connected |
| `…STARTUP_WAIT_MS=0`, `MCP_TIMEOUT=120000`, no `--permission-prompt-tool` | 2 | 1.3 | 3.0 | **pending** | pending | connected |
| `…STARTUP_WAIT_MS=0`, `MCP_TIMEOUT=120000`, the person's own servers too | 0 | 1.2 | 4.2 | connected | pending | connected |

## What the table says

- **Nothing set holds the first turn up to `MCP_TIMEOUT`**, 30 seconds by default, and then gives
  up on the slow server for good. This is the `CONNECT_TIMEOUT … timed out after 30000ms` a hosted
  Job showed on every process start.
- **`MCP_TIMEOUT` alone makes it worse**: the first turn waits for the slow server, 46 seconds here.
- **The startup wait alone makes the first turn quick but still kills the slow server at 30
  seconds**, because `MCP_TIMEOUT` is also each server's connect timeout.
- **A wait of 3000 buys nothing over 0** for the servers measured. Gitnexus takes 7 seconds and an
  `npm exec …@latest` server 14 seconds, so neither makes a 3-second window, and every process
  start would pay the 3 seconds.
- **The door is waited for whatever the setting.** A door 2 seconds late under a wait of 0, or 5
  seconds late under a wait of 3000, was still `connected` in the init line. Without
  `--permission-prompt-tool` the same door was `pending`.
- **The last row is the hosted Job as it runs**, without `--strict-mcp-config`: 20 servers, with
  the door connected and the person's own servers `pending` at the first turn. By 60 seconds every
  server that was not `needs-auth` was connected.

## What this does not show

- Whether a server that connects mid-turn is offered to the model in that turn or only in the next.
  The probe turn called no tools.
- Behaviour at load 60. The worst load during these runs was 13. Nothing measured here depends on
  load except how long a server takes, and the door was waited for at every delay tried. A door
  slower than `MCP_TIMEOUT` was not tried.

## Reproducing it

`035-run.py` is the driver and `035-slow-server.py` the stub server. From a scratch directory:

```
python3 <repo>/docs/spikes/035-run.py w0t120 CLAUDE_CODE_MCP_STARTUP_WAIT_MS=0 MCP_TIMEOUT=120000
```

`DOOR=<seconds>` delays the door, `--no-ppt` drops `--permission-prompt-tool`, and `--loose` drops
`--strict-mcp-config`. Each run takes about 75 seconds and prints one JSON line.
