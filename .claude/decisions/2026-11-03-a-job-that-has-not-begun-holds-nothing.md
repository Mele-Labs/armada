# Does a Job that has never run hold the Job that started it?

**Decided 2026-11-03.** On 30 Oct a live instance began holding its Job open,
and a Job a Job started reads Running from its first row, whether or not it has
run. So a finished Job stayed open for hours, holding its slot, for a child on
its start card or waiting on another Job, with nothing its agent could do about
it. On this Mac e951cc2f held 9ce4266e, and 0d98464a held 4e9dba06. Neither
child had taken a turn. It also made one wait impossible: a child waiting on its
parent with `ends` needed the parent to end, and the parent's agent could not
finish.

**Chosen: a Job that has not begun holds nothing.** Its parent can end and give
back its slot. A Job that is running still holds its parent.

| Asked | He chose | The cost he took |
|---|---|---|
| Which Jobs stop holding | Any with no run yet: on its start card, or waiting on another, approved or not | A parent can be finished and gone before the work it started begins |
| What End does | Still lists it, and ending the parent neither ends it nor stops it starting when its condition is met | A child may start with no parent there to hear of it |
| What happens once it runs | It holds a parent still live again | A parent that had finished up may be refused a later `all_done` |
| What a parent hears | One told to wait is told in a sentence that it may finish | One more notice for a parent refused before this |

**What it keeps.** The 30 Oct rule stands for everything else: a pull request,
ticket, deploy or Job that is really under way refuses `all_done` and
`finish_up`. A Job already ended is unaffected.

**"Has not begun" is the hosting's `ran`**, kept when the Job's process first
spawns. `last_turn_at` was not used, since it is stamped when a turn ends, and a
child on its first turn is running.

## What would make this wrong

**That a parent's work depends on seeing the child through.** If Jobs start
children and then finish before those children have had any of their context
checked, the work starts with a handoff nobody can question. The thread of a
Job that ended takes no word, so a child with a question for its parent would
have to ask the person.

Code: `crates/fleet/src/lifecycles/ending.rs`. Rule:
`docs/concepts/session/the-rail.md`, *A live instance holds its Job open*.
