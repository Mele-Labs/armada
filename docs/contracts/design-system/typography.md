# Typography

**Part of:** [Design System — UI & Voice](../design-system.md).

**IBM Plex Sans** for interface. **IBM Plex Mono** for anything
machine-derived — job IDs, file paths, branch names, commands, diffs,
durations, token counts.

That split is a rule, not a style preference: monospace signals *this is
a fact the system reported*, and it makes IDs and paths scannable in a
dense table.

Scale is tighter than web defaults. This is a dashboard.

**Legibility pass, 2026-08-20.** The whole ladder was raised ~15% after
the 11px and 13px steps proved hard to read at desk distance. Ratios and
roles are unchanged — every step moved together, so nothing about
hierarchy or the mono-one-step-smaller rule changes. The heights that
hold the larger text moved with it, and `tokens/spacing.css` is the
authority on each one.

```
--text-2xs   13px / 18px   table metadata, timestamps
--text-xs    14px / 20px   labels, badges, secondary
--text-sm    15px / 22px   BODY DEFAULT — most UI text
--text-base  16px / 24px   emphasis within body
--text-lg    18px / 28px   panel headings
--text-xl    23px / 32px   page titles
--text-2xl   28px / 36px   the rare hero number
```

Weights: 400 body, 500 labels, 600 headings. Never 700+.

**Emphasis on the dark ground is bought with size, tracking and hue, not
weight.** Dark polarity reads slower than light, and raising weight
improved glanceable reading in light mode only; it did not resolve
halation on a dark ground (Beier et al., *How bold can we be?*,
Readability Consortium, 2023). That is also why the 20 Aug legibility
lift raised the whole ladder and touched no weights. Emphasis within
body is one step up the ladder, or `0.012em` tracking at body sizes, or
`--fg-default` against `--fg-muted` — never 500 on a dense surface. 500
stays for labels, which is a role, not emphasis.

**One carve-out: `**bold**` in a model's text.** `Prose` draws it at
`--weight-heading` as well as `--fg-default`, never above. The owner chose
this on 1 Oct 2026 ("real bold weight"). The reason is that contrast alone
has nothing to add to a passage already at `--fg-default`, such as a
brief, so a bold run there drew as plain text. Every other surface keeps
the rule above.

**A second carve-out: a model's own headings.** `--text-lg` belongs to
panel headings, but `Prose` draws a `#` or `##` at `--text-lg` over a
`--border-default` rule, and a `###` at `--text-base`. The owner chose
this on 1 Oct 2026 ("B, more life"). A model's message has sections, and
at body size they did not read as sections. Both steps come from the
ladder. The heading is still a paragraph, so it takes no place in the
page's outline.

**Figures are lining and tabular everywhere**, set once at the root, so
numbers in a column align in the sans and compare by shape. Mono is
therefore never used to line figures up; it keeps its one job, marking
a fact the system reported. `--text-2xl` is the Overview summary's
count, the one hero number.
Mono runs one step smaller than adjacent sans at the same optical size —
14px mono next to 15px sans.
