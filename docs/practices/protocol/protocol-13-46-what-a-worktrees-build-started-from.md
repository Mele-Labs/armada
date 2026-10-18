# Protocol 13.46: what a worktree's build started from

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`RunSheet.seeding` and `CheckoutRunSheet.seed`, additive (#1064). `seeding` is absent where the Job's Manifest declares no `setup.seed`; otherwise it is `seeded`, with the base commit and the directories cloned, `cold`, with Fleet's sentence for why, or `unrecorded` for a worktree cut before seeding existed. `seed` is absent where the Manifest declares none; otherwise it names the directories and the Commands that warm them, and says whether the seed at the current base commit is `warm`, `warming` or `cold`.
