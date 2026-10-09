# Pocket

The phone app: a Home Screen web app named Armada. A plain responsive page, so
narrow the browser window to see it as a phone. Jobs come from the Phone Gateway
(`crates/pocket`) through `src/data.ts`; sessions and dispatch still read
`src/fixtures.ts` (#2004, #2006).

`pnpm -C apps/pocket dev` serves it with Vite on port 5173 (`--port` to change it).
With no Gateway behind it the app sends you to `/pair`. Routes: `/` the tabs,
`/pair?code=…`, `/jobs/:id`, `/sessions/:id`, `/dispatch`.

Every Gateway call is signed in `src/client.ts`. The key is made on `/pair`,
non-extractable, and lives in IndexedDB. `public/sw.js` is the service worker.

Checks: `apps/pocket:typecheck` and `apps/pocket:pocket_test`.
