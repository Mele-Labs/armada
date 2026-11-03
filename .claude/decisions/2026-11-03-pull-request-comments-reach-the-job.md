# How does a reviewer's comment reach the Job that owns the pull request?

**Decided 2026-11-03.** Fleet told a Job when its pull request failed checks,
conflicted or merged, but never when someone commented. The panel's
Conversation also left out file and line comments, though Fleet already read
them for the review tool.

**Chosen: Fleet files every comment where the Job can read it and sends one
line pointing at the file.** The agent reads or greps the file only if it
decides to. He asked for the file over a notice carrying the whole text, so
a busy Job's context does not fill with comments it may not act on.

| Asked | He chose | The cost he took |
|---|---|---|
| Whose comments wake the Job | Human reviewers only. Bots and his own comments are filed without a notice | A bot finding, such as a review bot's line comment, waits until the agent or he looks |
| A Job taking on a pull request that already has comments | All of them are filed, with one notice | One extra notice per pull request taken on |
| The same comment seen twice | Sent again only when it was edited | An edit that changes one character sends a notice |

**What it keeps.** A Job never replies on the pull request for him. Pressing
Evaluate on a comment asks the Job for a verdict and a proposal in Needs you,
and nothing is posted to the forge.

## What would make this wrong

**That bots carry the findings worth acting on.** If review bots' line
comments turn out to be what he wants a Job to weigh, the bot rule moves.
