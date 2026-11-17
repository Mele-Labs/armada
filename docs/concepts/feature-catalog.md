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
| Sketches, pages, walks and drafts | The act that made them names the feature |
| Commits | Detected from `Feature:` trailer lines in git |
| How to test it, how it runs, how to observe it, tech debt | Written by the Job that did the work |

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

## Stale

> **Rule.** A written section is stale when a merge after it changed a file mapped to its feature, and it reads `stale since #<pr>`, the first such merge.
> Why: the section was true of the code it was written against, and nothing says it still is.

The merge of the Job's own pull request does not make that Job's sections stale, because the Job wrote them inside the work the merge carries.
A merge is kept with the files it changed, so the mapping is read again when a page is read.

## The page condensed

A condensed page is text for an agent, about 1,000 tokens, counted as characters over four.
It holds, in this order: name, area, what it is current as of and the stale marks; the Jobs on it now; the top five entry points; the test command; the newest three open bugs; how it runs in one line; designs as titles; debt in one line; and one line saying how to fetch a full section.
When it is over the cap the last of these go first, and the head always stays.

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

> **Rule.** A merged or renamed slug stays as an alias of the feature it became.
> Why: a trailer already in git names the old slug, and Fleet reads it again each time, so without the alias it would make the pending feature again.

The Operator may also merge two seeded features that are one, and tells him on a card that carries what was absorbed, with an Undo that keeps it again through `upsert_features`.
That keeps the feature and its prefixes and entry points, and takes the alias away, but the links, files and asks that moved stay with the target.

## Seeding

Armada's own catalog is kept once from a data file Fleet carries, `crates/fleet/src/feature_library/seed/armada.toml`: the eighteen areas and the features the owner approved on 16 Nov 2026, each with its path prefixes and entry points.
Fleet tells the Operator once when a project holds Armada's repository and no feature, and the Operator calls `seed_features`.

> **Rule.** A seed is kept once per project, and only where a repository of the project holds the seed's marker paths.
> Why: a second seed would rewrite what Jobs and the Operator have since confirmed, merged and renamed.

Fleet leaves out a prefix or entry point none of the project's repositories holds, and the call names each.
A feature's `added_at` is the moment it was kept and is never rewritten, so the earliest across a project is when its library was seeded.

## How it is judged

Jobs on Armada should search the code less before their first edit.
Fleet counts, for each Job, the Grep and Glob calls and the shell `grep`, `rg`, `find` and `git grep` commands before its first Edit or Write, in its own turns and its subagents', from the transcripts it already reads.
`get_first_edit_searches` answers the figure a week at a time, so the week before the library and the week after can be compared.
A Job whose turns were read before searches were counted is in no week until its transcript is read again.

## Operations

| Operation | Does |
|---|---|
| `list_features` | A project's areas and features, with who is on each now and the oldest stale merge |
| `get_feature_page` | One feature whole |
| `get_feature_brief` | One feature condensed |
| `get_features_of_path` | The features a file belongs to |
| `write_feature_sections` | The calling Job writes sections of a feature |
| `upsert_features` | The Operator keeps areas and features |
| `seed_features` | The Operator keeps Armada's own catalog from the seed Fleet carries |
| `change_feature` | The Operator merges a feature into another or renames it |
| `file_ask` | The Operator files an ask said outside a Job under features, see [Job story](job-story.md) |
| `get_first_edit_searches` | The weekly figure above |

## Where it lives

> **Rule.** The library is on this Mac and is not shared.
> Why: sharing it with teammates through the project's store has not been designed.

Every id in it is minted whole and no row depends on a position, so rows can move to a shared store unchanged.
What the library would hold in the clear there is in [Central store](../contracts/central-store.md#a-feature-library-stays-on-this-mac).

## Not built

- A commit hook that refuses a commit without a `Feature:` trailer.
- Bugs and open issues on a feature: the page reads links of kind `bug`, and nothing writes them yet.
- Asks said in a Job sorted onto features.
