# Feature catalog

**What it is:** The library Fleet keeps of a project's features: where each one lives in the code, what Jobs have written about how it runs, and every Job, pull request, merge and design made for it.
A Job that touches a feature is handed its page before it reads code, and writes the parts only it knows before it ends.

---

**Kind:** Record, one library per project, kept on this Mac.

A Job is started on "Voting on takes".
Before it has searched the code, it holds a page of about a thousand tokens: the files to open first, the command that tests it, the bugs open on it, and which sections are stale.
When the Job's pull request has merged, Fleet knows which features its files belong to, and `all_done` waits for the Job to say how each of them tests, runs, is observed and what debt it carries.

## What a feature holds

| Part | Where it comes from |
|---|---|
| Area, slug, title, one line | Seeded by the Operator, then confirmed |
| Path prefixes it owns | Seeded by the Operator |
| Entry points, as `path` or `path:line` | Seeded by the Operator |
| Jobs on it, now and before | Detected from the files each Job wrote |
| Pull requests and merges | Detected from the pull requests Fleet attached and the forge's merges |
| Sketches, pages, walks and drafts | The act that made them names the feature: `feature` on `show_sketch`, `show_window` and `propose_draft`, `armada-feature` in a page's head |
| Commits | Detected from `Feature:` trailer lines in git |
| How to test it, how it runs, how to observe it, tech debt | Written by the Job that did the work |

A Job is on a feature now only while it is live and holds a worktree slot, the same fact the Jobs list shows as its `slot`.
A live Job that holds none has stopped without ending, and drops off.
A Job waiting on the person while it still holds its slot stays on it.
An area holds features, and two levels are all there are.
A feature belongs to one area.

> **Rule.** Fleet detects what it can see and never asks a Job to report it.
> Why: a link a Job reports is a link a Job can forget.

## Which feature owns a file

Fleet does not read the code graph, so a file is mapped by three things it can see.

| Source | Meaning |
|---|---|
| A path prefix of the feature | The file is the prefix or under it, on whole path segments |
| A commit's `Feature: <slug>` trailer | The files of that commit belong to the features it names |
| A write while the Job's commits named one feature | The files the Job wrote that no prefix owns belong to that feature |

> **Rule.** The longest matching prefix wins, and several features may own one file.
> Features that tie on the longest prefix all own it, and so does a feature a Job filed the file under.
> Why: a file that serves two features is in both pages, and a wide prefix must not take a file a narrow one claims.

A `Feature:` trailer is one or more lines in a commit's last paragraph, each naming one slug or several separated by commas.
A trailer naming a slug the project does not hold files under a pending feature, in a project that has a library; see [A feature a Job names that does not exist](#a-feature-a-job-names-that-does-not-exist).

## Which feature a commit serves

> **Rule.** A commit names the feature it serves in a `Feature:` line, and the mod's hook refuses a `git commit` that gives its message in the line and has none.
> Why: the Job says which feature each change serves, and a commit that does not say is filed under whatever the code owns.

The refusal names the features the files about to be committed belong to, the most files first, each as the line to copy.
Where no feature owns them it says so, and asks for the feature the commit serves or one that does not exist yet.

> **Rule.** Only a Job started after its project's catalog was seeded is refused.
> Why: approving a plan is not consent to change a Job already running.

The catalog was seeded when its first feature entered it, the earliest time any feature of the project was added.
A project with no feature, and a Job that started before that time, are never refused.
A commit that takes its message from a file, another commit or an editor is not read, and a commit with no files is not refused.
A Fleet that is out or slow lets the commit run.

## Which feature an act serves

An act that makes a sketch, a walk or a draft takes an optional `feature`, one slug or a list, and a page names its features in its head.
Fleet links the artifact to each feature the project holds, as a link of kind `sketch`, `walk`, `draft` or `page`.

> **Rule.** A slug the project holds no feature of is kept, not refused.
> Why: the Job may be making the first thing for a feature the Operator has not seeded yet, and the Operator confirms it.

Fleet makes such a name a pending feature of the first project of the Job that holds features, and links the artifact to it, as it does a trailer's unknown slug.

## Stale

> **Rule.** A written section is stale when a merge after it changed a file mapped to its feature, and it reads `stale since #<pr>`, the first such merge.
> Why: the section was true of the code it was written against, and nothing says it still is.

The merge of the Job's own pull request does not make that Job's sections stale, because the Job wrote them inside the work the merge carries.
A merge is kept with the files it changed, so the mapping is read again when a page is read.

## The page condensed

A condensed page is text for an agent, about 1,000 tokens, counted as characters over four.
It holds, in this order: name, area, what it is current as of and the stale marks; the Jobs on it now; the top five entry points; the test command; the newest three open bugs; how it runs in one line; designs as titles; debt in one line; and one line saying how to fetch a full section: `get_feature_page` with `feature` and `section`.
When it is over the cap the last of these go first, and the head and that line always stay.

## Handing a Job its pages

The mod's hooks hand a Job the page of a feature at three moments, and Fleet decides each time whether that page was handed before.

| Moment | Which pages |
|---|---|
| A prompt names features | Each feature it names, by a `Feature: <slug>` line or by its slug between backticks |
| A subagent starts | Each feature its prompt names, in the same two ways, added to the prompt |
| The Job reads or searches a file | Each feature that owns the file or the folder, at most three a call and the rest at the next read |

> **Rule.** A feature's page is handed to a Job once.
> Why: the page costs about a thousand tokens and says the same thing the second time.

The page rides on the result of the call that reached it, so the agent reads it before it digs.
A read or search that was turned away hands nothing, and neither does a search of the whole repository or a file outside the Job's checkout.
Fleet keeps the record per Job, holder and feature, so it holds across turns and a restart.
A subagent has a context of its own, so it is handed a feature's page whether or not its Job was, and then once itself: its own reads and searches are mapped the same way.
A Fleet that is out or slow hands nothing and the read goes on.

## Holding a Job's end

> **Rule.** `all_done` is refused while the Job's merged or pushed work changed files of a feature whose four sections it has not written since it started.
> Why: only the Job that did the work knows how the feature runs now, and a page nobody rewrites goes stale in silence.

The refusal is 409 `fleet.feature_sections_owed`.
It names each feature and the sections owed, one feature to a line.
A Job whose work changed no feature's files is never held, and neither is a Job with nothing pushed.
A Job that started before a feature entered the catalog is never held for it, because approving a plan is not consent to change a Job already running.
The person's End job is never held.
A Job rewrites a section that is still true by writing it again as it stands.

## A feature a Job names that does not exist

A Job names features in a `Feature:` trailer and in the `feature` of a sketch, page, walk or draft.
A slug the library does not hold is filed under a **pending** feature, made at once, so nothing the Job did is lost.
A pending feature is listed under the area `pending`, with no prefix and no line.

> **Rule.** A pending feature is made only in a library that has a feature already.
> Why: the earliest `added_at` across a project is when it was seeded, and a pending feature in an unseeded project would move it.

Fleet tells the Operator once, with the Jobs that named it, the files filed under it and the features whose prefixes cover those files.
The Operator brings him one card, with three options.

| Option | What the Operator does |
|---|---|
| Confirm | `upsert_features` with state `confirmed`, an area, a title and a line. The feature keeps what was filed under it |
| Merge into an existing feature | `change_feature` with `into`. Its prefixes, entry points, links, files, sections and asks move to the target, which keeps its id, its state and its `added_at` |
| Rename | `change_feature` with `slug` and `title` |

With the Operator off, he does each of the three himself from the Features screen, and Bridge makes the same calls the Operator would.
Fleet accepts `upsert_features` and `change_feature` from the Operator's own session and from Bridge, and from no other session: a Job's agent is refused.

> **Rule.** A merged or renamed slug stays as an alias of the feature it became.
> Why: a trailer already in git names the old slug, and Fleet reads it again each time, so without the alias it would make the pending feature again.

The Operator may also merge two seeded features that are one, and tells him on a card that carries what was absorbed, with an Undo that keeps it again through `upsert_features`.
That keeps the feature and its prefixes and entry points, and takes the alias away, but the links, files and asks that moved stay with the target.

## Seeding

Armada's own catalog is kept once from a data file Fleet carries, `crates/fleet/src/feature_library/seed/armada.toml`: the eighteen areas and the features the owner approved on 16 Nov 2026, each with its path prefixes and entry points.
Fleet keeps the seed itself, on its own tick, when a project holds Armada's repository and no feature, whether or not the Operator is on, and logs what it left out.
A project the seed does not fit is checked once in a Fleet run.
The Operator's `seed_features` is the same act on his word, and is refused once the project holds a feature.

> **Rule.** A seed is kept once per project, and only where a repository of the project holds the seed's marker paths.
> Why: a second seed would rewrite what Jobs and the Operator have since confirmed, merged and renamed.

Fleet leaves out a prefix or entry point none of the project's repositories holds, and the call names each.
A feature's `added_at` is the moment it was kept and is never rewritten, so the earliest across a project is when its library was seeded.

## An area that grows

An area that holds more than eight features is told to the Operator, who reads it with `list_features` and raises one card that proposes a split: the new areas and the features that go to each, with keeping the area as an option.
Pending features are not counted.
Nothing changes until he answers.
After he picks, the Operator keeps each new area with `upsert_features` and moves each feature with `change_feature` and `area`, which must be an area the project keeps.
With the Operator off, he splits the area from the Features screen, which makes the same two calls.
A new area is kept by `upsert_features` with `features` empty.
A move changes the feature's area and nothing else: its slug, prefixes, asks and `added_at` stay.

> **Rule.** An area is told again only when it has grown by another eight.
> Why: a card he answered by keeping the area would otherwise come back on every tick.

The seed's largest area holds eight, so seeding raises none.

## Librarian

The librarian is the Fleet work that keeps a project's catalog current.
It is a set of parts: `floor_plans`, `links`, `decisions`, `health`, `drift`, `retire`, `glossary`, `since_you_looked` and `gists`.
`since_you_looked`, `gists` and the model's part of `drift` are written by a model, so a night may spend only so many calls on them.
No part is built yet, so a run today keeps its record and does nothing else.

Three settings in `settings.json` say what it does, on the Features page, `runs` first.
`features.librarianRuns` is `on_press`, `after_merges` or `nightly`, and it ships as `on_press`.
`features.librarianParts` names the parts a run does, all nine as it ships.
`features.librarianModelShare` is how many model calls a night may spend, twenty as it ships, a number chosen and not measured.

`run_librarian` runs the enabled parts for one project, on a press of Catch up, and keeps one record of the run: when it ran, what started it, the newest merged pull request and the commit it read through, and the parts that ran.
`list_features` carries the project's librarian state beside its areas: the last run, the newest merged pull request it read through, and the merged pull requests since.
Fleet reads those numbers from the merge links it already keeps and does not ask the forge.
Bridge draws "not read since #N" from it.

> **Rule.** Fleet starts no librarian run by itself while `features.librarianRuns` is `on_press`.
> Why: the owner wants it on demand until it has been tested more, and a run Fleet started on its own would be one he did not ask for.

`after_merges` and `nightly` start a run from Fleet's tick, and only where the owner has set them.
A word Fleet cannot read means `on_press`.

## How it is judged

Jobs on Armada should search the code less before their first edit.
Fleet counts, for each Job, the Grep and Glob calls and the shell `grep`, `rg`, `find` and `git grep` commands before its first Edit or Write, in its own turns and its subagents', from the transcripts it already reads.
`get_first_edit_searches` answers the figure a week at a time, so the week before the library and the week after can be compared.
A Job whose turns were read before searches were counted is in no week until its transcript is read again.

## Operations

| Operation | Does |
|---|---|
| `list_features` | A project's areas and features, with who is on each now and the oldest stale merge |
| `get_feature_page` | One feature whole, or with `section` that one section alone |
| `get_feature_brief` | One feature condensed |
| `get_features_of_path` | The features a file belongs to |
| `check_commit_feature` | The mod's hook asks whether a commit names its feature, and Fleet refuses it with the features its files belong to |
| `hand_over_features` | The mod's hook asks which pages a Job is owed for a read or a brief, each once |
| `write_feature_sections` | The calling Job writes sections of a feature |
| `upsert_features` | The Operator, or he from the Features screen, keeps areas and features |
| `seed_features` | The Operator keeps Armada's own catalog from the seed Fleet carries |
| `run_librarian` | The owner's Catch up, or the Operator, runs a project's librarian now |
| `change_feature` | The Operator, or he from the Features screen, merges a feature into another, renames it, or moves it to another area |
| `file_ask` | The Operator files an ask said outside a Job under features, see [Job story](job-story.md) |
| `get_first_edit_searches` | The weekly figure above |

## Where it lives

> **Rule.** The library is on this Mac and is not shared.
> Why: sharing it with teammates through the project's store has not been designed.

Every id in it is minted whole and no row depends on a position, so rows can move to a shared store unchanged.
What the library would hold in the clear there is in [Central store](../contracts/central-store.md#a-feature-library-stays-on-this-mac).

## Not built

- Bugs and open issues on a feature: the page reads links of kind `bug`, and nothing writes them yet.
- Asks said in a Job sorted onto features.
