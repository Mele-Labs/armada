# How does a branch get a migration or protocol number without colliding?

**Decided 2026-10-02.** Replaces what #1059 recorded on 14 Sep: *"a branch carries no migration or protocol number."*

Two agent branches each add a store migration, both pick V93, and the one that lands second merges main, renumbers to V94, rewrites the comment and protocol section that named it, and runs every Check again. On 1–2 Oct that was a large share of a 3-hour run. He said: *"Not every repo that uses armada can change migrations like we can. Leasing feels like the most flexible for multi-repo support,"* and *"Not all migration systems use numbers. For example rails uses dates."*

Offered the 14 Sep union-merge plan, a number filled in at landing, and a lease taken when a branch starts, he took none of them: *"We have a new plan system. Why can't part of the plan be that it needs to lease a new migration (also what else besides migrations might need to be leased like this to avoid conflicts)."*

**Chosen: a plan's task declares the leases it needs, and Fleet issues them when the plan is recorded.** Each repository declares its leasable kinds; work outside a plan leases from the CLI.

- **Asked what happens when Job B holds 96 and is ready before Job A, holding 95, he chose "B waits for A".** A Job lands only once every lower number of its kind has landed or been handed back. **Cost he took:** a slow or stuck Job holds the Jobs behind it in the merge line.
- **Asked whether append-only lists (component exports, CSS imports, `operations.toml`, `icons.toml`, mod lists) belong in the same work, he chose "keep both lines".** No lease; a conflict confined to a declared list file is resolved by taking both sides. **Cost he took:** a list file must hold only entries, or two edits interleave unseen.

**Not decided:** whether a stalled Job's lease expires or is taken back by a person.

**Where it landed:** https://github.com/NickMele/armada/issues/1059, rewritten. Nothing built.
