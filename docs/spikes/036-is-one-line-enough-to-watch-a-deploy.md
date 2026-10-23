# Spike 36 — Is one line, with parts, enough to watch a deploy?

**Answer: yes, with parts, if the script carries two things the line has no field for.**
Replayed through the design's wake rules, three real deploy runs woke the agent 7 times in 52 readings, each at a moment that mattered: 1 failure it called ours, 4 it called not ours and set aside, and 2 ends.
**Without parts the line is noise: all three runs read `failed` within 5 minutes of the push, while 5, 29 and 12 deploys were still going, and none of them could ever read as finished.**

Measured 22 Oct 2026 against the Job that `docs/design/fleet-watches-ci-and-deploys-for-a-job.md` opens with, which pushed three commits to a shared QA environment between 19 and 22 Oct.
The question is the design's own *What would make this wrong*.

## How it was replayed

The Job's repository runs its CI on one provider and its tests and deploys on another, a deploy provider.
One script per provider was written, each taking a commit and printing the design's one line.
Both name another repository and its internals, so they are kept on the person's machine in `~/.armada/machine/watches/spike/` and not committed here.

- **The deploy provider's script** finds the push run from the commit's GitHub status and reads every task with the provider's CLI.
- Its parts are one per service and kind: a service's checks, and a service's QA deploy.
- **The CI provider's script** reads the build and its jobs from the provider's REST API.
- It could not be run: this machine reaches that provider only through a hosted MCP server's own login, and holds no token for its API.

The deploy provider still held all three runs, with each task's start and finish time.
The script took a time and read the run as it would have stood then: a task not started is `waiting`, one started and not finished is `running`, one finished is its result.
That assumes every task is visible from the run's first minute; a live read may see an embedded run's tasks only once it spawns.
A reading was taken each minute from the push to two minutes after the run ended, as a watch every 60 seconds would.

The wake rules are the design's: wake when the watch turns `failed`, ends `passed`, gives up or breaks three times.
At each `failed` wake the agent's verdict was taken from what the Job's agent actually said about that failure in its transcript, and a `not ours` set those parts aside.

## The runs

| Push | Readings | Wakes | Each wake, and the verdict the transcript gives | Deploy parts not done at the first `failed` |
|---|---|---|---|---|
| First, 19 Oct | 4 | 1 | Minute 4: service A's schema check failed. **Ours**: the QA branch was far behind main, and the agent merged main into it | 5 |
| Second, 19 Oct | 28 | 3 | Minute 4: a test in service B, which the change did not touch. **Not ours**. Minute 6: service C's QA deploy failed. **Not ours**. Minute 28: ends passed | 29 |
| Third, 22 Oct | 20 | 3 | Minute 5: a unit-test task in service D, which the change did not touch. **Not ours**, and the person confirmed it. Minute 8: service E's build, red on main for every branch, which skips E's QA deploy. **Not ours**. Minute 20: ends passed | 12 |
| **Total** | **52** | **7** | 1 ours, 4 not ours, 2 ends | |

What the Job really did, for comparison:

- The first push's failure was first read 46 minutes after it happened, when the person asked.
- The second push's two failures were first read 47 hours after the push, when the person asked.
- The third push's first failure reached the agent within a minute through Fleet's pull request notice, because the pushed commit was also an open pull request's head.
  The run ended 19 minutes after the push, and the transcript stops 19 minutes after that with its end still unread.

The largest line was 81 parts and 5.3 KB, from the second push.

## The same runs without parts

The deploy provider's run-level status turned `failure` at 2, 3 and 4 minutes, with the words "Failed (still in progress)", and stayed there until the run ended.

| Script choice | Wakes a run | What it hides |
|---|---|---|
| The run-level status as `state` | 1 at the first failure, then 1 at give-up after 2 hours | Whether the change's own deploys ran. A `not ours` has nothing to set aside, and `failed` never turns into an end |
| `running` until the run ends, then its result | 1, at the end, 14 to 26 minutes after the first failure | Which part failed. All three runs ended `failed`, each with every deploy of the change done |

## What the line could not express

These are what the scripts had to carry, and what neither could.

1. **That the provider has finished.** `failed` is the same whether the run is still going or done, so a script has to invent a part for it.
   Without that part, a run whose failures were all set aside would end `passed` as soon as its known parts passed, which is wrong when tasks appear late.
2. **A skipped step.** 46 of the CI build's 77 jobs and 13 of the first deploy run's tasks were skipped as normal, mostly deploys to other environments.
   A skipped QA deploy leaves a service on old code.
   The script has to know which skip is a failure, and show it as `waiting` until the run ends.
   Shown only at the end, it was a part the agent could not set aside earlier, and it cost the third push one more wake.
3. **That someone else replaced the deploy.** Someone reset the shared QA branch to main on 21 Oct, two days after the second push's watch would have ended `passed`.
   The change left the environment and nothing said so until the third push found it 22 hours later.
   A watch follows a commit's run, so it cannot see this.
   A second watch on what the environment is running could.
4. **A retry.** One CI step ran 7 times on the first push's build, and every attempt failed.
   The line has no attempt, so the script keeps the latest.

## What else the replay found

- **A set-aside lasts one watch.** A step that fails on every build, like that CI step, costs one wake on every push.
  The design keeps standing rules with the agent's lessons, so the agent answers each wake quickly, but it is still woken.
- **A watch and a pull request notice can tell one failure twice.** The third push's commit was a followed pull request's head, so Fleet's notice and the watch would both have woken the agent in the same minute.
- **`Ours` ends the watch while deploys run on.** On the first push, 5 deploys were still going when the agent called the schema failure ours, and all 5 passed.
  The environment got the change, and nothing told the agent.
- **The CI provider gave no reading at all.** Its script exits with no token, a broken reading, so its watch would stop after three.
  GitHub held no status from the CI provider for any of the three QA builds, so a script cannot read it there either.

## Not measured

- The CI provider's builds for the second and third pushes: the transcript recorded only `running`, and its API was unreachable.
  Its first build is known only by its counts: 1 job failed, 29 passed, 46 skipped.
- Live intermediate readings: every state above is derived from task times, not read while the run was going.
- The slowing after 30 minutes unchanged: no run lasted that long.

## What would fix it

The design holds for when to wake; the gaps are in what the line can say and in how long a set-aside lasts.

- **A field for finished**, or a rule that a script must say so through a part, so `failed` and done are told apart.
- **Set-asides that a recipe carries to its next watch**, for a step that fails on every run.
- **One notice for one failure**, when a watch's commit is also a followed pull request's head.
- **A watch on what an environment is running**, beside the one on a commit's run, so a replaced deploy is news.
