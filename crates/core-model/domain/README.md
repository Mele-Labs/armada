# crates/core-model/domain

**Authored here.** These files are the authority on Armada's domain sets —
what a Check outcome is, what a WorkflowDef declares, which acts Bridge carries
and what a Studio is made of. The design workspace's databases are the record of the decision and the reasoning that
produced them, not the authority on the current set: a page there that
disagrees with a file here is stale, not right.

They live beside `core-model` rather than in `docs/` for the reason
`crates/config/settings.toml`, `packages/tokens` and `packages/icons` do. A set
that code reads is a data file beside the crate that reads it, with a check over
it. While these lived in a database nothing could compare them to an enum. As
files, a rule can — and the rule is the point.

| Path | What it is |
|---|---|
| `check-outcomes.toml` | What one declared mechanical Check did. The values a Check run's `outcome` takes |
| `enum-verbs.toml` | The word, the glyph and the colour each variant renders as, for every enum its own header lists |
| `workflowdef-fields.toml` | What a WorkflowDef declares: a step's gates, its evidence scope, the def-level defaults |
| `workflows.toml` | The workflows Armada runs, and the ones it has decided not to design yet |
| `workflow-samples/*.json` | The WorkflowDefs themselves, verbatim. One per workflow whose row says it has one |
| `actions.toml` | The verb, the glyph and the binding every act in Bridge carries |
| `studio-kinds.toml` | What a [Studio](../../../docs/concepts/studio.md)'s graph is made of: its node kinds and their states, its edge kinds, the forge states a node carries and the rungs of promotion between them |

**`actions.toml` is the one file here that is not a domain enum**, and it lives
here for the reason `enum-verbs.toml` does: it is a set code reads, and a set
code reads is a data file beside the crate with a check over it. It holds acts
where the rest hold states, so no key is spelled in both — `killed` is a badge
in `enum-verbs.toml`, `kill` is an action. Its glyphs are keys in
`packages/icons/icons/` rather than a second roster, and its bindings are
the two blocks under "Keyboard and command palette" in
`docs/contracts/design-system.md`, which the gate reads and compares.

**`studio-kinds.toml` is held to three readers rather than one enum**, because
a Studio's sets are spelled in `core-model`, in the store's `CHECK` constraints
and in the TypeScript mirror, and nothing compared them. Its promotions are not
an enum at all: they are `from`/`to` pairs over the node kinds, so a rule can
read a rung's ends against the kinds.

## What a rule can check, and why the layout is shaped for it

A table is keyed on the wire value, with no slug and no `name` field beside it,
the way `settings.toml` does: a setting has a human name and a key, an enum
variant has only the one spelling, and a second spelling is the drift these
files exist to catch.

The WorkflowDef samples are JSON rather than TOML because that is the shape a
repo authors and Fleet interprets, and embedding them in a TOML string would
make them unparseable as what they are. A rule can read one and check every key
against `workflowdef-fields.toml`, every `advance_gate` against that row's value
table, and every `mechanical_check.type` against the sanctioned set.

## What did not survive the move

Relations in the source are arrays of page links, and this repository is public,
so no address into that workspace may appear here. Every relation is carried as
the **name** of the thing it points at, which is also what makes it checkable —
a link resolves to nothing a rule can compare, a name resolves to a table key.
One row carried a raw workspace address in prose where it meant the Workflow
concept; it reads as the concept's name now.

`In code` is carried verbatim as the source recorded it, and it is provenance
rather than fact: no crate in this workspace implements any of it yet, including
the rows that say otherwise.

## Adding to these files

A value added here is a value some enum must gain, so add it in the change that
adds the variant, not before and not after. `actions.toml` has no enum to gain
a variant: a row there needs its line in the contract's key map in the same change, and a glyph
or a stated reason for having none. Nothing under this directory is Rust and
nothing is under `src/`.

## Open questions

- **[workflowdef-schema-gaps]** Five keys appear in the workflow samples with
  no row in the field catalogue: `workflow_id`, `version`, `order`, `required`,
  `manifest_rule_overrides`.
  What decides it: the samples are the working shape and the catalogue is the
  schema, so either the catalogue is incomplete or the samples carry keys
  nothing reads. Only one can be true, and the answer decides what a parser
  accepts.
