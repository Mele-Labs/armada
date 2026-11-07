# The scripted person

Every answer in this run was scripted before the Job started. Nothing was judged live, and nothing was changed after the run began. The script is `person.py` and `rules.json` in this folder; this page says what each rule is and which of the owner's habits it comes from. The habits are in `.claude/skills/how-the-owner-decides/SKILL.md`; each decision file named below was read before it was leaned on.

Spike 46's person took the first option of a question it had no rule for, and the agent edited a product file because of it. This one never does that. A question no rule meets gets the option marked Recommended, or if there is none a sentence, and the act is logged as `no_rule` so the write-up can say so.

## The acts that do not depend on the agent's question

| Act | When | Why |
|---|---|---|
| Approve the plan | Every plan version, as soon as it appears | He approves a plan that matches what he asked, and the run is about gates, not plan quality. A plan that breaks the template is not sent back; the keeper logs it |
| Send the design note back once | The first revision of the first document: "Say what this does not cover." | The one note is a real owner habit of asking for the limits of a design, and it makes the document gate close on the second revision, not the first |
| Approve the second revision | The next revision of that document, as written, without checking whether it took the note | The script plays the person who reads it and agrees. Whether the agent took the note is scored from the page |
| One change request | After the Job's own branch has been pushed once the design note is closed, or the first build group's tasks are all done | Written before the run, in `problem.md`: drop the Custom tab's state with its Job and key it by Job and tab (armada-20v4). It is small, it is real, and it adds a build group |
| Approve the re-recorded plan | As above, every plan version | |
| Merge | When an item or a question asks for the branch to be merged: into the bare origin's `main` | The merge is his and he does it once asked; there is no pull request and no CI here |
| Permission prompts | Allow unless the text names `gh bd curl wget ssh sudo`, `rm -rf`, `--force`, a push to main or a path under the real home | As in the rig |
| Delivery | Keep local | No forge |

## The rules for questions the agent asks

The first rule whose pattern matches the question wins; `rules.json` has the patterns.

| Rule | The answer | The owner habit behind it |
|---|---|---|
| What "gone" means | Drop only when Fleet no longer lists the Job. Not on end | "Pay for the record being right" and "A screen never says a thing it does not know" (`2026-09-22-judge-and-check-sign-the-record.md`, `2026-09-22-no-verdict-recorded.md`). A Job that ended can still be opened and forked; losing its layout is a loss he would see, and a key that lingers is a loss nobody sees. Rigour about what is stored, speed about what is shipped |
| The pull request and operator families | Leave them alone unless the design note shows a clear way for them to be gone, and say so | "Match the ceremony to the stakes", "Retire the old line; do not run two", and the standing wish to stay in the asked scope. The issue is about a Job's key |
| Migration, a setting, a flag, old keys | None | "Absolutely no one is using armada so we don't need to be this precise" (`2026-09-22-armada-is-pre-alpha.md`): no compatibility ceremony |
| Doing more, other kept keys, drafts | No, stay with pane layouts | An agent's leftovers get filed, not built. The change request is the only addition and it comes from the person |
| The sweep at start | Wait until Bridge has read the list of Jobs; never sweep an empty list | Same record as the first rule: a list nobody has read says nothing is gone |
| No rule met | The option marked Recommended | "He picks the recommended option most of the time. So the recommendation is doing the work" |
| No rule met and nothing marked | "Take the smaller change that never drops a layout of a Job that is still listed." | The first rule, as a sentence. He answers with a sentence when no option fits |

## What the script cannot do

It answers the pattern, not the question. An agent that asks something the patterns do not name, in words they do not match, gets the fallback. That is logged. A real owner would read the question; this script does not.
