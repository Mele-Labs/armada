# Protocol 18.0: the answer that could no longer appear is gone

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1518` shipped `allow_every_read` in 17.3, on a card over a read of Armada's own door. `#1525`
made a door read run without anybody being asked an hour later, so no card is drawn over one and
the answer had nothing to appear on. **A variant Bridge matches on, removed, is a major bump**, and
this is that bump: `HelmCallAnswer` loses `allow_every_read` and `HelmCallSettled` loses
`every_read_allowed`.

**A major is refused in either direction**, so Fleet and Bridge come up together or not at all.
That is what `scripts/restart` does in one move, and it is why a dead variant was worth removing
rather than leaving: a union member nothing can produce is a promise the system does not keep.

**The owner decided this and it was not this branch's to decide.** Removing it reverses his own
approval of an hour earlier, which `CLAUDE.md` says is his call; he was asked and said remove it.

**Nothing else moved.** `ran_unasked`, added in 17.4, stays. Minor resets to 0.
