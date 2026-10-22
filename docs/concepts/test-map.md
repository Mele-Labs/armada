# Test map

**What it is:** The map Fleet keeps of which tests reach each file in a repository, fed from whatever source answers best for that file and read the moment an agent edits one.
Nobody asks for it, and nobody maintains it by hand.

---

**Kind:** Record, one per repository.

An agent on a Rails app edits a controller.
Before it has typed a test name, Fleet knows the request spec, the two system specs and the Cypress run that go through that file, and those are what run.
A file the map has never seen gets the Check as the [Manifest](manifest.md) declares it, which is what runs today.

## What it answers

| Question | Answer |
|---|---|
| Which tests reach this file | Each test, with its Check and where it is |
| Which files does this test reach | Each file, with the source that saw it |
| Which changed files does no test reach | The files, so an agent runs wider or writes one |
| How sure is each edge | Its source and the commit it was read at |

> **Rule.** Every edge carries its source and the commit it was read at.
> Why: an answer with no provenance cannot be ranked, refreshed or doubted.

## Where an edge comes from

Fleet has more than one way to learn that a test reaches a file, and uses whichever one it can afford for that file.

| Source | How it reads | Exact | Costs | Blind to |
|---|---|---|---|---|
| A run with coverage on | The runtime's own collector, one lap per test | Yes | A test run | What did not run that time |
| A static graph | Imports and calls, as vitest and the code graph read them | Partly | Nothing | Dynamic dispatch, an HTTP hop |
| A model's reading | A model reads the test and the code and names what it reaches | No | A model call | Nothing by kind, and it says it is a guess |
| The Manifest's declaration | `when` and `narrow` on the Check | No | Nothing | Everything finer than a directory |

> **Rule.** A measured edge outranks a static one, and a static one outranks a model's reading. The Manifest's declaration is the floor.
> Why: each source is sharper than the one below it, and the floor is what runs today, so the map can only narrow and never widen what is checked.

> **Rule.** A model reads only a file no collector can measure, and its reading is marked as one until a run confirms it.
> Why: it is the one source that works for any language and any kind of test, and the one that can be wrong without knowing, so a file a run can reach waits for the run.

> **Rule.** A source is a plug in Fleet, and a repository may add one.
> Why: Ruby, a browser suite and a Rust workspace each have their own collector, and Fleet owns the map, not the collectors.

## How it stays current

> **Rule.** The map is fed by work Fleet already does. Nothing a person does is for the map.
> Why: a map somebody has to run is a map nobody runs.

| Moment | What Fleet does |
|---|---|
| A Check runs for a Job | Records a lap for each test that ran, where the runtime's collector costs no rebuild |
| A pull request merges | The run on main refreshes the laps for what it ran |
| A file changes on a branch | Marks its edges stale, and reads them again by the cheapest source that can |
| A new file appears | Asks the static graph, and a model only where no collector can measure it |
| The machine is asleep | Re-measures stale edges in a free bay, one test at a time, and stops when sleep mode goes off |

> **Rule.** A stale edge is still an answer, said as stale.
> Why: a file edited an hour ago is reached by nearly the same tests it was, and no answer at all sends an agent to the whole suite.

> **Rule.** Recording never slows a run on a shared machine past what the Check costs on its own, and a collector that needs a rebuild records only on the forge's runner or in a sleeping bay.
> Why: one machine carries every Check, agent and Fleet.

## What reads it

| Reader | Uses it for |
|---|---|
| `armada check test --changed` | The tests to run for the paths on stdin, with the Manifest's narrowing as the floor |
| Fleet, before a Job's Checks | Which Checks a change reaches, where the map is sharper than `when` |
| Bridge, on a changed file's row | The tests that reach it, and the files no test reaches |

> **Rule.** A run the map narrowed gates nothing. The merge line runs what the Manifest says.
> Why: `manifest.md`, *How much of the tree a Check reads*, and the map is a sharper reading of the same question.

> **Rule.** The map never writes to the repository.
> Why: it is Fleet's record of the code, kept in Fleet's store, and a repository that has never heard of Armada reads the same.
