# What fills the context window

**Part of:** [Job](../session.md).

**A Job's context tank opens onto what fills it, the way `/context` shows it.** The mod asks the engine for a `summary` breakdown, estimated locally and never `full`, which would call the token-count API for every tool and file.
It asks when the throttle lets a measurement through and the context moved, and sends `SessionRecord.context_breakdown`: `categories` (`name`, `tokens`, and a `kind` of `used`, `free`, `buffer` or `deferred` that Bridge branches on, never the name), `memory_files` (`path`, `tokens`), the ten `mcp_tools` that weigh most (`name`, `server`, `tokens`), `total_tokens`, `max_tokens` and `autocompact_threshold?`.
**Names, paths and counts only**: no message text, prompt or file contents leave the mod.
The live token count is the `context_tokens` already on `usage`.
