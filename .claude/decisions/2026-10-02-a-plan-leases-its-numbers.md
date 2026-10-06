# How does a branch get a migration or protocol number without colliding?

**Decided 2026-10-02.** Replaces what #1059 recorded on 14 Sep: *"a branch carries no migration or protocol number."*

Two agent branches each add a store migration, both pick V93, and the one that lands second merges main, renumbers to V94, rewrites the comment and protocol section that named it, and runs every Check again. On 1–2 Oct that was a large share of a 3-hour run. He said: *"Not every repo that uses armada can change migrations like we can. Leasing feels like the most flexible for multi-repo support,"* and *"Not all migration systems use numbers. For example rails uses dates."*

Offered the 14 Sep union-merge plan, a number filled in at landing, and a lease taken when a branch starts, he took none of them: *"We have a new plan system. Why can't part of the plan be that it needs to lease a new migration (also what else besides migrations might need to be leased like this to avoid conflicts)."*

**First answer, revised the same day: Fleet issues the number when the plan is recorded.** He then changed who decides: *"instead of fleet issuing the number I really think that we should let whoever proposed the 'needs' defines what they need. Fleet keeps it and when another tasks says that it needs something in the same directory/file, fleet lets that task know that something else is already ahead of it. Heck even potentially letting the two agents communicate with each other and then fleet keeps track of the lease resolution once they decide."*

**Chosen: a task declares what it needs on a file, and Fleet records it and the order.** A need is a path and what is needed there, in the declarer's words (`crates/store/src/migrations.rs`, *a new migration*). A second need on the same file hears which Job is ahead and what it took. No repository declares kinds up front; the file is the resource, so a Rails repository's need sits on `db/schema.rb`. A Drone can add a need mid-work, as it corrects its scope.

- **First to declare goes first, he chose.** Fleet tells the second Job who is ahead and what that Job took, and records the order. The two agents talk only when one asks to go first, and Fleet records what they agree; that part is a later build. **Cost he took:** until it exists, a Job that should jump the queue needs him to reorder it.
- **The later Job waits at the merge line, he chose.** A Job lands only once every need ahead of it on the same file has landed or been given back. **Cost he took:** a slow or stuck Job holds the Jobs behind it.
- **Sessions outside Fleet declare from the `armada` CLI, he chose**, the way `armada worktree lease` works. **Cost he took:** a step in `work-issue` an agent can forget, and a forgotten one collides as today.
- **Append-only lists (component exports, CSS imports, `operations.toml`, `icons.toml`, mod lists) keep both lines, he chose.** No need is declared for them; a conflict confined to a declared list file is resolved by taking both sides. **Cost he took:** a list file must hold only entries, or two edits interleave unseen.

**This changes a rule in `docs/concepts/fleet.md`**, *Write-scope overlap*: "It is deliberately not a lease." Overlap stays a warning; a declared need is the part that is ordered.

- **Asked on 6 Oct whether the checks should refuse a branch that adds a migration or bumps the protocol minor with no need declared, he chose "Yes, refuse it".** A branch had bumped 23.33 to 23.35 undeclared and taken the number another branch held. **Cost he took:** a branch that forgets is stopped at the merge line until it declares.

**Built without asking, for him to overrule:** Fleet holds its own merge act under both `merge_by: forge` and `push`, since under `forge` Fleet is still the one asking the forge to merge; a person pressing GitHub's own button bypasses the order. A need has no expiry by time; a person gives one back with `armada need --release`.

**Where it landed:** https://github.com/NickMele/armada/issues/1059, which lists what is built and what is open. The list files, `armada need`, Fleet's half (protocol 23.36) are in main.
