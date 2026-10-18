# Changes that read like they break nothing

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

A refusal on any ID mismatch means the question "is this change safe for an
older peer" no longer decides anything: every wire change is refused by a peer
built before it. The mistakes below still matter, because they are the ones
that break a peer *after* the IDs agree, in a Bridge whose mirror was edited
by hand.

**An optional field is left out when it is empty, never sent as `null`.**
Bridge types it `field?: T` and compares against `undefined`, which `null`
passes. Every `Option` on a DTO Fleet writes carries `skip_serializing_if`, or
`deserialize_with = "stated"` where `null` is a value a person chose.
`xtask/src/rules_protocol/nulls.rs` is the rule.

The three people get wrong most often: widening an enum "because it's just
adding cases," making a field `Option<T>` "to be safe," and renaming a variant
"for clarity." All three feel non-breaking from inside the change and are not.
If you catch yourself writing "this shouldn't break anything, it's just
adding/loosening X" — that sentence is the tell. Check whether the mirror in
`packages/protocol` has an exhaustive match, a presence assumption, or a name
lookup anywhere near the thing you are touching.

**A closed set Bridge matches on is a `wire_enum!`, and one it only renders is
a string.** `StudioNodeState` — where a node on a Studio stands — is a strict
`wire_enum!`, because Bridge draws a different thing for each of its values. A
new one would mean the Studio grew a state, which is a change every reader has
to be told about. A `Requester.kind`'s set grows every time Fleet
learns of another asker, which is a change no reader needs to be told
about at all. Ask which of those two a new set is before making it open.
