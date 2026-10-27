# Who keeps a Job's task list true: the agent, or Fleet?

**Decided 2026-10-27.** Bridge's Now panel reads a Job's task marks, and an audit
of 70 Jobs from 22 to 27 Oct found they could not be trusted. 419 of 828
finished tasks went from pending to completed without ever being in progress.
39 of 64 Jobs had two or more tasks in progress at once; Job c2d23c4c had ten,
one untouched for 244 calls. The longest stretch with no task update was a
median of 44 calls per Job and 487 at worst. Claude Code's own reminder to use
the task tools is in every one of those transcripts.

**Chosen: Fleet marks a task itself when it has proof, and tells the agent
once.** Four questions, each answered with the recommended option.

| Asked | He chose | The cost he took |
|---|---|---|
| What makes a task in progress stale | Edits landing in files that belong to another group of the plan. Where the plan cannot tell, because the group names no files of its own, 40 calls without the task being touched | A group with no files of its own goes stale only by the count |
| Who fixes a wrong mark | Fleet marks it from proof (a commit naming the task, the group's pull request merged, edits in a task's files with nothing in progress) and tells the agent once, naming the task | A Fleet message in the thread for each fix, and a wrong guess shows until the agent's next mark replaces it |
| Several tasks in progress at once | The newest leads; the others show stale with their age, never done. Two run side by side only while two subagents each edit their own group's files | A task finished and never ticked reads stale until proof or the agent closes it |
| Whether Fleet's mark looks different | The same mark, with a tooltip naming the proof: *Marked by Fleet: commit a1b2c3 names it* | He cannot tell a Fleet mark from the agent's at a glance |

**What it keeps.** *Detect, don't trust agents*: the agent's task tools enrich
what Fleet sees and no longer decide it alone. Fleet's mark holds until the
agent's own list changes that task, so a list that still says pending after a
commit names the task does not undo the mark.

## What would make this wrong

**That Fleet's proof is wrong often enough to mislead.** A commit subject close
to a task's words marks it done. If the agent commits half a task under its
title, Bridge says done early.
