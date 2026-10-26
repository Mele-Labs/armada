# 3. Layers

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

**Revised Aug 2026** after adversarial review. A Drone's prompt is
assembled from six sources, and **freeze time orders them.**

That ordering was derived from a mechanical constraint: injected MCP
servers cannot be swapped and the system prompt cannot be rewritten
mid-session without a kill and respawn, so a block that could not be
rewritten mid-session had to precede the block that was rewritten every
step. **The derivation is withdrawn and the ordering is not.** A Drone
belongs to a workflow step and every step is a kill and respawn, so all
six layers are assembled fresh at every boundary and none of them is
constrained by another's freeze time.

What holds the order up instead is that it is the one a reader can
predict: earliest-frozen first, so reading down a prompt reads outward
from what nothing can change to what changes every step. That is a
convention, said plainly as one. **Do not reorder the layers** — the
ordering was right for a reason that stopped applying, which is not the
same as being wrong.

| # | Layer | Source | Freezes at | Contains |
| --- | --- | --- | --- | --- |
| 1 | **Baseline** | Armada, compiled in | Build | Section 5. Identical for every Drone, every Job, every Manifest |
| 2 | **Kit** | The Kit file | Spawn | Voice and tone, **Way I work**, **Expectations**, machine-wide Skills, the Agent file, Sub agent definitions. **Voice classifies as Machine rather than Kit** — see the recorded disagreement above |
| 3 | **Manifest** | `armada.yml`, the repository's or Kit's copy | Spawn | Project conventions, repo-specific Skills, project Sub agents, the exemplar corpus (resolved here, injected at layer 6 — see section 6) |
| 4 | **WorkflowDef** | The Job's type | Spawn | What this kind of work means and what finishing it looks like |
| 5 | **Task** | The Job | **Job creation** — earlier than spawn | The request, plus `acceptance_criteria[]` with each entry's source (`ticket` / `helm_drafted` / `human`) |
| 6 | **Step** | Current workflow step | **Never** — re-assembled at each step boundary, like every layer above it | The step's criteria, the Evidence Scope **policy**, the exemplar corpus on steps that write prose |

**Task moved from last to fifth, Aug 2026.** It freezes at Job creation,
earlier than everything above it, and sat last. Under one ordering
principle it belongs ahead of the layer that changes most often. One
consequence survives: the block a reader can rely on is contiguous and
comes first.

The second consequence stated then does not survive. Layer 6 was "the only
thing re-emitted at a step boundary — which is the cheapest place for
re-assembly to sit", and **all six re-emit now**, so re-assembly is not
sitting anywhere cheap. What that costs is a prompt assembled per step
rather than per Job, which is arithmetic on a string and is not the
expensive part of a spawn.

The alternative was to declare prompt order to be recency rather than
freeze time. Rejected: a second organising principle means nothing
decides conflicts between them, and any order becomes justifiable after
the fact.

**The Agent file is Kit-only** — there is no Manifest-level counterpart.
Skills, MCP and Plugins are genuinely two-tier and inherit Kit → Manifest.

## Two rules, not one

**Corrected Aug 2026.** An earlier draft said "later layers narrow earlier
ones and never widen them," borrowing the config direction rule. That rule
was **withdrawn Aug 22 2026** — but this correction was right before the
withdrawal and is right after it, for its own reason: almost nothing in
this stack is a permission, and Drone states the scope in one line,
*instructions grant none.* The corpus at layer 3 narrows nothing — it
adds. Voice at layer 2 tunes length and formality. `acceptance_criteria[]`
supplies content that layers 4 and 6 point at.

| Kind | Rule |
| --- | --- |
| **Instructions** | A later layer may **add** and may **specialise**. It may not contradict an earlier layer. **Layer 1 is unreachable from config** |
| **Permissions** | Not governed here. Allowlist, MCP, Commands, secrets scope and Sub agents resolve per Drone's resolution table — a permission intersects, knowledge unions, with Commands the amended exception since namespacing removed the conflict intersection existed to resolve |

**Layer 1 is unreachable.** A Kit or Manifest able to edit the baseline
could delete the sentence making Evidence the only completion path, at
which point the completion path is user-configurable and this contract
means nothing. **This one stands on its own footing**, not on the
withdrawn config direction rule the first cut cited: it is a claim about
what config may reach, and layer 1 is reachable from nothing.

**No layer budget exists.** Six layers plus a 60-sample corpus, and
nothing bounds the total. `max_context_size` bounds the Judge's context;
nothing bounds the Drone's. Unresolved, and it is the constraint section 5
is measured against.
