# 6. Exemplars over rules

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

Established on the [Agent Copy Contract](../agent-copy.md) and restated here
because it is a prompt-assembly rule: **seeding a prompt with real samples
beats adding another rule.** Measured in the field — AI-generated
templates used words the real author had never once used, and the fix was
samples rather than a rule.

This is the answer to a failure a blocklist structurally cannot catch. A
clause welded into a noun generates a new string every time, so ban the
grammar, not the phrase — and show the grammar you want rather than
describing it.

## Resolved at layer 3, injected at layer 6

**Corrected Aug 2026.** The first cut sited the corpus at layer 3 and left
it there, which meant paying for it on every step of every Job forever.

The corpus is repo-specific in **origin** and step-specific in **use**.
Roughly 50 curated commit messages and 10 PR bodies, drawn from pre-AI
history — but the surfaces it exists to seed are written at `merge` and at
whichever step opens the PR. On Refactor's six steps it is live context
for one. Ten PR bodies is not a small constant, and layer 3 is frozen at
spawn and spans the whole Job.

So: resolve it at layer 3, where it comes from. Inject it at layer 6, on
steps whose evidence type is a commit message or a PR body.

**Eight is a legal size.** The Copy Contract's fallback: if the pre-cutoff
corpus is thin or poor, hand-write 8 to 10 exemplars. About an hour, and
arguably better, since the awkward cases — a revert, a partial fix, a
change that turned out wrong — can be covered deliberately. Fifty is a
target, not a floor.

**Resolution across several Manifests is unstated.** The corpus resolves
at layer 3, and layer 3 resolves across every Manifest gating the Job.
Under *knowledge unions* a Job gated by three carries 150 commit messages
and 30 PR bodies. Nobody has said whether that is intended.

Cutoff date decided — found by inspection when the corpus is harvested,
with the method recorded alongside it.
