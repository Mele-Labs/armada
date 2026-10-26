// The kit a Job's document may link: `<link rel="stylesheet" href="armada-kit.css">`. Armada's type and colour tokens as plain styling
// for headings, paragraphs, lists, tables and code, so a page with no CSS of its own still reads right. Bridge's reset strips every
// list's markers and indent, so the kit gives them back. The page's own style comes after it and
// wins. The only stylesheet a page can link; any other link is dropped. `<armada-sketch>` is the kit's one element, drawn by Bridge.

export const KIT_HREF = "armada-kit.css";

export const KIT_CSS = `
:root { color: var(--fg-default); font-family: var(--font-sans); font-size: var(--text-base); line-height: var(--leading-base); }
h1 { margin: 0 0 var(--space-4); font-size: var(--text-2xl); line-height: var(--leading-2xl); font-weight: var(--weight-heading); }
h2 { margin: var(--space-8) 0 var(--space-3); font-size: var(--text-lg); line-height: var(--leading-lg); font-weight: var(--weight-heading); }
h3 { margin: var(--space-6) 0 var(--space-2); font-size: var(--text-base); line-height: var(--leading-base); font-weight: var(--weight-heading); }
p, ul, ol { margin: 0 0 var(--space-3); max-inline-size: 68ch; }
ul, ol { padding-inline-start: var(--space-6); }
ul { list-style: disc; }
ul ul { list-style: circle; }
ol { list-style: decimal; }
li { margin: 0 0 var(--space-1); }
li > ul, li > ol { margin: var(--space-1) 0 0; }
li::marker { color: var(--fg-muted); }
a { color: var(--accent); }
strong { font-weight: var(--weight-heading); }
code, kbd, samp, pre { font-family: var(--font-mono); font-size: var(--text-xs); }
code { padding: 0 var(--space-1); border-radius: var(--radius-sm); background: var(--bg-sunken); }
pre { padding: var(--space-3); border-radius: var(--radius-md); background: var(--bg-sunken); overflow-x: auto; }
blockquote { margin: 0 0 var(--space-3); padding-inline-start: var(--space-4); border-inline-start: var(--edge-active) solid var(--border-default); color: var(--fg-muted); }
table { inline-size: 100%; margin: 0 0 var(--space-4); border-collapse: collapse; font-size: var(--text-xs); line-height: var(--leading-xs); }
caption { padding-block-end: var(--space-2); color: var(--fg-muted); text-align: start; }
th { padding: var(--space-2) var(--space-3); border-block-end: var(--border-width) solid var(--border-default); color: var(--fg-muted); font-weight: var(--weight-medium); text-align: start; }
td { padding: var(--space-2) var(--space-3); border-block-end: var(--border-width) solid var(--border-subtle); }
figure { margin: var(--space-6) 0; }
figcaption { margin-block-start: var(--space-2); color: var(--fg-muted); font-size: var(--text-xs); line-height: var(--leading-xs); }
svg { display: block; max-inline-size: 100%; block-size: auto; color: var(--fg-default); fill: currentColor; font-family: var(--font-sans); font-size: var(--text-xs); }
hr { border: 0; border-block-start: var(--border-width) solid var(--border-subtle); margin: var(--space-6) 0; }
`;
