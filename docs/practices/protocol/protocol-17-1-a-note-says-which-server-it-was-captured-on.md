# Protocol 17.1: a Note says which server it was captured on

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1294`. `StudioCapture.served`, optional — `{ run, name, address }`: the instance Fleet held, what
the Manifest calls it, and the origin the capture window was pinned to. Additive: one new optional
field and one new DTO.

**Absent is a Note captured on Bridge**, which is every Note written before this, so nothing an
older Bridge already reads changes and no stored row is rewritten. A Bridge that does not know the
field draws the Note exactly as it draws one taken on Bridge — which is what that build can
truthfully say about a page it has no record of.

**The origin, and `location` is the path within it.** Neither repeats the other, and the two
together are the page. The address is not an identity and the field does not claim to be one:
whatever bound that loopback port owns the origin, which is why capture ends with the Run —
`docs/practices/capture-window.md`, *What this does not claim*.

**Additive against 17.0, re-read as such rather than carried across the major.** This was written
on 16.x, and 17.0's rename is what makes it worth saying out loud: what it adds is one optional
field on `StudioCapture` and one new DTO beside it, neither named anywhere 17.0 touched.
`ProposedRunner` is the manifest-proposal surface and nothing here composes it. The two new
`Outcome` arms are Bridge's own — produced in main, read in its own renderer, sent in neither
direction — so they are not on this wire at all.
