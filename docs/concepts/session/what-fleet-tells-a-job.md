# What Fleet tells a Job

**Part of:** [Job](../session.md).

**A Job never has to be told to watch its pull request.** When a pull request a Job holds fails its checks, conflicts with the base, leaves the merge queue, is marked unmergeable, or merges, Fleet sends that Job one message, once.
It names the pull request, what happened and the branch.
For failed checks it lists each failing check with its log address, and the last lines of the first two logs.
A merge tells the Job to call `finish_up`, which releases its slot and deletes the merged branch, and never to delete the branch by hand or ask the owner to: ending the Job parks its slot and keeps the branch.
`docs/concepts/fleet.md`, *Telling the owner of a pull request*, has the triggers.

| Job | What happens |
|---|---|
| Hosted | The message is a turn: it wakes the Job and starts its process if it was ended for being quiet. The thread draws it as a message from **Fleet**, the voice another Job's message has, named `Fleet`, so it is never the person's |
| Terminal | Fleet cannot push into one. The message is held for its mod, prefixed `Fleet:`, and handed over when the mod next asks. A mod that is not asking leaves it unsent and Fleet tries again at the next reading |
| Ended | Nothing is sent |

**A Job is told when what it wrote would conflict.** When Fleet finds that the files a Job wrote conflict with another Job's, or with the base after it moved, it sends that Job one message: the files, who the other is and how to reach it.
The Job is not paused and the message is sent once per set of files.
`docs/concepts/fleet.md`, *Write clashes*.
Bridge marks the Job's row and puts the swords glyph on its Bays key, and the Bays panel names the other holder and the files, from the standing clashes Fleet lists.
A file in that list opens as a sheet of the Job's change to it and the other holder's, each from where the two split, with the hunks that collide marked in both; Fleet reads the two patches from git when the file is pressed (`read_clash_file`, `docs/concepts/fleet.md`, *Write clashes*).
A Fleet that cannot read one leaves the sheet with a sentence and no diff.

| The other is | The message names |
|---|---|
| A Job | Its address, `s-` and the first eight characters of its id, and its branch, and says to use `SendMessage` to that address |
| The base | The commit on it that changed the first file, and says to merge it into the branch |

A terminal Job reports the files it wrote through the mod's `touched` fact, as paths from its directory.

**A Job that held a slot is told when the slot is stranded.** When a slot a Job held last has work in it and nothing holds it now, Fleet sends that Job one message, once.
It names the slot and its branch, what the slot holds, and asks the Job to commit, push and release it.
`docs/concepts/fleet.md`, *Rescuing a stranded slot*.

**A Job whose slot is far behind its base is told to merge it.** When the checkout a Job holds is 20 or more commits behind the base, Fleet sends that Job one message: `slot-4 (branch armada/x) is 684 commits behind main. Merge main into your branch before your next push.` The count is the pool's own `behind`, and 20 is the figure Bridge's slot row starts from.
It is sent once.
The Job is told again only after the slot has fallen below 20 and then behind again, and a Fleet that restarts may say it once more.
A Job's slot, a free slot, a stranded slot and a subagent's bay are not told, and the owner is not called: merging the base in is the agent's work.

**A Job whose usage crosses a token rule is told once.** Context over 300k, subagents running most of the last hour, a cache hit rate under 70%, or 20 turns of reading on opus each send the Job one message with the figure and one action.
The Job is told again only after it fell back under the limit and 30 minutes passed.
Fleet never ends, forks or moves the Job for this, and a model change is only ever offered in the words.
`where-the-tokens-went.md` has the rules and the words.

**A Job can dispute a notice it thinks is wrong.** A hosted Job that finds Fleet's notice does not fit what it was doing calls `dispute_fleet` with `why`, the reason in a sentence.
Fleet ties the dispute to the Job's latest message in Fleet's voice, keeps it, and the Job carries on.
A Job Fleet has told nothing, an ended Job and an empty `why` are refused.
A terminal Job cannot dispute, because Fleet keeps no thread for it to tie the dispute to.
`crates/ipc/operations/dispute_fleet.toml` has the operation.

**Fleet reads the replies of a Job that did not call.** A notice that asks something is read once the wait has passed with no dispute and the Job has replied.
News, such as a merge, asks nothing and is never read.
One cheap model call reads up to the first five replies the agent made after the notice, and keeps a dispute where it pushed back: said the notice was wrong, did not apply, or would not be followed.
Agreeing, following it or not mentioning it is not pushing back.
A notice is marked read before the call, so neither the next pass nor a restart reads it twice.
A notice the Job disputed by the call is marked read there, and one whose Job ended is marked read without a call.

| Setting | Holds |
|---|---|
| `limits.disputeReadMinutes` | How long after a notice its replies are read. 10 minutes, 1 to 240, live |
| `models.dispute` | The model that reads them. The Judge's cheapest tier |
| `features.disputeReviews` | Whether a dispute starts a review and replies are read for one. Off as shipped, live |

**Dispute reviews are off as shipped.** Off, `dispute_fleet` still keeps the dispute and writes its `disputed` row, but no review is started or joined.
No notice is kept to be read and no kept one is read, so no model call is made.
A notice kept while the switch was on stays unread and is read once it is on again; one sent while it was off is never read.

**A dispute starts a review Job in Armada's own repository**, where dispute reviews are on. It is on the `bug` workflow, titled *Review a disputed Fleet notice*, and its brief carries the notice, the Job's reason, how the dispute was spotted (called, or read off its replies) and the thread around the notice.
The review decides who was right.
Where Fleet was wrong, it fixes Fleet and lands the fix.
Where the Job was wrong, it says why in one reply and ends, changing nothing.
Fleet does not record which code sent a notice, so the brief gives the review the notice's fixed words to search `crates/` for.

**A second dispute of the same notice joins the review that is open.** Two notices are the same when their words match with every number ignored, so a rule told twice with different figures is one notice.
The open review is sent the new dispute as a message and no second Job starts.
A Fleet that does not serve `armada` keeps the dispute and starts nothing, and says so on its stderr.

**Bridge marks the notice.** Fleet's card in the thread carries a Disputed mark, and a line under it gives the reason, how it was spotted and a press that opens the review Job.
Where no review was started the line says so and draws no press.
`packages/components/src/compositions/SessionThread/DisputedRow.tsx` draws it.
