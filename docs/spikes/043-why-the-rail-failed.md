# Spike 43: why workflows failed

Nobody kept a record of which step a Job was on.
The agent never held the steps: it worked from its own plan, task list, subagents and walks.
Fleet never stored the step either: it derived it afresh each time a Job's row was drawn, from facts that could vanish or mean two things (`crates/fleet/src/rail_following.rs:4`, "The step itself is derived while the row is drawn").
Whenever the derived answer changed, Fleet told the agent to do that step's work.
Every disputed rail notice is a moment where the derived step and the Job's real state disagreed.

Measured 6 Nov 2026 from the owner's store, read with `sqlite3 -readonly`.

## The numbers

165 Jobs ran on the rail (`session_rail.visits` not empty).
73 of them disputed where it put them, 205 disputes in all, and 31 were moved to an earlier step at least once.
No dispute was ever reviewed (`fleet_disputes.review` is empty on every row).

| What moved the step | Disputes | Jobs |
|---|---|---|
| A step entered, and its instructions sent | 78 | 61 |
| The order plan tasks were ticked | 50 | 27 |
| A push read as leaving a step | 32 | 31 |
| An edit read as skipping a step | 27 | 27 |
| A turn end read as stalling on a step | 18 | 18 |

## The 78 step entries

51 came after the Job's pull request had merged, and the agent said so ("#2801 has merged into main, so the notice putting the job back on the Implement step is stale").
`rail::current` (`crates/fleet/src/rail.rs:426`) answers "landed" from the build record or the ledger (`landed_by`, `rail.rs:221`).
When `finish_up` released the slot, or a read after it leased one again, the build record started afresh, `landed` read false, and the Job fell back to the last step whose entry still held, usually Implement or Refine.
A derived position has no memory, so a fact that drops out moves the Job backwards.

21 were approvals read wrongly: one walk of two approved, a withdrawn walk counted as approved, a walk approved before its step began, a plan recorded with nothing to see still held on Walk, and step text that contradicted the brief the owner approved ("write no tests yet" against "regression test first").
These are gates Fleet did see, matched to the wrong step, because the agent's real order of work differed from the template's.

The other 8 are not about placement: preview restarts and a `start_server` refusal filed against the step notice that happened to be current.

## The proxies

The other 127 disputes come from proxies the rail used because it had no gate to read.
A push was a subagent's branch or a merge of main, not delivery.
An edit was the Draft step's own document, or a browser snapshot.
A turn ended because the Job was waiting on its merge.
Task order broke when groups ran side by side.

## What came before

Jobs before 15 Oct ran one Drone per step with restricted tools and Checks at every step (`job_steps.assigned_drone`, 44 rows from 1 to 12 Oct).
The owner dropped them on 15 Oct (`.claude/decisions/2026-10-15-sessions-absorb-jobs.md`): they never planned with him, Checks at every step bogged his machine down, and Drones with restricted tools were slow and could not finish.
He chose a soft rail in their place: "Fleet detects the step from what it observes and nudges when one is skipped, nothing blocks".
That soft rail is what this spike measures failing.
Spike 39 found a fresh session per step saves 2.0% to 4.4% of all spend, and that 138 of 256 step boundaries end a step in which the owner wrote to the Job.

## What follows from it

A mechanism that works has to give the step one owner and one record.
The record moves forward only, and only on a fact Fleet saw close, such as an approval, a merge or a green Check.
It never moves on a proxy, and it never moves backwards because a fact dropped out of a recomputation.
Spikes 41 and 42 test which mechanism does that.

## Queries

```sql
-- disputes by what moved the step
select case
 when notice like '%you are on its%' then 'step entered'
 when notice like '%edited files before%' or notice like '%edited a test before%' then 'edit'
 when notice like '%pushed while tasks%' then 'push'
 when notice like '%turn ended on its%' then 'turn end'
 when notice like 'Fleet set these tasks%' then 'task order' end k,
 count(*), count(distinct session_id)
from fleet_disputes
where notice like 'Fleet: this Job follows%' or notice like 'Fleet set these tasks%'
group by k;

-- step entries the agent said came after the merge
select count(*) from fleet_disputes
where notice like 'Fleet: this Job follows%' and notice like '%you are on its%'
 and (why like '%merged%' or why like '%landed%')
 and why not like '%not approved%' and why not like '%has not%';
```

Jobs moved backwards: a `session_rail.visits` row whose step index falls between two lines.
