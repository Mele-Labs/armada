---
name: gitnexus-debugging
description: "Use when the user is debugging a bug, tracing an error, or asking why something fails. Examples: \"Why is X failing?\", \"Where does this error come from?\", \"Trace this bug\""
---

# Debugging with GitNexus

## When to Use

- "Why is this function failing?"
- "Trace where this error comes from"
- "Who calls this method?"
- "This endpoint returns 500"
- Investigating bugs, errors, or unexpected behavior

## Bind the repository first

A root cause traced in the wrong repository is a wrong root cause: bind it as
[gitnexus-guide](../gitnexus-guide/SKILL.md#bind-the-repository-first) says. A stale index describes the code from before your bug, so refresh
before trusting a trace.

## Workflow

```
0. list_repos {}                                          → Bind repo
1. query({search_query: "<error or symptom>"})            → Find related execution flows
2. context({name: "<suspect>"})                    → See callers/callees/processes
3. READ gitnexus://repo/{name}/process/{name}                → Trace execution flow
4. cypher({statement: "MATCH path..."})                 → Custom traces if needed
```

> If "Index is stale" → run `pnpm gitnexus:index` in terminal. Never a bare `analyze`, which rewrites `AGENTS.md`.

## Checklist

```
- [ ] list_repos {} — bind repo; explicit repo when >1 indexed, ask if ambiguous
- [ ] Understand the symptom (error message, unexpected behavior)
- [ ] query for error text or related code
- [ ] Identify the suspect function from returned processes
- [ ] context to see callers and callees
- [ ] Trace execution flow via process resource if applicable
- [ ] cypher for custom call chain traces if needed
- [ ] Read source files to confirm root cause
- [ ] State the repository and index freshness with the diagnosis
```

## Debugging Patterns

| Symptom              | GitNexus Approach                                          |
| -------------------- | ---------------------------------------------------------- |
| Error message        | `query` for error text → `context` on throw sites |
| Wrong return value   | `context` on the function → trace callees for data flow    |
| Intermittent failure | `context` → look for external calls, async deps            |
| Performance issue    | `context` → find symbols with many callers (hot paths)     |
| Recent regression    | `detect_changes` to see what your changes affect — pass `worktree` for a linked worktree |
| "How does A reach B?" | `trace` between the two symbols — shortest call chain in one call |

## Tools

This skill uses `query`, `context`, `cypher`, `trace` and `detect_changes`; the
guide's [Tools Reference](../gitnexus-guide/SKILL.md#tools-reference) says what
each returns.

When no path exists, `trace` reports the furthest reachable node — exactly where the chain breaks (dynamic dispatch, reflection, or an external boundary).

## Example: "Payment endpoint returns 500 intermittently"

```
0. list_repos {}
   → total: 2 (my-app, billing-api) — bind my-app explicitly on every call

1. query({search_query: "payment error handling", repo: "my-app"})
   → Processes: CheckoutFlow, ErrorHandling
   → Symbols: validatePayment, handlePaymentError

2. context({name: "validatePayment", repo: "my-app"})
   → Outgoing calls: verifyCard, fetchRates (external API!)

3. READ gitnexus://repo/my-app/process/CheckoutFlow
   → Step 3: validatePayment → calls fetchRates (external)

4. Root cause: fetchRates calls external API without proper timeout
   Repository: my-app  Index: current
```

With a single indexed repository, step 0 returns `total: 1` and the `repo`
argument drops out of every call above.
