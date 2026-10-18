# Protocol 23.3: the files a fix holds off the Jobs that hit its test

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

#1673, the Fleet half.

**One optional field, additive.** `ClaimedBreakage` gains `held_off`, the files no Job on the claim
but the fix may change while it stands: those the reporting Drone named in `draft_fix`, then those
the fix has declared it will change. Absent is none, which is every claim from before 23.3 whose
fix has declared nothing yet. It reads the same from either side of the claim, like the rest of
the entry.

**Bridge reads `whole.breakages` for everything else.** Which Job is the fix, its title, and which
Check and test it is fixing were already there: `fix`, `fix_title`, `check`, `test`. A failed Check
row matches a breakage by `check`.

**A landed fix leaves `whole.breakages` at the merge, as before**, though its files stay held off
each Job until that Job's next catch-up brings the fix in. That hold is not on the wire; the Drone
is told, and `docs/concepts/fleet.md`, *A test another Job is fixing*, has why it outlives the merge.

**Not on the wire: `draft_fix` gains a required `files`.** It is an MCP tool, not this protocol, and
its own schema says so. **Store V94** keeps a claim's files and what a landed fix still holds.
