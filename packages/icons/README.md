# packages/icons

**Authored here.** `icons/` is the authority on every lucide-react glyph
Armada assigns a meaning to — what its silhouette depicts, what UI family it
belongs to, and what it may never be reused for. Notion's "Armada Icons"
database is the record of the decision and the reasoning that produced it,
not the authority on the current assignment — a row there that disagrees
with this registry is stale, not right.

| Path | What it is |
|---|---|
| `icons/` | The registry. One file a glyph, `<name>.toml`, holding its `[icons.<name>]` table and the `[[icons.<name>.usage]]` entries for each context it is used in. The name is the kebab-case lucide-react name, and the gate refuses a file whose table is not named for it. `icons/_header.toml` says what the fields are |
| `conventions/` | `[conventions.<name>]`, one file a rule, for the rules that name several glyphs at once rather than assigning one its own meaning |

A glyph is a file so two branches that each add one touch two files: a pull request is merged without `.gitattributes`, and a shared file conflicts. `docs/practices/list-files.md`.

The reservation rule lives in each glyph's `reserved` field — there is no
separate list. A glyph with no `reserved` value has never been reserved
against reuse; a glyph reused in more than one place in the UI (`file-cog`,
`chevron-down`) carries each context as a `[[icons.<name>.usage]]` entry
rather than a second top-level table overwriting the first.

## What did not survive the move

Notion's `Concepts` and `Components` columns were relations — arrays of page
links carrying no text beyond the link itself — so both are dropped rather than
resolved to a title.

## Checking this file against the app

A gate that walks the component sheet, the mockups, and `apps/desktop` for
every lucide glyph in use, and fails on a glyph with no row here or one used
somewhere its `reserved` field forbids, is being added separately (see the
Notion contract checks page). This registry is what it checks against — adding
a glyph means adding its file here first.
