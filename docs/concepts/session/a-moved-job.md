# A moved job

**Part of:** [Job](../session.md).

**A Job started in the wrong repository is moved, not started again.** `move_session { session_id, manifest_id }` (`POST /sessions/move`) ends the Job and starts its whole conversation again in another served repository, under the same title, and answers the new Job's record. It is a fork across repositories, in one call: `start_session`'s `fork` with no `at`, after the old Job is ended.

| | |
|---|---|
| Refused | Before anything is ended or written. An unknown Job, 404 `fleet.no_such_session`. A repository Fleet does not serve, 404 `fleet.no_such_manifest`. The Job's own repository, 409 `fleet.session_move_same`. A terminal Job whose mod is still asking, 409 `fleet.session_move_listening`: it runs in the owner's terminal, which Fleet cannot move |
| The old Job | A live hosted one is closed as Close closes it: its process ends, its slot is parked (what is in it is committed to its own branch, nothing is pushed) and its branch is kept. An ended one, or a terminal one whose mod stopped asking, is left as it is |
| The new Job | A fork: its own id, ledger and slot, in the target repository's main checkout, leasing a slot at its first write. It keeps the old model, effort and mode |
| Ledger | The `forked_from` and `forked_to` rows of *A forked job*, `spent`, with `detail.moved` on both: the repository the Job came from on `forked_from`, the one it went to on `forked_to`. `forked_from` also carries `detail.branch`, the old Job's branch or branches, where it had any. A fork has neither, which is how Bridge tells a move from a fork |
| Told | **Every process of the new Job has a move note in its system prompt** in place of the fork note: moved from repository <root> to repository <root> at <time>, its conversation is a copy of <title> (<id>) which has ended, it now works in the second. Its slot, its branch (<name>) and the files it had changed stayed in the first. What the old Job started is not its own, a task-notification about it is not acted on, relaunched or reported, and it does not write in the first repository. The note is read from the ledger at each start, so it survives a Fleet restart |

**The conversation follows the directory.** The CLI keeps a conversation under the folder of the directory it ran in. The new Job's first process starts in the target repository's checkout and `bring_conversation_to` links the old transcript, found by id wherever it is, into that folder before `--resume <old> --fork-session` runs, so the copy is found across repositories.

**What stays behind is not moved.** The old repository's slot, branch and changed files are the old Job's, and the old Job is ended; the branch is still there for a person or a later Job to pick up. Moving a Job does not carry its pull requests.

**An agent may move its own Job** (`agent_access` Yes). Its process is the one that ends, so the answer never reaches it; the new Job is listed and Bridge opens it. The call names the Job, so an agent could name another's; it is held to its own by the instruction, not by Fleet.

**Bridge draws it from the ledger alone.** A Job's repository is a chip in its head, on the flight deck's stage, and where Fleet serves `move_session` it opens the other served repositories; a pick asks in the same layer, as a caution, what stays (the branch and its files) and, for a live Job, that it ends. A `forked_to` with `moved` is "Moved to" and a `forked_from` with `moved` is "Moved from", each a row that opens the other Job: the first ends the old thread and the second starts the new one, whose conversation is the old one's rows read in place and not folded as a fork's is. A Job with a `forked_to` that carries `moved` shows its repository as text, so it is moved once. A terminal still listening has every repository drawn off with the fact on its tooltip; Fleet's refusals are the composer's toast (`fleet.session_move_same`, `fleet.session_move_listening`, and the two 404s).
