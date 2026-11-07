# The scripted person

Every answer in this run was written before the Job started. Nothing was judged live, and nothing was changed after the run began. The script is `person.py` and `rules.json` in this folder. The habits behind each rule are in `.claude/skills/how-the-owner-decides/SKILL.md`.
Copy of the person from spike 48 with the fixes below.
A question no rule meets gets the option marked Recommended, or if there is none a sentence, and the act is logged as `no_rule`. It never takes the first option by default.

## Fixes from run 48

| Fix | Run 48 | Now |
|---|---|---|
| Merge | Merged when asked, whatever was open | Merges only when the keeper says `merge_ready`: every gate before Deliver closed, and the branch tip is the commit Checks passed at. Asked earlier, it answers which gates are still open |
| Held gate | Nothing raised, the Job sat | The keeper raises an item (Approve, Skip this group). The person acts on it by rule, below |
| Quiet stop | Counted the `done` item as an open item, so the rule never fired | Items exclude `done`. Quiet means 7 minutes with nothing running, nothing waiting and no change |

## The acts that do not depend on the agent's question

| Act | When | Why |
|---|---|---|
| Approve plan v1 | As soon as it appears | The run is about gates, not plan quality. A plan that breaks the template is logged, not sent back |
| One note on the design page | Revision 1 of the first page under `.armada/documents`: "Say what this does not cover." | A real habit of asking for a design's limits. It makes the document gate close on the second revision |
| Approve revision 2 | The next revision, as written, without checking whether it took the note | Whether it took the note is scored from the page |
| Change request | Once, after the Job's own branch is first pushed once the design note has closed | Written in `problem.md` (armada-mttk). Small, real, adds a build group on a file neither part touches |
| Approve the new plan | Every plan version | |
| Merge | When asked, and only when Deliver is live | See the table above |
| Permission prompts | Allow unless the text names `gh bd curl wget ssh sudo`, `rm -rf`, `--force`, a push to main or a path under the real home | As in the rig |
| Delivery | Keep local | No forge |

## The rule for a held gate item

The keeper raises one item per hold. By gate:

| Held on | Condition for Approve | If met | If not met |
|---|---|---|---|
| Plan the change | A plan version is recorded and waits | Approve it | Say: record the plan |
| Write the design note | An HTML page under `.armada/documents` has an open revision | Act on it as on any document (note once, then approve) | Message: nothing to approve, and what is missing (no HTML page exists, or only a file was shown), and to write it with the writing-a-document skill |
| Checks | Not a person gate | Never approved | Message: what Checks lacks |
| Deliver | `merge_ready` | Merge | Message: which gates are still open |

Skip this group is never used. A person who skips a group has stopped caring about the gate, and the run is about the gate.

## The rules for questions the agent asks

The first rule whose pattern matches wins; `rules.json` has the patterns.

| Rule | The answer | The owner habit behind it |
|---|---|---|
| Custom tab state key | Key it by Job and tab, as the issue says, and drop it with its Job. Nothing to migrate | The issue is the brief. "Absolutely no one is using armada so we don't need to be this precise" (`2026-09-22-armada-is-pre-alpha.md`) |
| What "gone" means | Drop only when Fleet no longer lists the Job. Not on end | "Pay for the record being right" and "A screen never says a thing it does not know". An ended Job can still be opened and forked; losing its layout is a loss he would see, and a key that lingers costs nothing he sees |
| The pull request and operator families | Leave them alone unless the note shows a clear way for them to be gone, and say so | "Match the ceremony to the stakes" and the standing wish to stay in the asked scope |
| Migration, a setting, a flag, old keys | None | Pre-alpha: no compatibility ceremony |
| Doing more, other kept keys, drafts | No, stay with the two issues | An agent's leftovers get filed, not built |
| The sweep at start | Wait until Bridge has read the list of Jobs; never sweep an empty list | Same record as the "gone" rule |
| No rule met | The option marked Recommended | He picks the recommended option most of the time |
| No rule met and nothing marked | "Take the smaller change that never drops a layout of a Job that is still listed." | The "gone" rule, as a sentence |

## What the script cannot do

It answers the pattern, not the question. An agent that asks something in words the patterns do not match gets the fallback, and that is logged. A real owner would read the question.
