# Pocket

The phone app: a Home Screen web app named Armada. A plain responsive page, so
narrow the browser window to see it as a phone. Fixture data only, behind
`src/data.ts`, which the Phone Gateway client replaces.

`pnpm -C apps/pocket dev` serves it with Vite on port 5173 (`--port` to change it).
It needs nothing running: no Fleet, no network. Routes: `/` the tabs, `/pair`,
`/jobs/:id`, `/sessions/:id`, `/dispatch`. A bad id draws an empty page.

`pnpm -C apps/pocket typecheck` is clean when it prints nothing.
