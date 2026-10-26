---
name: writing-a-document
description: Write the result of a Job as an HTML page the person reads in Bridge, with a conclusion first, sections, tables and live sketches, and revise it from numbered notes. Load when the Job's result is something a person reads, such as an investigation, findings or a report.
---

# Writing a document

**A document is the result a person reads, and you design it.** It is one HTML
page. Bridge opens it in a reading layer over the Job, shows its sections in a
contents rail, and lets the person note words, a part of a sketch or a table
row. Notes are the only feedback.

## Where it goes

| Do | Because |
|---|---|
| Write it to `.armada/documents/<name>.html` in your worktree | Fleet hears every Write or Edit to that folder and, when your turn ends, reads the folder again, so a page changed another way is kept as a revision too |
| Name it for the subject: `slot-lease-expiry.html` | The name is the document. A new name is a new document |
| Never `git add` it, never put it in a diff | `.armada/*` is gitignored. If the person asks to add it to the repo, `git add -f` the file and commit it |

## The skeleton

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Why do slot leases expire early</title>
<link rel="stylesheet" href="armada-kit.css">
</head>
<body>
<h1>Why do slot leases expire early</h1>
<p>The conclusion, in two or three sentences: what is true, how sure you are, what to do.</p>

<h2>What was measured</h2>
<p>…</p>

<h2>What was not measured</h2>
<p>…</p>
</body>
</html>
```

| Part | Rule |
|---|---|
| `<title>` | The document's name. Keep it the same in every revision, because a note names the page by it |
| One `<h1>` | The same words as the title |
| First `<p>` after the `<h1>` | The conclusion. The Landing shows it with the title |
| `<h2>` | One per section. The contents rail lists them, and a note names its section by the text. Keep an `<h2>` unchanged across revisions |
| `<h3>` | A part of a section. Not in the rail |

Say what was not measured in its own section. The person reads that section to
know how far to trust the rest.

## The kit and your own style

`<link rel="stylesheet" href="armada-kit.css">` gives headings, paragraphs,
lists, tables, code, quotes and figures Armada's type and colour. A page with
no style of its own reads right.

Add a `<style>` only for what the kit does not say: a narrower column, a
larger first paragraph, tabular figures, a bar. Your rules come after the kit's
and win. Use the kit's variables, not hex values.

| Variable | For |
|---|---|
| `--fg-default`, `--fg-muted`, `--fg-subtle` | Text, secondary text, metadata |
| `--bg-sunken`, `--bg-raised`, `--border-subtle`, `--border-default` | Wells, panels, rules |
| `--accent` | The one emphasis colour |
| `--status-completed-success`, `--status-completed-failed`, `--status-awaiting-review`, `--status-running` | Good, bad, caution, in flight |
| `--space-1` to `--space-12`, `--text-xs` to `--text-2xl`, `--font-mono` | Spacing, type size, figures |

## Sketches

`<armada-sketch>` draws a live sketch. The person plays its steps, and notes a
part of it.

```html
<armada-sketch title="Lease clock against a Check run"><script type="application/json">
{"nodes":[
  {"id":"handed","kind":"box","x":0,"y":0,"title":"Bay handed over","body":"The Job gets the bay","icon":"hard-drive","step":1},
  {"id":"clock","kind":"box","x":330,"y":0,"title":"Lease clock starts","body":"30 minutes","icon":"clock","step":2},
  {"id":"expires","kind":"box","x":660,"y":0,"title":"Lease expires","icon":"lock","step":3,"color":"red"}
],"edges":[
  {"id":"handed-clock","from":"handed","to":"clock","step":1},
  {"id":"clock-expires","from":"clock","to":"expires","label":"30 minutes","step":3}
]}
</script></armada-sketch>
```

The `title` is the sketch's name in a note, so give each sketch its own. The
scene is JSON, and it must not contain the text `</script>`. Put a sketch where
the words need a picture of a flow, a structure or a sequence, and one idea per
sketch.

**A scene that does not validate draws an empty frame, and Fleet tells you in a
notice.** The write is not refused, so fix the scene and write again. The rules:

| Part | Rule |
|---|---|
| Ids | Unique among nodes, and unique among edges |
| Edge | `from` and `to` are ids of nodes in the scene |
| Node `kind` | `box`, `group`, `lane`, `label`, `code`. A `group` and a `lane` need `w` and `h` |
| Node fields | `id`, `kind`, `x`, `y`, and optionally `w`, `h`, `title`, `body`, `icon`, `lang`, `step`, `shape`, `color` |
| Edge fields | `id`, `from`, `to`, and optionally `label`, `flow`, `step`, `color` |
| `icon` | One of: activity, bot, box, clock, cpu, eye, file, git-branch, git-merge, globe, hammer, hard-drive, layers, lock, package, rocket, scale, scroll-text, server, settings, shield-check, terminal, webhook, wrench, zap |
| `shape` | `box`, `pill`, `circle`, `diamond`, `cylinder`, `hexagon` |
| `color` | `blue`, `teal`, `green`, `amber`, `orange`, `red`, `grey` |
| `step` | A finite number. Nodes and edges of one number play together, in order |
| Layout | Boxes are laid out from `x` and `y`. Space them about 330 apart across and 170 down, and Bridge moves them only as far as overlap needs |

An unknown `kind`, `icon`, `shape` or `color`, a duplicate id or an edge to a
missing node fails the whole scene. Use `color` for meaning: `red` for what
fails, `amber` for what is unknown, `green` for what works. Leave the rest
uncoloured.

`kind` also accepts `wire` and `screen`, wireframes of a product's screen. The
`show_sketch` tool describes them.

## Figures and charts

There is no chart element. A chart is an inline `<svg>` in a `<figure>`, drawn
from shapes and text. Bridge drops `img`, so a chart is never a picture file.

- Give the `<svg>` a `viewBox` and no fixed width, so it scales to the column.
  The kit sets its text colour, type and size.
- Colour a mark with a class and a rule in your `<style>` using the kit's
  variables, as below. A `fill` or `stroke` attribute may also be a variable,
  such as `fill="var(--accent)"`. A hex value does not follow the theme.
- Say what the chart shows in `role="img"` and `aria-label` on the `<svg>`, and
  put the figures behind it in a `<table>` as well, so a person can note a row.
- Set `fill="none"` on a line or outline, as any SVG needs.

```html
<figure>
  <svg viewBox="0 0 240 100" role="img" aria-label="Failures by week">
    <rect class="bar" x="10" y="40" width="40" height="50"/>
    <rect class="bar" x="70" y="20" width="40" height="70"/>
    <rect class="bar bad" x="130" y="60" width="40" height="30"/>
    <text x="30" y="98" text-anchor="middle">W1</text>
    <text x="90" y="98" text-anchor="middle">W2</text>
    <text x="150" y="98" text-anchor="middle">W3</text>
  </svg>
  <figcaption>Failures by week</figcaption>
</figure>
<style>
  .bar { fill: var(--accent); }
  .bad { fill: var(--status-completed-failed); }
</style>
```

- Put figures in a `<table>` with `<thead>` and `<tbody>`. The person notes a
  row of the body, so one row holds one fact.
- A caption goes in `<caption>`, not above the table.
- For a bar, put a `span` with a class in a cell, and set its width and
  colour in your `<style>`:

```html
<style>
  .bar { display: block; block-size: var(--space-2); background: var(--accent); }
  .w62 { inline-size: 62%; }
</style>
<td><span class="bar w62"></span></td>
```

- Set figures in `--font-mono` with `font-variant-numeric: tabular-nums` so
  columns line up.

## What Bridge drops

The page is read element by element from an allow list. Dropped, without an
error:

| Dropped | Instead |
|---|---|
| `<script>`, event handlers such as `onclick` | Nothing runs. Use a table or a sketch |
| `style="…"` attributes | A class and a rule in `<style>` |
| `<img>`, `<canvas>`, `<video>`, `<audio>` | An inline `<svg>`, a table, a bar, a sketch |
| In an `<svg>`: `<script>`, `<foreignObject>`, `<use>`, `<image>`, an `<a>` with an address, gradients and filters, `style="…"` | Shapes, text and `<g>`, coloured by class |
| `<iframe>`, `<object>`, `<embed>`, `<form>`, `<input>`, `<button>` | Words |
| Any `<link>` but `armada-kit.css` | `<style>` |
| A CSS rule with `url(…)`, `@import`, `@font-face`, `@keyframes` | System fonts, no motion |
| A link that is not `#id` or `http(s)://` | A full address |

Nothing is fetched from the network. Name a source as a link with its words.

## How it should read

- Facts, in the order the person needs them. The conclusion comes first.
- No narration: not "let's look at" or "it is important to note". Each
  sentence adds something the person did not have.
- No emoji, no gradients, no glows, no sheen, no grain, no decoration. The
  kit's colour is enough.
- Apply the humanizer rules to every sentence: no "not X but Y", no closing
  line that repeats the paragraph, no triad for rhythm, no dashes as
  connectors.
- Name the number, the file and the date. Say "7 of 41" and not "a few".
- Say how sure you are once, where it applies, and say what you did not check.

## Revising

**The person's notes arrive as one numbered message.** Each note has its
subject on the number's line, which ends in the quoted words, the sketch and
its parts, or the table row, and the person's words beneath:

```
1. Document "Why do slot leases expire early", revision 1, section "How the lease clock runs": words "leave the limit as it is"
   Why not raise the limit instead?
```

- Answer every note by changing the page where the note points. A note on a
  sentence is answered at that sentence, a note on a part of a sketch in or
  beside that sketch, a note on a row in that row or just under the table.
- If the answer is "no, because", write the because into the page. The reply
  in the thread is short; the page is the answer.
- Write the same file again. Do not make a new file, and do not write a
  `-v2` copy. Fleet keeps each different write as the next revision, and
  Changes marks what moved on.
- Keep the `<title>` and the `<h2>` text. Each note finds its section by them,
  and the change that answers it is the nearest one in that section.
- Change what the note asked, and leave the rest of the page alone. A rewrite
  of untouched paragraphs hides the answers among changes.
- Say in the thread, in one line, that the next revision is written.
