# Capabilities

One file per capability, paired to one GitHub issue by its frontmatter. The
issue tracks — it lists its steps under a `**Steps**` heading, so progress is
computed rather than reported. A step serves as many capabilities as it genuinely
does, so the list lives in the body rather than as a sub-issue link, and a `#N`
anywhere else in the body is a cross-reference and not a step. The file holds the
prose, the rationale and the links to the concepts it depends on, because an
issue body buries all of that the moment the issue closes.

The frontmatter is the binding: `capability:` must match the filename, `issue:`
must name the issue that tracks it, and `milestone:` is the milestone it sits
under. `cargo xtask verify-foundations` checks the shape and this index offline;
`cargo xtask verify-roadmap` checks that the issue exists.

A capability acquires a file when it acquires prose worth keeping. One that is
only its steps does not need one.

- [`autopilot.md`](autopilot.md) — a repository works its own backlog from one
  plain-language instruction, every pick an ordinary Job, within caps a person
  sets, until they pause or stop it.
- [`merge-line.md`](merge-line.md) — the retired merge line, and what replaces
  each part of it. The full design is archived.
