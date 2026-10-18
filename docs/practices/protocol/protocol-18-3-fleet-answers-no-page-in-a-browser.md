# Protocol 18.3: Fleet answers no page in a browser

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1460`. One error code added — `api.from_a_page`, a 403 on any request or WebSocket upgrade
carrying an `Origin` header. Adding a code is minor by `docs/contracts/error-contract.md`: Bridge
looks one up or falls back, so a code it has never heard of renders either way.

**No DTO, route or event kind moved**, and no caller's behaviour changes. Bridge's main process
sets a content type and nothing else, the `armada` CLI writes its own request head, the `ws` client
sends no `Origin` unless told to, and the agent's door builds its inner request rather than
forwarding one — so nothing that reaches Fleet on purpose sends the header this refuses.

**The bump is here because the code is a new answer an older Bridge can meet**, not because the
shape changed. A Bridge built before this renders it through the fallback, which is exactly what
the error contract promises.
