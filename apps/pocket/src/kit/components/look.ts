// Bridge's look is on the root of every window that loads `index.css`: `depth.css`, `pages.css` and
// `@armada/tokens/takes.css` all start `:root[data-look="new"]`. Imported for this effect by each entry that
// loads that stylesheet (the renderer, the mock and the screens' browser tests), so none draws without it.
document.documentElement.dataset.look = "new";
