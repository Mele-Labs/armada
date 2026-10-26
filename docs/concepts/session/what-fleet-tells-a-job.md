# What Fleet tells a Job

**Part of:** [Job](../session.md).

**A Job never has to be told to watch its pull request.** When a pull request a Job holds fails its checks, conflicts with the base, leaves the merge queue, is marked unmergeable, or merges, Fleet sends that Job one message, once.
It names the pull request, what happened and the branch.
For failed checks it lists each failing check with its log address, and the last lines of the first two logs.
A merge tells the Job to call `finish_up`, which releases its slot and deletes the merged branch, and never to delete the branch by hand or ask the owner to: ending the Job parks its slot and keeps the branch.
`docs/concepts/fleet.md`, *Telling the owner of a pull request*, has the triggers.

| Job | What happens |
|---|---|
| Hosted | The message is a turn: it wakes the Job and starts its process if it was ended for being quiet. The thread folds the turn into one dim row: the notice, the agent's reply and its tool runs, opened by a press. The person's own message ends the fold. The row is named by the notice's first sentence, up to the first full stop and space or line break, cut to one line, with the whole notice on hover, and a check marks it handled once the agent replied. A notice that carries `notice` keeps its Disable menu on the folded row. The agent's last message in the turn draws outside the fold, as any agent message, because it is written to the person. A turn still waiting on the person, or whose notice was disputed, stays open as a message from **Fleet**, named `Fleet`, so it is never the person's |
| Terminal | Fleet cannot push into one. The message is held for its mod, prefixed `Fleet:`, and handed over when the mod next asks. A mod that is not asking leaves it unsent and Fleet tries again at the next reading |
| Ended | Nothing is sent |

**A Job is told when a watch it started changes.** A watch is a command Fleet runs for the Job (`docs/concepts/watch.md`), and Fleet sends one message when it turns failed, ends passed, ends not ours, gives up or breaks.
`waiting`, `running`, a pass not yet finished and a stop tell nothing, and a weighing tells nobody, since the agent gave it, save one that ends the watch not ours.
The message goes the way a pull request's does, by the table above.

| The watch | The message opens |
|---|---|
| Turns `failed` | `QA deploy failed: deploy-api failed.` |
| Ends `passed` | `QA deploy passed.` |
| Ends `not_ours` | `QA deploy ended not ours: provision failed.` |
| Gives up | `QA deploy gave up after 2h. It last said: two of three services done` |
| Breaks three readings in a row | `QA deploy stopped: its command failed three times.` |

**A failed notice names the counted parts that failed, then the link and a second line.**
Parts the agent set aside are left out, and a watch with no parts quotes the line's own sentence: `QA deploy failed: <says>`.
The reading's link follows after a space, and the second line names the watch for `weigh_watch`, where `w-…` is its id:

```
Weigh it with weigh_watch on watch w-…: ours ends it, and not_ours sets those parts aside and it carries on.
```

**A not ours notice names the set-aside parts that failed, then the link, each part's sentence, and a last line.**

```
provision, set aside: <the sentence that set it aside>
It did not pass, so do not report it as passed. The person settles it from Needs you.
```

**A gave up notice quotes the last thing the watch said.**
A watch that gave up before any reading says `QA deploy gave up after 2h without a reading.`
**A broken notice ends with the command's last output**, its last 15 lines and no more than 1,200 characters, or the reason where it printed nothing.
Each is told once for each change in what failed, so the same failure read again is not news, and a restart repeats nothing.
A Job that cannot take one now is offered it again a minute later.

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

**A dispute starts a review Job in Armada's own repository**, where dispute reviews are on.
It is on the `bug` workflow, and its brief carries the notice, the Job's reason, how the dispute was spotted (called, or read off its replies) and the thread around the notice.
It is titled *Disputed:* and two to four words naming what the notice is about, as *Disputed: opus spend notice*, which one cheap call to `models.dispute` names from the notice.
Where that call fails or answers anything but a few words on one line, the title is *Disputed notice*.
It is linked under the Job that disputed it, so the left panel nests it there, and the Job's thread gains no *Started* line beside its *Disputed* one.
The review decides who was right, with evidence from Armada's code, and reports the verdict in a few lines.
It then puts two to four choices that fit the case to the person.
Where Fleet was wrong they are such as fixing the notice or the rule behind it, sending it less often, or leaving it; where the Job was wrong, changing what hosted agents are told, or leaving it.
It changes nothing and opens no branch until the person picks, then does what was picked on the `bug` workflow.
Fleet does not record which code sent a notice, so the brief gives the review the notice's fixed words to search `crates/` for.

**A second dispute of the same notice joins the review that is open.** Two notices are the same when their words match with every number ignored, so a rule told twice with different figures is one notice.
The open review is sent the new dispute as a message, to weigh in its verdict or say whether it changes the choices, and no second Job starts.
It stays under the Job that first disputed the notice.
A Fleet that does not serve `armada` keeps the dispute and starts nothing, and says so on its stderr.

**Bridge marks the notice.** Fleet's card in the thread carries a Disputed mark, and a line under it gives the reason, how it was spotted and a press that opens the review Job.
Where no review was started the line says so and draws no press.
`packages/components/src/compositions/SessionThread/DisputedRow.tsx` draws it.
