# Spike 38 — How far behind is another machine on beads?

**Answer: one sync cycle behind, 12 to 33 seconds after something runs `bd sync`, and nothing runs it by itself.**
A write lands in its own machine's copy at once and is not pushed until a sync; the other machine sees it after its own next sync.
**Two machines that claim the same issue are both told they have it.** The sync settles it silently, the later write winning on both, so a claim is only known once it has been synced and read back.

Measured 24 Oct 2026 with beads 1.3.1 and Dolt 2.6.0, on this Mac, against a throwaway private GitHub repository.
The question is the owner's, 23 Oct 2026: whether his two machines and a second engineer would be synced with Armada's work, and whether an agent on one machine sees what an agent on another is doing.
It follows [spike 37](037-does-dolt-fit-the-central-store.md), where the owner chose to keep Armada's backlog public, on Armada's own repository.

## How it was run

`038-run.sh` stands two databases on this Mac for two machines, `a` and `b`, each in its default embedded mode.
`a` is a copy of the database spike 37 imported, 955 of Armada's issues.
Both use the GitHub repository as their Dolt remote, so the data sits on `refs/dolt/data`, the ref a normal clone never fetches.
`038-transcript.txt` is its output.

## What it measured

| Step | Took | What happened |
|---|---|---|
| `a` pushes the 955 issues first | 11.1 s | The remote ref held 204 KB |
| `b` clones with `bd init --remote` | 5.0 s | `b` held no issues. One `bd sync` on each machine later brought `b` all of them; why the first push and clone did not was not found |
| `a` creates five issues | 1.0 to 1.1 s each | **The remote ref did not move.** A write is not pushed by itself |
| `b` reads before syncing | | Does not see them |
| `a` runs `bd sync` | 11.7 s | Pushed |
| `b` runs `bd sync` | 16.0 s | Pulled; `b` sees them |
| `a` and `b` each create one, then sync at the same moment | 32.7 s for both | Both exit 0 and both copies hold both issues: the loser of the push race pulled, merged and pushed again on its own |
| `a` and `b` claim the same issue, then sync at the same moment | | Both claims print `✓ Updated`. Both syncs exit 0 with no conflict reported. Afterwards both copies say `mac-b` holds it: beads' sync resolves a field both sides changed by a three-way merge of its own, and the later change won |
| Size | | 962 issues came to 1.9 MB on the remote. Locally `b` holds 4.4 MB and `a` 67 MB, most of it left over from spike 37's import |

## What it means for Armada

- **Something has to run `bd sync`.** An agent cannot be trusted to remember it, so Fleet would run it: after a Job writes to beads, and on a timer so a machine that writes nothing still hears the others.
  With a sync every 30 seconds, an agent on the other machine sees a change within about a minute.
- **It is never real time.** Each machine reads its own copy.
  Real time means one shared database every machine reaches live, which is the server spike 37 turned down.
- **A claim is claim, sync, read back.** Only the copy after a sync names the winner, and both machines agree on it then.
  A Job that claims an issue starts work only once the read back names it; the loser is told who has it, as a red claim's `409` does.
  The window is one sync, about 30 seconds, and neither machine has started work inside it.
- **Size is not a concern.** Armada's whole backlog is under 2 MB on the remote.

## Not measured

- Two real machines: both databases were on this Mac, over this Mac's network to GitHub.
- Beads' server mode, which a machine running many Jobs at once would need; every write here was embedded mode.
- The remote on Armada's own repository with `--allow-git-origin`: the spike used a separate repository, and push and pull should not differ.
- Growth over months of writes: the spike made 7 issues and 2 claims.
- Why the first push left the clone empty until a sync.
- Which of two changes to one field wins in general: here it was the later one, and the rule was not read in beads' code.
