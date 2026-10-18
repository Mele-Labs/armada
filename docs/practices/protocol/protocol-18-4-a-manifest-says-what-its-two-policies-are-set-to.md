# Protocol 18.4: a Manifest says what its two policies are set to

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Two optional fields added to `ManifestSummary` — `auto_merge` and `review_gate`, the word each key
holds as `armada.yml` writes it. Additive: a Bridge built before this ignores both, and a Fleet
built before this sends neither.

**A gate that defers had no way to say what it deferred to.** A step declaring
`manifest_rule:review_gate` draws as *the repository decides*, and a reader could not tell whether
a person would be asked or nobody would — the owner, 29 September 2026, reading that row. Fleet
already resolves both policies at every gate (the `policy` module (deleted since)), so this reports a
value it computes rather than computing a new one.

**Reported, not resolved, and that is the whole of what crosses.** One Manifest's word, not the
fold across a Job's several gating Manifests, and not the answer at any particular gate: both
settings are live, so the resolution has a lifetime shorter than a read. A Bridge that rendered
this as *what will happen* rather than *what the repository says today* would be claiming the
policy cannot move, which is the one thing it can do.

**Absence is the older Fleet and nothing else.** A file that declares neither key means each
policy's own default, and Fleet sends that word rather than leaving the field out — so a missing
field says the peer is older than 18.4, and Bridge draws the deference unresolved rather than
naming a default the repository never wrote. They are `#[serde(default)] String` on Fleet's side
and `?: string` on Bridge's, which is `WorkflowSummary.source`'s spelling for the same situation.
