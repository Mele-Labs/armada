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
