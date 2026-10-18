---
name: gitnexus-exploring
description: "Use when the user asks how code works, wants to understand architecture, trace execution flows, or explore unfamiliar parts of the codebase. Examples: \"How does X work?\", \"What calls this function?\", \"Show me the auth flow\""
---

# Exploring Codebases with GitNexus

## When to Use

- "How does authentication work?"
- "What's the project structure?"
- "Show me the main components"
- "Where is the database logic?"
- Understanding code you haven't seen before

## Bind the repository first

Step 1 discovers what is indexed; bind every call after it as [gitnexus-guide](../gitnexus-guide/SKILL.md#bind-the-repository-first) says, and
report the repository and index freshness alongside your explanation.

## Workflow

```
1. list_repos {} or READ gitnexus://repos                          → Discover indexed repos
2. READ gitnexus://repo/{name}/context             → Codebase overview, check staleness
3. query({search_query: "<what you want to understand>"})  → Find related execution flows
4. context({name: "<symbol>"})            → Deep dive on specific symbol
5. READ gitnexus://repo/{name}/process/{name}      → Trace full execution flow
```

> If step 2 says "Index is stale" → run `pnpm gitnexus:index` in terminal. Never a bare `analyze`, which rewrites `AGENTS.md`.

## Checklist

```
- [ ] list_repos {} — bind repo; explicit repo when >1 indexed, ask if ambiguous
- [ ] READ gitnexus://repo/{name}/context
- [ ] query for the concept you want to understand
- [ ] Review returned processes (execution flows)
- [ ] context on key symbols for callers/callees
- [ ] READ process resource for full execution traces
- [ ] Read source files for implementation details
- [ ] State the repository and index freshness with the explanation
```

## Resources

| Resource                                | What you get                                            |
| --------------------------------------- | ------------------------------------------------------- |
| `gitnexus://repo/{name}/context`        | Stats, staleness warning (~150 tokens)                  |
| `gitnexus://repo/{name}/clusters`       | All functional areas with cohesion scores (~300 tokens) |
| `gitnexus://repo/{name}/cluster/{name}` | Area members with file paths (~500 tokens)              |
| `gitnexus://repo/{name}/process/{name}` | Step-by-step execution trace (~200 tokens)              |

## Tools

This skill uses `query` and `context`; the guide's
[Tools Reference](../gitnexus-guide/SKILL.md#tools-reference) says what each
returns.

## Example: "How does payment processing work?"

```
1. list_repos {}                             → total: 1 (my-app) — bind it
   READ gitnexus://repo/my-app/context       → 918 symbols, 45 processes
2. query({search_query: "payment processing"})
   → CheckoutFlow: processPayment → validateCard → chargeStripe
   → RefundFlow: initiateRefund → calculateRefund → processRefund
3. context({name: "processPayment"})
   → Incoming: checkoutHandler, webhookHandler
   → Outgoing: validateCard, chargeStripe, saveTransaction
4. Read src/payments/processor.ts for implementation details
5. Answer, noting: Repository my-app, index current
```

Had step 1 returned two repositories, every call above would carry
`repo: "my-app"`.
