# A bay's history: the pool's own record, and the slot's reflog

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`get_slot_history` (`GET /slot_history?manifest_id=&slot=`) answers the Cockpit's Bay pane: who held one slot, and what was checked out in it, newest first, for thirty days, at most 300 (`MOST_BAY_HISTORY`) with `cut` when older ones were left out. Since 15 Oct 2026. `agent_access` is `No`.

| Entry | Source | Why not the timeline |
|---|---|---|
| `leased`, `released` | `slot-<n>.history`, which the pool writes beside the lease record | The timeline's `slot.leased` is written by Fleet's own calls, so `armada worktree lease` never reaches it. A slot held 65 hours by an agent's lease had no history at all |
| `checkout` | The slot's own `HEAD` reflog, `<git dir>/worktrees/<slot>/logs/HEAD`, read as a file | Checkouts inside a bay are recorded nowhere else |

**The pool writes the lease line, in whichever process takes the lease.** Fleet noticing a record change was rejected: a read of the record misses a lease taken and given back between two reads and every one taken while Fleet is down, and the lease command is a process of its own. The pool already holds the slot's lock when it writes or clears the record, so the line goes down there (`crates/adapters/src/leasing/history.rs`) and no caller has to remember. A failure to write it never fails the lease. The file is cut to thirty days on its next write.

**A lease taken from a holder that had gone, without giving the slot back, is entered as that holder's release** first, dated when the lease took the slot: nothing dates the holder's going, and two leases in a row would leave a reader to guess.

**Fleet names the holders when asked.** The pool writes a process (pid and start time, with the agent session where its record has one) or an id. A Job's lease is its id, a hosted Session's is a Job of its own id, and a repair's is `<job>-repair-…`, so an id Fleet has no Job for is a `session`. A process is an `agent`, with the Session as `id` where the record has one and the pid where not. Titles are read when asked and not kept with the entry, so a Job forgotten since has none. This is the same resolution `list_worktrees` makes for a slot's holder.

**Additive.** One new operation, `BayHistory`, `BayHistoryEntry` (tagged by `kind`, strict), `BayHolder` and `BayHolderKind` (strict). Bridge's half is `packages/protocol/src/bay-history.ts`, written by hand, and `getSlotHistory(manifestId, slot)` on the preload. Every optional field is left out when empty. There is no number to take and `armada need` has nothing to order: the protocol ID moves with the wire files.

**The event stream is untouched**, so the back-pressure risk above is neither better nor worse. The lifeboat is untouched. The read is two small files and the reflog on each ask, in a blocking task, with nothing kept; a reflog of thirty days of an agent's `git switch` is the largest of them.
