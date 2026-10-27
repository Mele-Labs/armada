# Typography

**Part of:** [Design System — UI & Voice](../design-system.md).

**IBM Plex Sans** for interface. **IBM Plex Mono** for anything
machine-derived — job IDs, file paths, branch names, commands, diffs,
durations, token counts.

That split is a rule, not a style preference: monospace signals *this is
a fact the system reported*, and it makes IDs and paths scannable in a
dense table.

Scale is tighter than web defaults. This is a dashboard.

**The ladder is the Cockpit's, app-wide, from 26 Oct 2026.**
The owner chose it after looking at mission-control and cockpit interfaces: "they seem to be able to fit more on the screen.
Its like our font sizes, weights or spacing or something is restricting us".
This supersedes the 20 Aug 2026 legibility lift, which had raised every step about 15% because 11px and 13px proved hard to read at desk distance, and the third carve-out below, which gave the Cockpit alone a smaller ladder.
There is one ladder now, and the Cockpit has no carve-out.

```
--text-2xs   12px / 16px   table metadata, timestamps, a row's meta
--text-xs    13px / 18px   labels, badges, secondary, a row's title
--text-sm    14px / 20px   BODY DEFAULT — most UI text
--text-base  15px / 22px   emphasis within body
--text-lg    18px / 28px   panel headings
--text-xl    23px / 32px   page titles
--text-2xl   28px / 36px   the rare hero number
```

The look's roles agree with it: `--text-row` is `--text-xs`, `--size-meta` is `--text-2xs`, and a panel header and a title-bar place are `--text-xs` at 600 in tracked caps.
The steps above `--text-base` did not move.
The heights that hold the text moved with it, and `tokens/spacing.css` is the authority on each one.

**A model's text keeps the reading size.**
`Prose` declares its own `--text-2xs` to `--text-base` at 13, 14, 15 and 16px, and sets its body from them, so a message does not shrink with the app.
Inside the Cockpit it takes the Cockpit's steps, which it read before the ladder moved.
The owner decided this on 26 Oct 2026 with the ladder.

Weights: 400 body, 500 labels, 600 headings. Never 700+.

**Emphasis on the dark ground is bought with size, tracking and hue, not
weight.** Dark polarity reads slower than light, and raising weight
improved glanceable reading in light mode only; it did not resolve
halation on a dark ground (Beier et al., *How bold can we be?*,
Readability Consortium, 2023). That is also why the 20 Aug legibility
lift raised the whole ladder and touched no weights, and why the 26 Oct move back down touched none either. Emphasis within
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

`docs/concepts/cockpit.md`, *Density*, says what else the Cockpit tightens.

**Figures are lining and tabular everywhere**, set once at the root, so
numbers in a column align in the sans and compare by shape. Mono is
therefore never used to line figures up; it keeps its one job, marking
a fact the system reported. `--text-2xl` is the Overview summary's
count, the one hero number.
Mono runs one step smaller than adjacent sans at the same optical size —
13px mono next to 14px sans.
