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

## Docs and tickets a Job read

A page shows the docs and tickets that shaped a feature's work, as two lists of links beside the Jobs and pull requests.
A **doc** is anything a Job read from outside the code: a web page, an MCP server's answer, or a markdown or doc file in the repository, under `docs/` or outside the source folders.
A **ticket** is an issue or tracker item, as a GitHub issue, a beads id or a Linear or Jira item, and it has its own kind of link because the page shows it in a card of its own.

> **Rule.** A read becomes a link only when the same Job then writes a file the feature owns.
> Why: a Job reads a great deal that never shaped what it built, and a page that lists all of it says nothing.

The mod hears the call and its result and tells Fleet the call, what it named, and a title and a first sentence cut to about 300 characters together.
The Job reports nothing, and Fleet decides which calls were reads of a doc or a ticket and which were not: a server's call that changes something is not one, nor is a source file, `CLAUDE.md` or a server that stays on this Mac.
Fleet keeps each read the first time it hears it, per Job.
When the Job writes a file a feature owns, by the rules in [Which feature owns a file](#which-feature-owns-a-file), the reads it kept so far are linked to that feature, the newest twenty.
A read after the write is linked by the Job's next write, never by the one before it, and a read in one Job is never linked by another's write.
A link is one per Job, feature, kind and address, so reading a page again or writing again adds nothing, and the page shows an address once, under the newest Job that read it.

Fleet sees the order of the read and the write and nothing of whether the Job quoted or acted on what it read, so the order is the whole test.
A page that was fetched and read in the same Job as an unrelated edit is linked all the same.

A link carries the title and the first sentence of what was read, and where Fleet did not see the result it carries the address or the id alone.
A link whose address no longer answers keeps what it said and reads as `gone`.
The state is on the link; nothing checks yet.

A Job whose mod is older than 0.3.78 tells Fleet no reads, and a hosted Job's writes are linked from the mod's report of them, as the files it wrote are credited to a feature.

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

## Floor plan

A feature's page carries a floor plan: the project's code map, with the rooms the feature owns lit and the contracts it reaches drawn as doors.
Fleet reads it from the repository's files when the page is read.
It costs no model and does not read the code graph.

A room is a module.
Fleet walks each repository the project holds and takes the folder of each build manifest: `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, a Gradle `settings.gradle(.kts)`, `pom.xml`, `.csproj` and `Package.swift`.
A module is named as its manifest names it, and carries the manifest it came from and the modules of the same repository it depends on.
A manifest that only lists members is a root and is no room: a virtual Cargo workspace, an npm `workspaces` or pnpm root, a Maven aggregator.
A tree with no manifest has its top folders as rooms.
The walk skips hidden folders, `target`, `node_modules`, `dist`, `build`, `vendor` and the like, stops at eight folders deep and at 3,000 folders, and keeps at most 400 modules, shallowest first.

> **Rule.** A room is lit when the feature owns a file in it by the rules above, and no other way.
> Why: a second way to say what a feature touches would drift from the first, and a Job would learn to trust neither.

A room is lit when the feature's prefix is the module's folder or a parent of it, when a prefix or a filed file lies inside it, and when no feature owns it more closely.
A prefix `crates` lights each crate under it unless a longer prefix of another feature takes that crate.

A door is a dependency out of a lit module to another module of the same repository, as the manifest declares it.
Dependencies are the ones a build needs: Cargo `dependencies`, `build-dependencies` and target tables, npm `dependencies`, `peerDependencies` and `optionalDependencies`, and not the ones only tests or tools use.
A door is a contract where the module it leads to carries a wire or API contract.
A module does when its name or the last segment of its folder is one of `api`, `ipc`, `proto`, `contract`, `schema` or `openapi` (and their plurals), or when a `.proto` or OpenAPI or Swagger file lies inside it.
In Armada that is the `ipc` crate.

> **Rule.** A project declares no contracts, Fleet reads them off the names and files above.
> Why: a list a person keeps of the contracts is the second record that goes stale, and a name or a `.proto` file already says it.

A page read by `section` has no floor plan.
A repository that does not read, or a project with no module, has none either.

## Project overview

The Features screen opens a project on its overview: what it is, its code map, the features busiest now, and its glossary.
`get_project_overview` reads it from the repository's files and the library's rows, with no model.

What it is, is the first sentence of the first repository's README, with the file it came from.
The first repository is the first the project lists that Fleet serves.
Fleet takes the first paragraph of prose after the title, badges, code and lists, strips the markup, and ends it at the first full stop that is not an `e.g.`.
A repository with no README has none, and the screen draws nothing there.

The code map is the project's rooms, the modules the floor plan reads, and each room names the area that owns most of its files.
A file is counted for the feature that owns it by the rules above, the longest prefix first, and a pending feature owns none.
A room no feature owns a file in has no area.

> **Rule.** A room's area is counted from the files in it, and never kept as a second record.
> Why: a list of which area holds which room would drift from the prefixes the moment a feature moved.

The busiest features are the five with the most merges in the last 14 days, most first, each with its pull requests' numbers, newest first.
A feature with no merge in the window is not listed.
Both numbers are chosen, not measured.
The glossary is a list of `term` and `meaning`, and is empty until it is built.

## Since you looked

Beside the overview, the screen says what moved since he last looked.
Bridge keeps the time he last looked and sends it as `since` to `get_since_you_looked`.
The answer is by feature, each with a tag:

| Tag | Where |
|---|---|
| `needs_you` | The feature is pending and waits for his Confirm |
| `changed` | A pull request merged on it since, and it was in the catalog before |
| `new` | It entered the catalog since |

A pending feature is `needs_you` however old it is, and a feature both added and merged on since is `new`, since he has not seen it yet.
Needs you comes first, then Changed by newest merge, then New by newest added.
Each feature carries one line, `what`, made from the titles of the pull requests merged since, newest first and at most three.
A merge kept before titles were kept has none, and then the line is the feature's summary.
`pulls` are those pull requests' numbers.
`paragraph` is the one short paragraph the librarian's `since_you_looked` part wrote, and it is empty where none was written.
It is served only when `since` is at or before the run that wrote it, since he may have looked after that run, and the list under it is always read live.
The part and what it spends are under *Librarian*.

> **Rule.** Fleet builds the list from merges and the catalog's own dates, and a model only ever words it.
> Why: a list a model chose would drop a feature on a bad day, and he reads it to find what changed.

A project with nothing since answers an empty `features`, and a `since` that is no time is refused with `fleet.feature_since_unreadable`.

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
Only `since_you_looked` is built so far, and a run does nothing else but keep its record.

Three settings in `settings.json` say what it does, on the Features page, `runs` first.
`features.librarianRuns` is `on_press`, `after_merges` or `nightly`, and it ships as `on_press`.
`features.librarianParts` names the parts a run does, all nine as it ships.
`features.librarianModelShare` is how many model calls a night may spend, twenty as it ships, a number chosen and not measured.

`run_librarian` runs the enabled parts for one project, on a press of Catch up, and keeps one record of the run: when it ran, what started it, the newest merged pull request and the commit it read through, the parts that ran, the paragraph it wrote, the calls it made and the parts it left out for the share.
`list_features` carries the project's librarian state beside its areas: the last run, the newest merged pull request it read through, and the merged pull requests since.
Fleet reads those numbers from the merge links it already keeps and does not ask the forge.
Bridge draws "not read since #N" from it.

> **Rule.** Fleet starts no librarian run by itself while `features.librarianRuns` is `on_press`.
> Why: the owner wants it on demand until it has been tested more, and a run Fleet started on its own would be one he did not ask for.

**`since_you_looked` writes the paragraph at the top of Since you looked.**
It takes the features new, changed or waiting on him since the last run that did the part, each with its merged pull request titles, which is the page's own list and costs no model.
It puts them to the small model that sorts his messages in a Job (`models.dispute`), through the same call, and asks for at most three short sentences of facts, with no advice, no narration and no dashes.
Fleet keeps the answer with the run, cut to three sentences with any dash taken out.
A first run has no earlier run, so every feature reads as new, and the prompt names the first twenty and counts the rest.

> **Rule.** Where nothing moved since the last run, the part makes no call and writes no paragraph.
> Why: the model runs only where words must be written, and a paragraph about nothing would be words for the sake of words.

**The part spends from the night's share.**
A night is the machine's local midnight to midnight, and the share is counted in model calls for each project.
A call that fails still counts, since it was made.
Where the share is spent the part is left out, and the run's answer lists it in `skipped_for_share` so the librarian card can say "share spent".
The facts then wait: the part's next run reads from the last run that did it.

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
| `get_project_overview` | A project's README sentence, code map with areas, busiest features and glossary |
| `get_since_you_looked` | What is new, changed or waiting on him in a project since he last looked |
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
- The check that marks a doc or ticket link `gone`.
