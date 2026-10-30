# Pocket

The phone app: a Home Screen web app named Armada. A plain responsive page, so
narrow the browser window to see it as a phone. One app talks to every Mac it
is paired with, each directly: the Phone Gateway that Fleet serves on that Mac
(`crates/pocket`).

`pnpm -C apps/pocket fixture` serves it on a fixture Gateway with two Macs,
`studio` on Vite's port (5173) and `laptop` on the next one, so the app opens
on a list at `http://localhost:5173/`. `POCKET_FIXTURE_UNPAIRED=laptop` makes
`laptop` answer 401, as a Mac that forgot the phone does. The fixture is
`src/fixture/`.

`pnpm -C apps/pocket dev` serves it with no Gateway behind it, and the app
goes to `/pair`. Routes: `/` every Mac's Jobs (Drafts, Needs you, Running, under
the plan's 5-hour and weekly limits), `/pair` (`?code=…` from a pairing link,
`?origin=…` to pair a Mac again), `/dispatch`, a Job at
`/jobs/<Mac origin, encoded>/<id>`, and what it showed under it at
`/file?path=…` and `/sketch?id=…`. A draft Job, one held on its start card,
opens at `/drafts/<Mac origin, encoded>/<id>`: its brief to edit, how the work
lands where its workflow asks, Send and Discard. A Job that has begun opens as
a Job from there, and a held one opens as a draft from `/jobs/…`.

A Job's thread ends in what waits on the owner: a question, a Run card for a
command to run on the Mac, a Draft card to approve or send back. The header's
⋯ menu holds Stop, Pause or Resume, Fork, model, effort and End job.

Every call is signed with that Mac's key in `src/client.ts`. Each key is made
when its Mac is paired, non-extractable, and kept in IndexedDB
(`src/machines.ts`). `public/sw.js` is the service worker; a tapped push opens
the Job it is about.

Checks: `apps/pocket:typecheck` and `apps/pocket:pocket_test`.
