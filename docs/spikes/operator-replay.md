# Spike: Is the Operator's one fresh call good enough to build on, and on which model?

**Answer: yes, on Sonnet, as a safe call that does little.**
Over 120 arrivals from last week, all 113 calls on each model returned a readable ruling.
It took a median 5.2 s on Sonnet and 5.9 s on Haiku, with the slowest call at 15.2 s, so every call was inside the 20 second first-write limit.
A call sends about 15,000 tokens, half of it the cached instructions, and Sonnet costs about 17 times what Haiku does per call.
No ruling would have acted against the owner where Fleet's rules let it act: every answer it gave to a question Fleet allows matched his choice.
It acted on very little, though.
It raised 39 of 46 questions on Sonnet, started 44 of 46 Jobs, and named an overlapping Job on most starts and nearly every plan.
On the three overlapping Jobs of 3 Nov it named the right Job at plan time but still started all three.
**Sonnet, because Fleet's rules would have refused 14 of Haiku's rulings against 4 of Sonnet's, and Haiku held or delegated 5 Jobs the owner started, against Sonnet's 2.**

Measured on 4 Nov 2026 against the design in the Operator page, revision 28, *Order of the build*, step 1.
The call stands on Autopilot's decision call in `crates/fleet/src/autopilot/deciding.rs`.
The arrivals and the owner's choices come from Fleet's store and stay outside the repository, and so does the replay script, since it reads them by path.

## The data

The week of 28 Oct to 3 Nov 2026, from the store backup taken at 01:15 UTC on 4 Nov, opened read only.
**This store does not hold the figures the design page counted.**
The page's 202 Jobs, 267 questions, 63 bug reports and the 3 Nov report *Live Jobs build the same thing with no knowledge of each other* are in none of this machine's backups or its live store.
The replay ran on what this store holds.

| Arrival | In the week | Replayed |
|---|---|---|
| Jobs dispatched from Bridge, not started by a Job | 53 | 24 |
| Jobs started by a Job | 22 | 22 |
| Question prompts with an option marked recommended, answered by the owner | 82 of 96 | 34 |
| Question prompts with options and none recommended, answered | 12 of 13 | 12 |
| Plans recorded, first version | 67 | 16 |
| Bug reports | 20 | 12 |
| **Total** | | **120** |

Jobs started from a terminal (16) were left out, since the Operator only hears of them after they begin.
Where the owner's choice is the measure, it is read from the row Bridge writes when he answers a question, which repeats the question and the option he pressed.
Six answered prompts were in his own words and match no option.
For every Job start the owner let it start: none of the 75 was refused.
For a plan, the measure is the plan was raised, since Fleet's rules raise every plan.
For a bug report, he kept 2 of the 12 and left the rest untouched, which is a weak measure.

## How it ran

- One call per arrival, or per burst: arrivals in one repository within 20 seconds of each other went in one call, numbered, which made 113 calls of 1 to 3 items.
- Each call was `claude -p` with no tools, no MCP servers, no settings files and no session kept, under `taskpolicy -c utility`, one at a time, Haiku and then Sonnet.
- The unchanging part came first, as the system prompt: the six ways, the rule to start unless there is a reason, the answer format, the owner's Logbook lines on how he works as of that arrival, and the armada repository's written decisions in `.claude/decisions/` dated on or before it.
- Then the state as of the arrival's time, rebuilt from the store: every live Job with its title, repository, plan groups and their paths and seen mark, its dispatch words where it had no plan, and the last 12 files it wrote.
- The meters were not recorded as percentages, so the call was given the tokens all Jobs sent in the last 5 hours and 7 days instead.
- The Line was empty, since it did not exist, and pull request state was not rebuilt.
- Then the replay's own last 20 rulings in that repository on the same model, each marked with the owner's correction where he chose otherwise before this arrival.
- Questions were offered answer and raise whatever Fleet's rules say, so the replay could count where the call would answer an item the rules keep for the owner.
- Fleet's rules were applied afterwards: a question from a Job with no plan or a seen plan, the "carry on in a new Job" question at 300k, and anything matching a destructive word must be raised.
  By those rules 69 of the 94 answered prompts in the week, and 31 of the 46 replayed, could only be raised.
- The seen mark is kept per plan in the store, not per group, so a seen plan marked all its groups seen.

## Rulings by kind and model

Agrees, disagrees, raised where the owner answered (a safe miss), and refused: a way Fleet would not have let through, so the item passes on as today.

| Kind | Replayed | Haiku agree / disagree / safe miss / refused | Sonnet agree / disagree / safe miss / refused |
|---|---|---|---|
| Question with a recommended option | 34 | 1 / 0 / 27 / 6 | 0 / 0 / 33 / 1 |
| Question with no recommended option | 12 | 3 / 0 / 6 / 3 | 3 / 0 / 6 / 3 |
| Dispatch | 24 | 18 / 3 / 0 / 3 | 23 / 1 / 0 / 0 |
| Job started by a Job | 22 | 19 / 2 / 0 / 1 | 21 / 1 / 0 / 0 |
| Plan | 16 | 16 / 0 / 0 / 0 | 16 / 0 / 0 / 0 |
| Bug report | 12 | 4 / 7 / 0 / 1 | 2 / 10 / 0 / 0 |
| **All** | **120** | **61 / 12 / 33 / 14** | **65 / 12 / 39 / 4** |

- **Questions.** Of the 15 replayed questions Fleet would let it answer, Haiku answered 4 and Sonnet 3, every one as the owner chose; the rest were raised.
  On the 40 replayed questions he answered with an option, its pick, answered or raised, was his on 26 for Sonnet and 25 for Haiku.
  On the 30 with a recommended option, Sonnet picked the recommended one 27 times and he did 21 times, so its pick adds nothing to the recommendation.
- **Starts.** Sonnet held one Job and delegated one dispatch, both naming a live Job whose plan or files touched the same paths, and neither matched a write clash Fleet saw later.
  Haiku held or delegated 5 and raised 4, which a start does not offer.
- **Bug reports.** Sonnet filed all 12, where the owner kept 2 of them and left 10.
  Haiku started a Job from 4, where the store records none started from any report.

## The dangerous rulings

A ruling that answered a question Fleet's rules keep for the owner, or answered differently from his choice.

| | Haiku | Sonnet |
|---|---|---|
| Answered a question the rules keep for the owner | 9 | 4 |
| Of those, differently from his choice | 4 | 2 |
| Answered an allowed question differently from his choice | 0 | 0 |

**Every dangerous ruling was one Fleet's rules refuse, so none would have acted.**

- Each model answered three "carry on in a new Job" questions at 300k, and got one right.
  Mostly it leaned on a Logbook line about one Job's choice, to be reminded at a higher limit, and applied it to every Job.
  On the other two he chose differently, once in his own words.
  The design already raises this question, and this is why it should.
- Haiku answered two armada questions from Jobs with no plan or a seen plan, and picked another option than his.
  One was a design question about which answer ends a workflow step, the other where a keyboard shortcut sits.
  Both are what the seen rule is for.

## The overlap case

The 3 Nov report is not in this store, so the overlap test is the three armada Jobs Fleet told of a write clash that afternoon, in `apps/desktop/src/renderer/src/job-panes/`.

| Job | At | Arrival | Haiku | Sonnet |
|---|---|---|---|---|
| Tmux-like PR Review Tabs | 14:10 | dispatch | started; no live armada Job had a plan or files | the same |
| Terminal Bugs | 15:30 | dispatch | named PR Review Tabs on the title, and raised it for another reason, which a start does not offer | named PR Review Tabs on the words "open terminals in a new tab", and started it |
| Terminal Bugs | 15:32 | plan | named PR Review Tabs, on plan paths | named PR Review Tabs, on plan paths and files written |
| Agent Opens Terminal in Tab, handed off by Terminal Bugs | 15:34 | start | named PR Review Tabs, on plan paths, and started it | named PR Review Tabs and Terminal Bugs, and started it |
| Agent Opens Terminal in Tab | 15:42 | plan | named Terminal Bugs, as the same work, and PR Review Tabs | named Terminal Bugs, as the same group on the same paths |

- **It sees the overlap and does not act on it.**
  Of the four arrivals after the first, Haiku named the Job that later clashed on all four and Sonnet on three, from plan paths and dispatch words; both started both Jobs.
  The rule to start unless there is a reason is doing what it says.
- On the last plan both models read a second overlap the clash notices did not record: the handed-off Job's group was still in Terminal Bugs' plan, the same work twice.
  Haiku's read said one of the two should be dropped.
- At plan time both overlaps would have been on the plan's row when the owner approved it.
- The other clash of the week, two armada Jobs on 2 Nov in `session-view/`, was missed on both models: the second Job's plan was read against two other Jobs.
- **The overlap it names is mostly noise.**
  It named some live Job on 25 of 46 starts and 14 of 16 plans on Sonnet, and 29 and 15 on Haiku, where Fleet later saw a write clash on 5 of those arrivals.
  Some it named were real duplicates no clash recorded, such as two Jobs moving the same views to the spine, so the precision is unknown, not low.

## Time and tokens

| Per call | Haiku | Sonnet |
|---|---|---|
| Wall time, CLI start included | median 5.9 s, p90 8.5 s, max 15.2 s | median 5.2 s, p90 7.0 s, max 10.3 s |
| The model alone | median 4.7 s, p90 7.5 s | median 3.8 s, p90 5.7 s |
| Tokens in | median 14,838, 11,140 to 17,311 | the same |
| Of which cached instructions | median 7,343 | the same |
| Tokens out | median 857, p90 1,500 | median 410, p90 690 |
| List price, as the CLI reports it | median $0.0021 | median $0.036 |
| The whole replay | $0.24 | $4.09 |

Sonnet is faster on the wall because it writes half as much.
The CLI writes its cache for an hour, so calls minutes apart in use read it as the replay did.

**Per day.** Last week on this machine would have made about 190 calls in 5 working days, 38 a day and 57 on the busiest, 28 Oct.
That counts every dispatch and Job start, every first plan, every report and the 27% of questions Fleet's rules let it answer.
It leaves out permission cards, turn ends, stalls, plans recorded again (55 more) and review requests, none of which were replayed.

| Per day | Haiku | Sonnet |
|---|---|---|
| 38 calls, tokens in | about 560,000, half cached | the same |
| Tokens out | about 33,000 | about 16,000 |
| List price | about $0.08 | about $1.35 |
| Busiest day, 57 calls | about $0.12 | about $2.00 |

The design page puts Sonnet at about three times Haiku per token; per call here it was about 17 times.

## Starting switch positions

On only where the replay's rulings match what the owner did, as the page's *What you control* sets it.

| Role | On Sonnet | Why |
|---|---|---|
| Evaluate | On | Readable on all 113 calls and inside every time limit |
| Answer | On | 3 of 3 answers matched him, and none answered an allowed question wrongly. It raises the other 12 allowed questions, so it saves little |
| Raise | On | The fallback every other role leans on |
| Dispatch | Off | 44 of 46 starts matched; one hold and one delegation he did not make |
| Delegate | Off | The one delegation was not what he did |
| File | Off | It filed 12 of 12 reports where he kept 2 |

On Haiku the same, except that Answer is not On by the same rule, since 9 of its answers were to questions the rules keep for him.

## What this contradicts in the design

- The Answer table says a question with a recommended option, from a Job on groups not seen, is answered.
  On this store the owner took the recommended option on 11 of the 19 such prompts he answered, another option on 3, and wrote his own answer on 5.
  Answering with the recommendation would have been wrong on 8 of 19.
  The call did not do that, which is why it raised them, and why Answer saves little.
- The seen rule leaves the call 25 of 94 answered prompts, not most of the questions with a recommended option the page expects to stop reaching the owner: 48 came from a Job with no plan yet.
- Sonnet against Haiku is about 17 times per call, not 3.

## What was not measured

- The figures and the report on the design page, which are not in this machine's store.
- Permission cards, turn ends, stalls, review requests, Later items and red main, which the replay did not include.
- The 5 hour and weekly meters as percentages, which the store does not keep, and pull request state.
- A plan's state at the time of an earlier arrival: the store keeps the latest plan, so a Job's groups are the ones it ended with, shown from its first recording.
- The seen mark per group, which the store keeps per plan.
- Destructive questions, found here by words such as delete, drop and overwrite rather than by Fleet's own screen.
- Whether a hold would have saved tokens: no Job was held, so what the second Job would have done is unknown.
- Fleet keeping a call process started and waiting, which the page expects to take the CLI's start off each call.
- The owner's corrections to rulings, which the replay fed from his real choices; in use he would correct only what he sees.
- Repeat runs: each arrival was ruled once per model.
