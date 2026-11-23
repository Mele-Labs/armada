# Feature catalog

**What it is:** The library Fleet keeps of a project's features: where each one lives in the code, what Jobs have written about how it runs, and every Job, pull request, merge and design made for it.
A Job that touches a feature is handed its page before it reads code, and writes the parts only it knows before it ends.

---

**Kind:** Record, one library per project, kept on this Mac.

A Job is started on "Voting on takes".
Before it has searched the code, it holds a page of about a thousand tokens: the files to open first, the command that tests it, the bugs open on it, and which sections are stale.
When the Job has pushed, `all_done` waits for the Job to say how each feature its commits name tests, runs, is observed and what debt it carries.

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
| Decisions | Detected from what he settled in a Job, see *Decisions* |
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

## Decisions

A decision is something the owner settled in a Job.
It is his answer to a question, an `AskUserQuestion` card or an item of `waiting_for` that offers options, his pick on takes, his Approve or Ask for changes on a sketch, a draft, a page or a plan, or his Looks good on a Ready to try card.
Fleet keeps the question or the artifact's title, what he chose, his note if he gave one, the Job and the time, and `get_feature_page` serves them as `decisions`, newest first.
No model reads anything: each place Fleet already records one of these answers files it.

> **Rule.** A decision is filed under the features its artifact names, and where it names none, under the features its Job is linked to.
> Why: a sketch made for one feature says which feature his answer shapes, and an answer to a bare question is best placed where the Job's work landed.

The Job's features are read when the page is served, so a feature the Job is linked to after he answered shows the decision too.
An artifact's features are the ones its link says, so a feature renamed or merged keeps its decisions.
Hearing the same answer twice files it once, and an answer given in his place by the Operator or Autopilot is not filed.
A plan's Approve settles the sketches it names, and those are filed with the plan, not again as their own.

## Stale

> **Rule.** A written section is stale when a merge after it changed a file mapped to its feature, and it reads `stale since #<pr>`, the first such merge.
> Why: the section was true of the code it was written against, and nothing says it still is.

The merge of the Job's own pull request does not make that Job's sections stale, because the Job wrote them inside the work the merge carries.
A merge is kept with the files it changed, so the mapping is read again when a page is read.

## Health

> **Rule.** A mark always carries its reasons, and a source Fleet does not keep is not guessed.
> Why: a mark with no reason cannot be argued with, and a guess about a source reads as a fact about the feature.

Each feature in the list and on its page reads `good`, `watch` or `poor`, with a reason for each fact behind it: a kind, one short sentence and, where there is something to open, a target.
`good` has no reasons.
It is worked out on each read, from what Fleet already keeps, with no model and no call to the forge.

| Source | Reads as | Mark |
|---|---|---|
| A flaky test under a fix Job whose test map edges reach its files | `flaky test <name> reaches it` | `watch`, `poor` from three |
| A written section stale | `<section> section stale since #<pr>` | `watch` |
| The test map holds edges and none reaches its files | `no test reaches its files` | `watch` |
| An open link of kind `bug` | `open bug: <title>` | `watch`, `poor` from three |
| A Check, or a lint or type Check, whose last run in a live Job working in its files failed | `clippy failed on the last run of "<job>" in its files` | `poor` |

The three and the sources are chosen, not measured.
A source Fleet holds nothing for adds nothing: a test map never fed gives no `no test reaches its files`.
Left out, because Fleet keeps nothing per file for them: a pull request's forge checks and main's CI state.

## The page condensed

A condensed page is text for an agent, about 1,000 tokens, counted as characters over four.
It holds, in this order: name, area, what it is current as of and the stale marks; the Jobs on it now; a line for each open pull request of someone else on it (see [Who's on what](#whos-on-what)); the top five entry points; the test command; the newest three open bugs; how it runs in one line; designs as titles; debt in one line; and one line saying how to fetch a full section: `get_feature_page` with `feature` and `section`.
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

A Job may write a gist for a doc or ticket it read, at its end, as `gists` in its `write_feature_sections` call: `{target, gist}`, one line of what the doc says that bears on the feature, in its own words.
It applies to the read links of that Job under every feature its projects hold, is cut to 200 characters on one line, and wins over the first sentence: a first sentence heard later does not replace it, and the page serves it with `gist_by_job`.
A target the Job never read and linked is refused as `fleet.gist_not_read`, and a blank gist as `fleet.gist_blank`.
Where `all_done` is refused for sections owed, and in the `owed` answer of `write_feature_sections`, Fleet names the docs the Job read under those features, eight at most, and says a gist is welcome.

> **Rule.** A gist is never required, and no refusal of `all_done` rests on one.
> Why: a Job that read nothing worth summarising should not be held for a line it has nothing to say in, and a gist written to clear a refusal would be worse than the first sentence.

A Job whose mod is older than 0.3.78 tells Fleet no reads, and a hosted Job's writes are linked from the mod's report of them, as the files it wrote are credited to a feature.

## Holding a Job's end

> **Rule.** `all_done` is refused while a feature the Job's commits name in their `Feature:` line has a section the Job has not written since it started.
> Why: only the Job that did the work knows how the feature runs now, and a page nobody rewrites goes stale in silence.

The refusal is 409 `fleet.feature_sections_owed`.
It names each feature and the sections owed, one feature to a line.
A feature whose files the Job changed without naming it stays linked to the Job and is owed nothing, so a field added to a struct in thirteen files across seven features owes the one feature the commit names.
Fleet keeps what each commit names when the commit gate lets it through, and reads the Job's `commit` links as well.
A Job whose commits name no feature is never held, and neither is a Job with nothing pushed.
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

When Bridge asks for them, each room holds its files, with each file's size in bytes as git keeps it at the checkout's `HEAD`, so Fleet opens no file to size it.
An agent's read of the page has rooms without files, so it stays small and costs no git read.
A file the feature owns is lit by the same rule as a room, and a file a live Job has written is marked as changing.
Lock files, images, fonts and test snapshots are left out.
A plan carries about 1,500 files: the feature's files and the ones being changed stay single, then the largest, and the rest are counted together by folder with their sizes added up.

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
The glossary is a list of `term` and `meaning`, read from the project's glossary (*Glossary*).

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

## Who's on what

In a large team nobody knows who is working on what, so Fleet reads the open pull requests the Reviews tool already pulls, teammates' included, and sets them beside the owner's own Jobs.
Whenever the Reviews lists are read, Fleet keeps each open pull request's author, title, draft mark, head commit and the paths it changes, the first 100 of them, with a mark where there are more.
A pull request whose head is the one already kept is not read again: that is one forge read for each pull request and each new head, at the moment the lists are read anyway, and no model.
A pull request the lists no longer name is dropped, except from a read where one of the lists did not answer.
The files are mapped to features as any path is, by the prefixes a feature owns and the files Jobs filed under it.

`get_whos_on_what` answers by feature, each with a cell for every person on it:

| Part | Holds |
|---|---|
| `people` | The owner first, where Fleet knows his login, then everyone with a cell by login |
| `features` | Only features with a pull request or a Job, the most people first and then by title |
| a cell | A `login`, the open `pulls` on the feature and, in the owner's cell alone, the ids of his Jobs on it now |

A feature with two or more cells is an overlap.
Fleet does not flag it: the screen counts the cells.
The owner is whoever Armada is signed in to the forge as, else the forge command line's account.
His Jobs are the ones the feature's page lists as on it now.

> **Rule.** A Job is told of a teammate's open pull request on a feature in the page it is handed, and the owner is not told by a notice.
> Why: the Job can change what it does with it, and the owner has the screen to look at when he wants to.

A page handed to a Job names each open pull request by someone else that changes a file of the feature, newest first and at most five: `Open elsewhere: #412 "Start card remembers the last workflow" by priya touches this feature.`
Where Fleet does not know the owner's login nobody can be called a teammate, so the page says nothing of the kind.
The pull requests are those the Reviews lists name, so the screen shows none until Reviews has been read once.

## Ranking the issues he might pick up

The project's Dashboard lists the open issues nobody is assigned to, that no Job holds and that no pull request already closes, as the ones he might pick up.
`list_ranked_issues` maps each to the features it touches, three ways, and uses no model and asks the forge for nothing: it reads the issue lists Fleet holds, the open pull requests the Reviews lists left with their files, and the Jobs the library recorded.

| Way | An issue matches a feature when |
|---|---|
| Files of a pull request | An open pull request of the same repository names the issue in its title, and one of its files is the feature's by the rules in [Which feature owns a file](#which-feature-owns-a-file) |
| Label | A label equals the feature's slug, or its area's, with case and separators ignored |
| Title | The feature's slug or title stands in the issue's title as whole words, with case and separators ignored |

> **Rule.** A row is ranked by how many of the owner's Jobs filed a file under the features it matched in the last 30 days, each Job counted once, then the most recently updated first.
> Why: the work he has been doing lately is the work he is most likely to pick up next, and a Job on two matched features is still one Job.

An issue that matched no feature comes after every issue that did, and the row names each feature it matched and by which way, so the screen can say why it is there.
A pending feature is never matched.
The same answer lists the open pull requests that are not his, mapped by their own files, as the ones he might be interested in.
They are ranked as the issues are, the ones that matched a feature first, then the most Jobs, then the highest number.
It is empty where Armada does not know who he is on the forge, since then nobody can be called somebody else.

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
Only `since_you_looked`, `gists`, the free part of `drift`, `retire` and `links` are built so far, and a run does nothing else but keep its record.

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

**`gists` writes a one-line gist for a linked doc that has none.**
It reads the docs linked by a Job's read that are files of the project's repository at `HEAD`, where no link of the doc has a gist: no Job wrote one, and the first sentence was empty because the result was not visible.
It takes the oldest links first, thirty docs a run at most, and reads the first 4000 bytes of each from disk; an address or a doc it cannot read is left out.
It puts up to ten docs in one call to the small model that sorts his messages, each as a numbered item with its path and opening text, and asks for one line a number: a plain fact of at most twenty words about what the doc covers, with no advice and no dashes.
Fleet reads the reply by number, cuts each line to 200 characters on one line, and keeps it on the doc's read links with `gist_by` `librarian`, so the page serves it in `gist`.
A number the reply lacks leaves that doc as it was, and the next run asks about it again.
Each call is spent from the night's share, and where the share runs out the rest wait and the run lists `gists` in `skipped_for_share`.
The step is `write_gists`, after every other part.

> **Rule.** The model writes a gist only where none exists.
> Why: a Job's own words or the doc's first sentence already say it for free, so a call there would spend the share on a line the page has.

> **Rule.** A Job's gist replaces the librarian's, and a first sentence heard later does not.
> Why: a Job's words are about what it changed, and a first sentence is the weakest account of a doc.

**`drift` flags a document that names what the code no longer has.**
Its free part reads the documents linked to the project's features that are files of its repository, up to two hundred a run, and costs no model.
From each it takes the names written in backticks outside fenced blocks, up to forty: a `snake_case` or `CamelCase` word, a `name()`, `Type::name` or `module::Type`, and an endpoint such as `GET /features/page`.
It asks git whether each name was in the code, outside markdown, when the document was last changed, and whether it is now, with one search at each commit for the whole document.
Fleet keeps the names that were there and are gone, and the page's `docs` carry them as `drifted`; a document whose check finds none loses its flag.

> **Rule.** A name has drifted only where the code had it when the document was last changed and does not have it now.
> Why: a name the document invented, or one it never had right, was never the code's to lose, and flagging it would teach him to ignore the flag.

The model's part of `drift` is not built.

**`retire` and `links` share one free step that reads features and linked docs against the code.**
It costs no model and asks git once for the files at `HEAD`, and once for each feature for the newest commit that touched its paths.
A feature whose paths match no file at `HEAD` and never did is **planned**: the code is not written yet, and the mark clears at its first file.
A feature whose paths matched files once and match none now carries a **Retire proposal**, with the last commit that touched them as evidence: "Its paths match no file since abc1234, 14 May".
A feature that has files carries one too where nothing has changed them for 180 days, and no Job has written a file of it or been linked to it in that time: "No change to its files since abc1234, 2 May; no Job on it since 3 Apr".
The 180 days are chosen, not measured.
A linked doc that is a file of the repository and is not at `HEAD`, nor on disk, is marked `gone` and keeps its gist; an address or a ticket never is.
Pending features and features with no paths are not read, and where git cannot be read the step marks nothing and leaves what it marked.
`list_features` carries `retire` and `planned` on a feature, and `get_feature_page` carries `state` `gone` on a doc.
`answer_retire` is his answer: Retire takes the feature out of the catalog and leaves its links, Keep holds the proposal.

> **Rule.** Retire is only proposed, never done by Fleet.
> Why: retiring hides a feature, and only the owner knows it is truly gone.

> **Rule.** A Keep holds until the evidence changes.
> Why: a proposal he answered should not return on every Catch up.

**The part spends from the night's share.**
A night is the machine's local midnight to midnight, and the share is counted in model calls for each project.
A call that fails still counts, since it was made.
Where the share is spent the part is left out, and the run's answer lists it in `skipped_for_share` so the librarian card can say "share spent".
The facts then wait: the part's next run reads from the last run that did it.

`after_merges` and `nightly` start a run from Fleet's tick, and only where the owner has set them.
A word Fleet cannot read means `on_press`.

**The Catch up card shows a run's steps before he presses, and each as it moves.**
`list_features` carries the steps a run would take now, each free or model with one line on what it reads or spends: `read_merges`, `check_drift` where `drift` is on, `check_upkeep` where `retire` or `links` is on, a step for each model part that is on and built, `write_gists` last among them where `gists` is on, then `read_through`.
A run tells each step on the stream as `librarian.step`, running and then done, skipped or failed, with a line of what it did.
While a run goes, `list_features` also carries it with each step it has reached so far, so a Bridge that opens mid-run redraws the steps and Stop.
`stop_librarian` stops a run between steps: the steps after the one running are skipped, the run keeps its record marked stopped, and the catalog stays read through where the run before left it.
One run of a project goes at a time, and a press while one runs is refused.

## Glossary

Each project has a glossary of its terms: the word, what it means, and the words that mean the same and should not be used.
The librarian keeps it and Jobs fill most of it.
A Job that introduces a word gives it as `glossary`, `{term, meaning, not}`, in the `write_feature_sections` call it makes before it ends, and no Job is ever asked for one.
A term is unique per project without regard to case, so a later write of `take` replaces `Take`.
`get_glossary` reads a project's terms, and each carries the Job that wrote it.
Nothing here reads a model: the librarian's `glossary` part, not built yet, is where a model fills the gaps Jobs leave.

Armada's own glossary is seeded from the lexicon and the retired terms in [Voice and copy](../contracts/design-system/voice-copy.md), with the same rule as the catalog: kept once per project, and only where a repository of the project holds the seed's marker paths.
Only a word that means one thing in this repository is listed as one not to use, so `server`, `session`, `run`, `step` and `agent` are left out.
A term a Job wrote before the seed was kept stays as the Job wrote it.

A commit a Job is about to make is read for the `not` words of its project's glossary, in the commit's own words and not in the paths given to `git add`.
Each word found is filed as a `glossary_flag` link on every feature the Job is on, naming the word as its target, the term to use as its title and the Job.
The flag is kept once for a word and a Job, and it shows among the feature's links.

> **Rule.** A word used against the glossary is a flag on the feature, never a refusal.
> Why: the Job may be quoting the word, or the glossary may be the one that is wrong, and a refused commit would teach Jobs to avoid the word rather than say what they mean.

Sections, pull request descriptions and other text Jobs write are not read yet.

## How it is judged

Jobs on Armada should search the code less before their first edit.
Fleet counts, for each Job, the Grep and Glob calls and the shell `grep`, `rg`, `find` and `git grep` commands before its first Edit or Write, in its own turns and its subagents', from the transcripts it already reads.
`get_first_edit_searches` answers the figure a week at a time, so the week before the library and the week after can be compared.
A Job whose turns were read before searches were counted is in no week until its transcript is read again.

## Operations

| Operation | Does |
|---|---|
| `list_features` | A project's areas and features, with who is on each now, the oldest stale merge and each one's health |
| `get_feature_page` | One feature whole, with the decisions filed under it, or with `section` that one section alone |
| `get_feature_brief` | One feature condensed |
| `get_project_overview` | A project's README sentence, code map with areas, busiest features and glossary |
| `get_since_you_looked` | What is new, changed or waiting on him in a project since he last looked |
| `get_whos_on_what` | Who has an open pull request or a running Job on which feature of a project |
| `list_ranked_issues` | A project's open, unassigned issues and other people's open pull requests, each with the features it matched, ranked by his recent Jobs on them |
| `get_features_of_path` | The features a file belongs to |
| `check_commit_feature` | The mod's hook asks whether a commit names its feature, and Fleet refuses it with the features its files belong to |
| `hand_over_features` | The mod's hook asks which pages a Job is owed for a read or a brief, each once |
| `write_feature_sections` | The calling Job writes sections of a feature, the glossary terms it introduced and its gists of the docs it read |
| `get_glossary` | A project's glossary |
| `upsert_features` | The Operator, or he from the Features screen, keeps areas and features |
| `seed_features` | The Operator keeps Armada's own catalog from the seed Fleet carries |
| `run_librarian` | The owner's Catch up, or the Operator, runs a project's librarian now |
| `stop_librarian` | The owner's Stop, or the Operator, stops a project's running librarian run before its next step |
| `answer_retire` | The owner's Retire or Keep for a feature the librarian proposed to retire |
| `librarian.step` | One step of a librarian run started or ended, with a line of what it did |
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
