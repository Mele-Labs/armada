# Not built

**Part of:** [Job](../session.md).

**The Manifest switch is drawn on mock data only.** The chip is read off the wire, but the form's *Pull request owners* switch still runs on `draft.prOwners`, so a real Bridge draws none and the key is set in `armada.yml`.
A row is deleted by a Fleet that saw the pull request open and then gone: one that merged while no Fleet was reading keeps its row, and so does a claim that was given back.

**The `/` list is empty until an agent has started.** `hosted.commands` is the names in the stream's `init` line, slash commands then skills, and a job whose own process has not started is given the last one any job read.
It is held in memory, so after a Fleet restart it is empty until a process starts, and the harness says nothing of what each command does, so the list draws names only.

**A job that dies without ending stays `live`.** Its `last_seen_at` is what says it has gone quiet; nothing yet checks the process.

**A message another job sent a terminal job is not drawn.** A hosted
job's thread carries what another hosted job wrote to it; a terminal
thread skips the line, and the mod reports the message as a count and never
its text.

**Hosted jobs in different permission modes may not wake each other.** The
tool says a job in another mode holds a cross-session message for approval.
Not measured.

**A message names where it went and not which job**, because the harness does not say: a delivery says whether it came from a peer or a teammate, and never a job id.
